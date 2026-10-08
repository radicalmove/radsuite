"""Pinned URGENT25 TFGridNetV3 with explicit native48k window execution."""
import hashlib
import importlib
import json
from pathlib import Path
import resource
import sys
import time
import numpy as np
from uses_trial import validate_audio,encode,decode,normalize,strict_load

RATE=48000
WINDOW=144000
HOP=96000
WEIGHT_SHA='8350b6f84bb5de01646b7cebe9d19d5b1fc4318cd85c31d242caccc2442d276e'
CONFIG_SHA='9591c84a91349834fb7a31a021fa4d0858b46c001c6aa7ab096f0ee502684a61'
VENDOR_SHA='d3d51b2dc41886f467d04fc25895ad7ff3b728a5f9336c487c76080ab46a3406'
CONFIG=dict(n_srcs=1,n_imics=1,n_layers=6,lstm_hidden_units=200,attn_n_head=4,
    attn_qk_output_channel=2,emb_dim=48,emb_ks=4,emb_hs=1,activation='prelu',eps=1e-5)
POLICY=dict(window_samples=WINDOW,hop_samples=HOP,overlap_samples=WINDOW-HOP,
    overlap='half-sample raised cosine; normalized sum',context='independent neural windows',
    resampling=False,output_peak_normalization=False,between_window_gain_matching=False)


def sha(path):return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def overlap_apply(audio,sr,process):
    x=validate_audio(audio,sr);n=len(x);y=np.zeros(n,dtype='float64');weight=np.zeros(n,dtype='float64');windows=[]
    energy=np.zeros(n,dtype='float64');coverage=np.zeros(n,dtype='uint8')
    overlap=WINDOW-HOP;start=0
    while True:
        end=min(n,start+WINDOW);part=x[start:end].copy();out=np.asarray(process(part))
        if out.shape!=part.shape or out.dtype!=np.float32 or not np.isfinite(out).all():
            raise ValueError('Neural window output must be finite float32 with unchanged length')
        w=np.ones(len(part),dtype='float64')
        if start>0:
            k=min(overlap,len(part));w[:k]=.5-.5*np.cos(np.pi*(np.arange(k)+.5)/overlap)
        if end<n:
            k=min(overlap,len(part));w[-k:]=.5+.5*np.cos(np.pi*(np.arange(k)+.5)/overlap)
        y[start:end]+=out*w;weight[start:end]+=w
        energy[start:end]+=out.astype('float64')**2*w;coverage[start:end]+=1
        windows.append(dict(start_sample=start,end_sample=end))
        if end==n:break
        start+=HOP
    if np.any(weight<=0):raise ValueError('Window policy left uncovered samples')
    result=(y/weight).astype('float32')
    import analysis as qa
    expected=qa.frames(np.sqrt(energy/weight),sr);actual=qa.frames(result,sr)
    joined=(coverage[:len(expected)*960].reshape(-1,960)>1).any(axis=1)
    eligible=joined&(expected>1e-12)
    delta=20*np.log10(np.maximum(actual,1e-30)/np.maximum(expected,1e-30))
    protected=eligible&qa.masks(x,sr)[0]
    return result,dict(POLICY,windows=windows,sample_count=n,minimum_weight=float(weight.min()),maximum_weight=float(weight.max()),
        join_energy_worst_db=float(delta[eligible].min()) if np.any(eligible) else None,
        join_energy_protected_worst_db=float(delta[protected].min()) if np.any(protected) else None,
        join_energy_note='Merged vs weighted per-window frame energy; includes amplitude and phase variation, not a pure phase or reverb estimate.')


