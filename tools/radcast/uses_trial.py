"""Native-rate USES experiment primitives. No application preset or fallback."""
import hashlib
import importlib
import json
from pathlib import Path
import resource
import sys
import time
import numpy as np

RATE=48000
FFT=1536
HOP=768
CONFIG_SHA256='3cd7b57bdb541672fea82d9ab16a04ce63299ac4b108907a5c406b2dd1dc8793'
CHECKPOINT_SHA256='6b2a0c78b2eea566fcfd39b0d77ffc0102b0baf6af60a4a1f4dee93995081121'
CODE_REVISION='bfc13cecfd0a07ed8e21d733b0ce130a1c69211a'
MODEL_REVISION='927a9ecea245120a6f2d88c2552864b937ec5ab9'
SEPARATOR_CONFIG=dict(num_spk=1,enc_channels=256,bottleneck_size=64,num_blocks=6,
    num_spatial_blocks=3,segment_size=64,memory_size=20,memory_types=2,rnn_type='lstm',
    bidirectional=True,hidden_size=128,att_heads=4,dropout=0.0,norm_type='cLN',
    activation='relu',ch_mode='tac',ch_att_dim=256,eps=1e-5,ref_channel=0)


def _sha(path):
    digest=hashlib.sha256()
    with Path(path).open('rb') as source:
        for data in iter(lambda:source.read(1024*1024),b''):digest.update(data)
    return digest.hexdigest()


def _configuration(artifacts):
    import yaml
    for name,expected in (('config.yaml',CONFIG_SHA256),('20epoch.pth',CHECKPOINT_SHA256)):
        if _sha(artifacts/name)!=expected:raise ValueError(f'{name} hash mismatch')
    config=yaml.safe_load((artifacts/'config.yaml').read_text())
    expected=dict(separator='uses',encoder='stft',decoder='stft',num_spk=1,sample_rate=8000)
    if any(config.get(k)!=v for k,v in expected.items()):raise ValueError('Unsupported USES configuration')
    if config.get('separator_conf')!=SEPARATOR_CONFIG:raise ValueError('Unsupported separator configuration')
    if config['model_conf'].get('normalize_variance') is not True:
        raise ValueError('Pinned upstream variance normalization is required')
    if config.get('encoder_conf')!=dict(n_fft=256,hop_length=128,use_builtin_complex=False,default_fs=8000):
        raise ValueError('Unsupported upstream encoder configuration')
    if config.get('decoder_conf')!=dict(n_fft=256,hop_length=128,default_fs=8000):
        raise ValueError('Unsupported upstream decoder configuration')
    return config


def _vendor(vendor_dir):
    audit=json.loads((vendor_dir/'vendor-audit.json').read_text())
    if audit.get('code_revision')!=CODE_REVISION or audit.get('model_revision')!=MODEL_REVISION:
        raise ValueError('Unsupported vendor revisions')
    if audit.get('is_uses2_checkpoint') is not False:raise ValueError('Legacy USES vendor provenance required')
    for relative,expected in audit['files'].items():
        path=(vendor_dir/relative).resolve()
        if not path.is_relative_to(vendor_dir.resolve()) or _sha(path)!=expected:
            raise ValueError(f'vendor {relative} hash mismatch')
    package='radcast_uses_vendor'
    loaded=sys.modules.get(package)
    expected=(vendor_dir/package/'__init__.py').resolve()
    if loaded is not None and Path(loaded.__file__).resolve()!=expected:
        raise ValueError('Different vendor package already imported in this process')
    # Unique package namespace in this experiment only. No ESPnet/app module aliases.
    directory=str(vendor_dir.resolve())
    if directory not in sys.path:sys.path.insert(0,directory)
    module=importlib.import_module(package)
    if Path(module.__file__).resolve()!=expected:raise ValueError('Unexpected USES vendor import origin')
    return module,audit


def _process_peak_rss_bytes():
    value=resource.getrusage(resource.RUSAGE_SELF).ru_maxrss
    return int(value if sys.platform=='darwin' else value*1024)


