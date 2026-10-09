"""Bound intermediate allocations while preserving pinned mono USES equations.

The input/output STFT spans the whole utterance. Only feature allocation is tiled:
one-frame convolution halos, pointwise channel normalization, and the original
non-overlapping USES segment/memory schedule. There is no audio stitching.
"""
import torch


def _validate(separator,spectrum,mode):
    from radcast_uses_vendor.tcn import ChannelwiseLayerNorm
    if mode!='dereverb':raise ValueError('Bounded USES requires mode dereverb')
    if (not isinstance(spectrum,torch.Tensor) or spectrum.ndim!=3 or spectrum.shape[0]!=1
            or not spectrum.shape[1] or not spectrum.shape[2]
            or spectrum.dtype!=torch.complex64 or spectrum.device.type!='cpu'
            or not torch.isfinite(spectrum).all()):
        raise ValueError('Bounded USES requires finite mono native complex64 CPU spectrum [1,T,F]')
    if separator.num_spk!=1:raise ValueError('Bounded USES supports one speaker')
    uses=separator.uses
    if separator.training or uses.training:raise ValueError('Bounded USES requires eval mode')
    if (uses.memory_types!=2 or len(uses.memory_tokens)!=2 or uses.memory_size<=0
            or uses.segment_size<=0 or separator.ref_channel!=0):
        raise ValueError('Unsupported mono two-memory USES configuration')
    if not isinstance(uses.layer_norm,ChannelwiseLayerNorm) or uses.layer_norm.shape!='BDT':
        raise ValueError('Bounded feature normalization must reduce only the channel axis')
    if any(block.ch_mode!='tac' for block in uses.atf_blocks):
        raise ValueError('Bounded USES supports the pinned TAC configuration')
    for block in uses.atf_blocks:
        for transformer in (block.freq_nn,block.temporal_nn):
            if not all(isinstance(norm,ChannelwiseLayerNorm) for norm in (transformer.norm_attn,transformer.norm_ff)):
                raise ValueError('Unsupported Transformer channel normalization')
    conv=separator.post_encoder;deconv=separator.pre_decoder;point=uses.bottleneck_conv1x1
    if (not isinstance(conv,torch.nn.Conv2d) or conv.kernel_size!=(3,3)
            or conv.stride!=(1,1) or conv.padding!=(1,1) or conv.dilation!=(1,1)
            or conv.groups!=1 or conv.in_channels!=2
            or not isinstance(deconv,torch.nn.ConvTranspose2d) or deconv.kernel_size!=(3,3)
            or deconv.stride!=(1,1) or deconv.padding!=(1,1) or deconv.dilation!=(1,1)
            or deconv.output_padding!=(0,0) or deconv.groups!=1 or deconv.out_channels!=2
            or not isinstance(point,torch.nn.Conv1d) or point.kernel_size!=(1,)
            or point.padding!=(0,) or point.stride!=(1,) or point.dilation!=(1,) or point.groups!=1):
        raise ValueError('Unsupported convolution configuration')
    if (not isinstance(uses.output,torch.nn.Sequential) or len(uses.output)!=2
            or not isinstance(uses.output[0],torch.nn.PReLU)
            or not isinstance(uses.output[1],torch.nn.Conv2d)
            or uses.output[1].kernel_size!=(1,1) or uses.output[1].stride!=(1,1)
            or uses.output[1].padding!=(0,0) or uses.output[1].groups!=1):
        raise ValueError('Unsupported pointwise USES output mapping')
    if any(p.dtype!=torch.float32 or p.device.type!='cpu' for p in separator.parameters()):
        raise ValueError('Bounded USES requires float32 CPU parameters')