class TFGridModel:
    def __init__(self,artifacts,device='cpu'):
        import torch,yaml
        if device not in ('cpu','mps'):raise ValueError('Unsupported local device')
        if device=='mps' and not torch.backends.mps.is_available():raise ValueError('Local MPS device unavailable')
        self.device=device
        self.artifacts=Path(artifacts).resolve();manifest=json.loads((self.artifacts.parent/'artifact-manifest.json').read_text())
        for name in ('config.yaml','model.pth'):
            if sha(self.artifacts/name)!=manifest['files'][name]['sha256']:raise ValueError('Published artifact hash changed')
        if sha(self.artifacts/'model.pth')!=WEIGHT_SHA:raise ValueError('Wrong released checkpoint')
        if sha(self.artifacts/'config.yaml')!=CONFIG_SHA:raise ValueError('Published configuration changed')
        c=yaml.safe_load((self.artifacts/'config.yaml').read_text())
        if c['separator']!='tfgridnetv3' or c['separator_conf']!=CONFIG or c['model_conf'].get('normalize_variance_per_ch') is not True:
            raise ValueError('Unsupported published model configuration')
        if c['encoder_conf']!=dict(n_fft=256,hop_length=128,use_builtin_complex=True,default_fs=8000) or c['decoder_conf']!=dict(n_fft=256,hop_length=128,default_fs=8000):
            raise ValueError('Unsupported native-rate STFT configuration')
        vendor=self.artifacts.parent/'vendor';audit=json.loads((vendor/'vendor-audit.json').read_text())
        if sha(vendor/'vendor-audit.json')!=VENDOR_SHA:raise ValueError('Pinned vendor audit changed')
        for name,digest in audit['files'].items():
            if sha(vendor/'radcast_tfgrid_vendor'/name)!=digest:raise ValueError('Vendor source changed')
        path=str(vendor)
        if path not in sys.path:sys.path.insert(0,path)
        package=importlib.import_module('radcast_tfgrid_vendor')
        if Path(package.__file__).resolve()!=(vendor/'radcast_tfgrid_vendor/__init__.py').resolve():raise ValueError('Unexpected vendor import')
        torch.set_num_threads(4)
        self.model=torch.nn.Module();self.model.add_module('separator',package.TFGridNetV3(input_dim=769,**CONFIG));self.model.float().cpu().eval()
        state=torch.load(self.artifacts/'model.pth',map_location='cpu',weights_only=True);info=strict_load(self.model,state)
        if info['tensor_count']!=270 or info['element_count']!=8524352:raise ValueError('Unexpected model tensors')
        if device=='mps':
            # Preserve publisher forward/class math. Only real neural features
            # cross devices; complex STFT and inverse remain native CPU.
            self.model.to(device)
            self.model.separator.conv.register_forward_pre_hook(lambda module,args:(args[0].to('mps'),))
            self.model.separator.deconv.register_forward_hook(lambda module,args,out:out.to('cpu'))
        self.info=dict(info,model_family='TFGridNetV3 URGENT2025',model_revision=manifest['model_revision'],
            publisher_commit=audit['publisher_commit'],checkpoint_sha256=WEIGHT_SHA,config_sha256=sha(self.artifacts/'config.yaml'),
            vendor_audit_sha256=sha(vendor/'vendor-audit.json'),adapter_sha256=sha(__file__),
            stft_helper_sha256=sha(Path(__file__).with_name('uses_trial.py')),torch_version=torch.__version__,
            device=device,complex_stft_device='cpu',threads=4,dtype='float32',sample_rate=RATE,fft=1536,hop=768,frequency_bins=769,
            window_policy=POLICY,output_peak_normalization=False,resampling=False,cascade=False,
            target='Published preparation retains approximately50ms early response; not guaranteed anechoic.',
            variance='Per-window torch.std(correction=1); restored once before overlap')
        self.last_trace=None

    def cleanup(self,audio,sr):
        import torch
        x=validate_audio(audio,sr);details=[];started=time.perf_counter()
        def process(part):
            if not np.any(part):
                details.append(dict(silence_bypass=True,neural_calls=0));return part.copy()
            if len(part)<=768:raise ValueError('Nonzero input too short for native reflection padding')
            with torch.inference_mode():
                norm,scale=normalize(torch.from_numpy(part).unsqueeze(0));spectrum=encode(norm,sr)
                outputs,_,_=self.model.separator(spectrum,torch.tensor([spectrum.shape[1]],dtype=torch.long))
                if len(outputs)!=1:raise ValueError('Expected one predicted source')
                y=(decode(outputs[0],sr,len(part))*scale).squeeze(0).numpy().astype('float32',copy=True)
            details.append(dict(silence_bypass=False,neural_calls=1,spectrum_shape=list(spectrum.shape),input_std=float(scale)))
            print(f'TFGRID window {len(details)} complete',flush=True)
            return y
        y,trace=overlap_apply(x,sr,process)
        trace.update(window_details=details,frequency_bins=769,sample_rate=sr,wall_seconds=time.perf_counter()-started,
            peak_process_rss_bytes=int(resource.getrusage(resource.RUSAGE_SELF).ru_maxrss),
            whole_context_equivalence_claimed=False,global_input_sample_count=len(x))
        self.last_trace=trace;return y