def _bind_equivalence(artifacts,info):
    """Bind root's trained comparison report; absence never implies a pass."""
    report=artifacts.parent/'bounded-equivalence.json'
    info['equivalence_report_sha256']=None
    if not report.exists():return
    record=json.loads(report.read_text())
    if record.get('passed') is not True:raise ValueError('bounded equivalence passed flag is not true')
    for key in ('checkpoint_sha256','config_sha256','vendor_audit_sha256','adapter_sha256',
                'uses_bounded_sha256','execution_config'):
        if record.get(key)!=info[key]:raise ValueError(f'bounded equivalence {key} mismatch')
    info['equivalence_report_sha256']=_sha(report)
    info['trained_backend_equivalence_verified']=True


class UsesModel:
    """Strictly pinned legacy USES, one whole utterance, explicit dereverb mode.

    No resampling, spectral truncation, downstream treatment, output normalization,
    chunkwise reconstruction, or application fallback is performed here.
    """
    def __init__(self,artifacts,vendor_dir=None,backend='upstream'):
        if backend not in ('upstream','bounded'):raise ValueError('Unsupported USES execution backend')
        import torch
        self.backend=backend
        self.artifacts=Path(artifacts).resolve()
        config=_configuration(self.artifacts)
        vendor_dir=Path(vendor_dir) if vendor_dir is not None else self.artifacts.parent/'vendor'
        vendor,audit=_vendor(vendor_dir)
        torch.set_num_threads(4)
        self.model=torch.nn.Module()
        self.model.add_module('separator',vendor.USESSeparator(input_dim=FFT//2+1,**config['separator_conf']))
        self.model.to(device='cpu',dtype=torch.float32).eval()
        state=torch.load(self.artifacts/'20epoch.pth',map_location='cpu',weights_only=True)
        if not isinstance(state,dict) or not all(isinstance(v,torch.Tensor) for v in state.values()):
            raise ValueError('Checkpoint must contain a tensor state dictionary')
        info=strict_load(self.model,state)
        if info['tensor_count']!=261 or info['element_count']!=3052492:
            raise ValueError('Unexpected pinned model tensor/element count')
        self.separator=self.model.separator
        self.info=dict(info,model_family='legacy USES (ASRU 2023)',is_uses2_checkpoint=False,
            code_revision=CODE_REVISION,model_revision=MODEL_REVISION,
            config_sha256=CONFIG_SHA256,checkpoint_sha256=CHECKPOINT_SHA256,
            vendor_audit_sha256=_sha(vendor_dir/'vendor-audit.json'),vendor_symbol_count=len(audit['symbols']),
            device='cpu',dtype='float32',torch_version=torch.__version__,threads=torch.get_num_threads(),
            weights_only=True,sample_rate=RATE,upstream_default_fs=8000,fft=FFT,hop=HOP,
            frequency_bins=FFT//2+1,complex_representation='native torch complex',
            normalize_variance='torch.std(correction=1), restored exactly once',
            output_peak_normalization=False,resampling=False,cascade=False,
            mode='dereverb',memory_types=2,memory_reset='one local initialization per whole utterance',
            execution_backend='upstream' if backend=='upstream' else 'bounded_features',
            upstream_class_asts_unchanged=True,trained_backend_equivalence_verified=False,
            adapter_sha256=_sha(Path(__file__)),
            uses_bounded_sha256=_sha(Path(__file__).with_name('uses_bounded.py')),
            backend_sha256=_sha(Path(__file__).with_name('uses_bounded.py')) if backend=='bounded' else None,
            backend_equivalence_requirement='Trained whole-utterance comparison required before bounded full-file rendering')
        self.info['execution_config']=dict(backend=self.info['execution_backend'],device='cpu',dtype='float32',
            torch_version=torch.__version__,threads=4,sample_rate=RATE,fft=FFT,hop=HOP,frequency_bins=FFT//2+1,
            channels=1,num_spk=self.separator.num_spk,ref_channel=self.separator.ref_channel,
            segment_size=self.separator.uses.segment_size,memory_size=self.separator.uses.memory_size,
            memory_types=self.separator.uses.memory_types,atf_blocks=len(self.separator.uses.atf_blocks),
            mode='dereverb',memory_index=1,memory_initializations_per_utterance=1,
            normalize_variance_correction=1,variance_restorations=1,external_audio_chunks=1,
            audio_crossfade=False,resampling=False,output_peak_normalization=False,
            post_encoder_time_halo=1 if backend=='bounded' else None,
            pre_decoder_time_halo=1 if backend=='bounded' else None,
            final_padding_after_bottleneck=True,whole_utterance_stft=True)
        self.info['equivalence_report_sha256']=None
        if backend=='bounded':_bind_equivalence(self.artifacts,self.info)
        self.last_trace=None

    def cleanup(self,audio,sr,mode='dereverb'):
        import torch
        if mode!='dereverb':raise ValueError('This bounded trial requires explicit mode dereverb')
        x=validate_audio(audio,sr)
        started=time.perf_counter();peak_before=_process_peak_rss_bytes()
        trace=dict(mode=mode,memory_indices=[],observed_separator_modes=[],sample_rate=sr,
            sample_count=len(x),fft=FFT,hop=HOP,frequency_bins=FFT//2+1,threads=torch.get_num_threads(),
            whole_utterance=True,external_chunks=1,memory_reset='upstream local initialization per forward',
            process_peak_rss_before_bytes=peak_before,execution_backend=self.info['execution_backend'],
            upstream_separator_forward_called=False,upstream_uses_forward_called=False)
        if not np.any(x):
            trace.update(silence_bypass=True,model_forward_calls=0,wall_seconds=time.perf_counter()-started,
                         process_peak_rss_bytes=_process_peak_rss_bytes(),output_sample_count=len(x))
            self.last_trace=trace
            return x.copy()
        if len(x)<=FFT//2:raise ValueError('Input is too short for upstream reflection padding')
        if torch.get_num_threads()!=4:raise RuntimeError('Isolated USES runtime must keep four CPU threads')

        def observe_memory(module,args,kwargs):
            trace['memory_indices'].append(kwargs.get('mem_idx'))
            trace['upstream_uses_forward_called']=True

        def observe_mode(module,args,kwargs):
            trace['observed_separator_modes'].append(kwargs.get('additional',{}).get('mode'))
            trace['upstream_separator_forward_called']=True

        hooks=[]
        if self.backend=='upstream':
            hooks.append(self.separator.uses.register_forward_pre_hook(observe_memory,with_kwargs=True))
            hooks.append(self.separator.register_forward_pre_hook(observe_mode,with_kwargs=True))
        try:
            with torch.inference_mode():
                normalized,scale=normalize(torch.from_numpy(x.copy()).unsqueeze(0))
                if not torch.isfinite(normalized).all():raise ValueError('Variance normalization is nonfinite')
                spectrum=encode(normalized,sr)
                frame_lengths=torch.tensor([spectrum.shape[1]],dtype=torch.long)
                if self.backend=='upstream':
                    spectra,_,_=self.separator(spectrum,frame_lengths,additional={'mode':mode})
                else:
                    from uses_bounded import bounded_separator
                    enhanced,execution_trace=bounded_separator(self.separator,spectrum,mode=mode)
                    trace.update(execution_trace)
                    spectra=[enhanced]
                if len(spectra)!=1 or not torch.is_complex(spectra[0]):
                    raise RuntimeError('Expected one native complex enhanced spectrum')
                result=decode(spectra[0],sr,len(x))*scale
                y=result.squeeze(0).cpu().numpy().astype('float32',copy=True)
                trace.update(input_spectrum_shape=list(spectrum.shape),output_spectrum_shape=list(spectra[0].shape))
        finally:
            for hook in hooks:hook.remove()
        if trace['memory_indices']!=[1]:
            raise RuntimeError('Actual upstream dereverb memory selection was not observed')
        if self.backend=='upstream' and trace['observed_separator_modes']!=['dereverb']:
            raise RuntimeError('Upstream separator mode hook did not observe dereverb')
        if self.backend=='bounded' and trace['memory_initialization_count']!=1:
            raise RuntimeError('Bounded backend did not preserve whole-utterance memory initialization')
        if y.shape!=x.shape or not np.isfinite(y).all():
            raise RuntimeError('USES output must be finite with exact original sample count')
        trace.update(silence_bypass=False,model_forward_calls=1 if self.backend=='upstream' else 0,
                     learned_backend_invocations=1,wall_seconds=time.perf_counter()-started,
                     process_peak_rss_bytes=_process_peak_rss_bytes(),output_sample_count=len(y))
        self.last_trace=trace
        return y


def validate_audio(audio,sr):
    x=np.asarray(audio,dtype='float32')
    if sr!=RATE or x.ndim!=1 or not len(x) or not np.isfinite(x).all():
        raise ValueError('USES trial requires finite, nonempty mono 48000Hz audio')
    return x


def encode(x,sr):
    import torch
    if sr!=RATE or x.ndim!=2 or x.shape[-1]<=FFT//2:
        raise ValueError('Native48k mono batch must exceed reflection-padding length')
    window=torch.hann_window(FFT,dtype=x.dtype,device=x.device)
    # Matches pinned ESPnet STFT after fs=48000 reconfiguration. No resampling,
    # magnitude exponent, spectrum scaling or frequency truncation.
    return torch.stft(x.float(),FFT,HOP,FFT,window,center=True,
                      normalized=False,onesided=True,return_complex=True).transpose(1,2)


def decode(spectrum,sr,count):
    import torch
    if sr!=RATE or spectrum.ndim!=3 or spectrum.shape[-1]!=FFT//2+1:
        raise ValueError('Unexpected full-band native48k spectrum')
    window=torch.hann_window(FFT,dtype=spectrum.real.dtype,device=spectrum.device)
    return torch.istft(spectrum.transpose(1,2),FFT,HOP,FFT,window,center=True,
                       normalized=False,onesided=True,length=count,return_complex=False)


def normalize(x):
    import torch
    scale=torch.std(x,dim=1,keepdim=True)
    # Exact silence is an explicit protective bypass of upstream division by0.
    scale=torch.where(scale==0,torch.ones_like(scale),scale)
    return x/scale,scale


def strict_load(model,state):
    expected=model.state_dict()
    missing=sorted(set(expected)-set(state));extra=sorted(set(state)-set(expected))
    shapes=[k for k in set(expected)&set(state) if expected[k].shape!=state[k].shape]
    if missing or extra or shapes:
        raise ValueError(f'Checkpoint mismatch: missing={missing}, extra={extra}, shapes={shapes}')
    model.load_state_dict(state,strict=True)
    return dict(tensor_count=len(state),element_count=sum(v.numel() for v in state.values()),
                missing=[],unexpected=[],shape_mismatches=[])


def room_conditions(clean,sr):
    from scipy.signal import fftconvolve
    x=validate_audio(clean,sr);early=x.copy();late=x.copy()
    for delay,gain in ((round(.014*sr),.25),(round(.037*sr),.12)):
        if delay<len(x):early[delay:]+=x[:-delay]*gain
    first=round(.05*sr);n=round(.8*sr)-first
    rng=np.random.default_rng(20261009)
    t=np.arange(n)/sr
    tail=rng.standard_normal(n)*np.exp(-np.log(1000)*t/.65)
    tail*=rng.uniform(size=n)<.05;tail*=.35/np.sqrt(np.sum(tail*tail))
    tail=tail.astype('float32')
    if first<len(x):
        convolved=fftconvolve(x,tail)[:len(x)-first].astype('float32')
        # FFT roundoff must not introduce a response before the first arrival.
        arrivals=np.flatnonzero(x)
        if len(arrivals):convolved[:arrivals[0]]=0
        late[first:]+=convolved
    return early,late