def _encoded_segment(separator,spectrum,start,end,trace):
    """True STFT neighbors supply the 3x3 encoder's one-frame temporal halo."""
    uses=separator.uses;frames=spectrum.shape[1]
    halo_start=max(0,start-1);halo_end=min(frames,end+1)
    selected=spectrum[:,halo_start:halo_end]
    feature=torch.stack((selected.real,selected.imag),dim=1).moveaxis(-1,-2)
    embedded=separator.post_encoder(feature)
    offset=start-halo_start
    embedded=embedded[...,offset:offset+end-start].contiguous()
    trace['maximum_post_encoder_frames']=max(trace['maximum_post_encoder_frames'],halo_end-halo_start)
    trace['maximum_post_encoder_elements']=max(trace['maximum_post_encoder_elements'],
        spectrum.shape[0]*separator.enc_channels*spectrum.shape[2]*(halo_end-halo_start))
    # Upstream cLN reduces dim=1 only; every F/T point is independent.
    normalized=uses.layer_norm(embedded.reshape(1,separator.enc_channels,-1))
    output=uses.bottleneck_conv1x1(normalized).reshape(1,1,uses.bottleneck_size,spectrum.shape[2],end-start)
    # Upstream pads bottleneck features, not raw STFT or encoder embeddings.
    if end-start<uses.segment_size:
        output=torch.nn.functional.pad(output,(0,uses.segment_size-(end-start)))
    return output


def _decoded_chunk(separator,pending,left,right):
    parts=[]
    if left is not None:parts.append(left)
    parts.append(pending)
    if right is not None:parts.append(right)
    neighborhood=torch.cat(parts,dim=-1) if len(parts)>1 else pending
    real_imag=separator.pre_decoder(neighborhood)
    offset=0 if left is None else 1
    real_imag=real_imag[...,offset:offset+pending.shape[-1]]
    return torch.complex(real_imag[:,0],real_imag[:,1]).transpose(1,2)


@torch.inference_mode()
def bounded_separator(separator,spectrum,mode='dereverb'):
    """Return a whole enhanced complex STFT and an honest execution trace.

    Uses the original submodules and learned weights. The outer allocation
    schedule is mirrored here, so upstream separator/USES.forward hooks do not
    execute. Memory group one is accessed once and its state carries through all
    original ATF blocks in exactly the original segment order.
    """
    _validate(separator,spectrum,mode)
    uses=separator.uses;frames=spectrum.shape[1];frequency=spectrum.shape[2]
    trace=dict(execution_backend='bounded_features',mode=mode,memory_indices=[],
        memory_initialization_count=0,memory_reuse_count=0,atf_block_calls=0,
        segment_size=uses.segment_size,memory_size=uses.memory_size,
        segment_count=(frames+uses.segment_size-1)//uses.segment_size,
        upstream_separator_forward_called=False,upstream_uses_forward_called=False,
        post_encoder_time_halo=1,pre_decoder_time_halo=1,final_padding_after_bottleneck=True,
        maximum_post_encoder_frames=0,maximum_post_encoder_elements=0,
        external_audio_chunks=1,audio_crossfade=False,whole_utterance_stft=True,
        memory_state_carried_across_segments=True,input_spectrum_shape=list(spectrum.shape))
    mem=None;pending=None;left=None;chunks=[]
    for start in range(0,frames,uses.segment_size):
        end=min(frames,start+uses.segment_size)
        out=_encoded_segment(separator,spectrum,start,end,trace)
        if mem is None:
            # Actual learned ParameterList access, not an upstream-forward hook.
            memory_index=1
            mem=uses.memory_tokens[memory_index].repeat(1,1,1,frequency,1)
            trace['memory_indices'].append(memory_index)
            trace['memory_initialization_count']+=1
        else:trace['memory_reuse_count']+=1
        out=torch.cat((mem,out),dim=-1)
        for block in uses.atf_blocks:
            out=block(out,ref_channel=separator.ref_channel)
            trace['atf_block_calls']+=1
        mem,out=out[...,:uses.memory_size],out[...,uses.memory_size:]
        # Truncate last-segment padding before the pointwise output map.
        out=out[...,:end-start]
        with torch.amp.autocast('cuda',enabled=False):
            mapped=uses.output(out.mean(1))
        if pending is not None:
            # Hold one completed feature segment until its true right neighbor
            # exists; the transposed 3x3 decoder sees identical temporal context.
            chunks.append(_decoded_chunk(separator,pending,left,mapped[...,:1]))
            left=pending[...,-1:].clone()
        pending=mapped
    chunks.append(_decoded_chunk(separator,pending,left,None))
    result=torch.cat(chunks,dim=1)
    if result.shape!=spectrum.shape or not torch.isfinite(result).all():
        raise RuntimeError('Bounded USES produced invalid enhanced spectrum')
    trace['output_spectrum_shape']=list(result.shape)
    return result,trace
