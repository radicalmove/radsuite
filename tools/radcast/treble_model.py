"""Pinned native 48 kHz Treble runtime; cached artifacts only."""
import configparser
import importlib.metadata
import json
import os
from pathlib import Path
import tempfile
import time
import numpy as np
import analysis as qa

RATE=48000
ARTIFACT_HASHES={
    'config.ini':'22aad5d5f95312051b755b61cbf050d68cf55d86bb1800931964f4b65983573b',
    'checkpoints/model_119.ckpt.best':'a3cc6a842ee7674c9763dabbe16ffd562cf617aed961b797dd0bc39d1e422fd1',
}

def verify_artifacts(folder):
    folder=Path(folder)
    for name,expected in ARTIFACT_HASHES.items():
        path=folder/name
        if not path.is_file():raise FileNotFoundError(f'Pinned artifact missing: {name}')
        if qa.sha(path)!=expected:raise ValueError(f'Pinned artifact hash mismatch: {name}')
    return ARTIFACT_HASHES.copy()


def as_array(tensor):
    if hasattr(tensor,'detach'):tensor=tensor.detach().cpu().numpy()
    return np.asarray(tensor)


def audit_state_dict(expected,checkpoint):
    missing=sorted(set(expected)-set(checkpoint))
    extra=sorted(set(checkpoint)-set(expected))
    mismatched=[k for k in set(expected)&set(checkpoint)
                if tuple(expected[k].shape)!=tuple(checkpoint[k].shape)]
    if missing or extra or mismatched:
        raise ValueError(f'Incomplete checkpoint: missing={missing}, unexpected={extra}, shape={mismatched}')
    for name,tensor in checkpoint.items():
        if not np.isfinite(as_array(tensor)).all():
            raise ValueError(f'Checkpoint tensor is not finite: {name}')
    return dict(tensor_count=len(checkpoint),element_count=sum(as_array(v).size for v in checkpoint.values()),
                missing=[],unexpected=[],shape_mismatches=[])


def validate_audio(audio,sr):
    x=np.asarray(audio)
    if sr!=RATE or x.ndim!=1 or not len(x) or not np.isfinite(x).all():
        raise ValueError('Model input must be finite, nonempty, mono 48000 Hz PCM')
    return x.astype('float32',copy=False)


def validate_output(source,output):
    y=validate_audio(output,RATE)
    if y.shape!=source.shape:raise ValueError('Model output must retain every source sample')
    return y


class TrebleModel:
    def __init__(self,model_dir):
        start=time.perf_counter()
        self.model_dir=Path(model_dir)
        hashes=verify_artifacts(self.model_dir)
        versions={p:importlib.metadata.version(p) for p in
                  ('torch','torchaudio','deepfilternet','deepfilterlib','numpy','scipy','soundfile')}
        required={'torch':'2.1.1','torchaudio':'2.1.1','deepfilternet':'0.5.6','deepfilterlib':'0.5.6',
                  'numpy':'1.26.2','scipy':'1.11.4','soundfile':'0.12.1'}
        if versions!=required:raise ValueError(f'Unpinned model-trial runtime: {versions}')
        import torch
        from df.enhance import init_df
        from df.config import config
        from libdf import DF
        torch.set_num_threads(min(4,os.cpu_count() or 1))
        torch.manual_seed(43)
        # Initialize architecture without the upstream permissive weight loader.
        # Only a disposable config copy gains runtime defaults/device overrides.
        parser=configparser.ConfigParser()
        parser.read(self.model_dir/'config.ini');parser.set('train','device','cpu')
        with tempfile.TemporaryDirectory(prefix='radcast-trial-config-') as d:
            with open(Path(d)/'config.ini','w') as f:parser.write(f)
            self.model,state,_=init_df(d,post_filter=False,log_level='ERROR',log_file=None,epoch='none')
            checkpoint=torch.load(self.model_dir/'checkpoints/model_119.ckpt.best',
                                  map_location='cpu',weights_only=True)
            audit=audit_state_dict(self.model.state_dict(),checkpoint)
            self.model.load_state_dict(checkpoint,strict=True)
            for name,value in self.model.state_dict().items():
                if not torch.equal(value.cpu(),checkpoint[name].cpu()):
                    raise ValueError(f'Loaded checkpoint differs: {name}')
            effective={s:dict(config.parser.items(s)) for s in config.parser.sections()}
        self.model.cpu().eval()
        if state.sr()!=RATE:raise ValueError('Selected checkpoint is not native 48 kHz')
        self.state_args=dict(sr=state.sr(),fft_size=state.fft_size(),hop_size=state.hop_size(),
                             nb_bands=32,min_nb_erb_freqs=2)
        self.make_state=DF
        self.info=dict(checkpoint_epoch=119,model_directory=str(self.model_dir.resolve()),
                       artifact_sha256=hashes,load_audit=audit,versions=versions,device='cpu',
                       sample_rate=RATE,effective_runtime_config=effective,
                       initialization_seconds=time.perf_counter()-start,
                       processing='observed-spectrum ERB gains and complex multi-frame filters',
                       postfilter=False,attenuation_limit_db=None,original_inference_blend=0,
                       delay_compensation_samples=state.fft_size()-state.hop_size(),
                       context='whole input; no segmented decoding or chunk joins')

    def cleanup(self,audio,sr):
        x=validate_audio(audio,sr)
        import torch
        from df.enhance import enhance
        # Fresh analysis/synthesis state plus upstream recurrent-state reset on
        # every input prevents preceding controls contaminating later renders.
        state=self.make_state(**self.state_args)
        start=time.perf_counter()
        y=enhance(self.model,state,torch.from_numpy(x[None].copy()),pad=True,
                  atten_lim_db=None).cpu().numpy()[0]
        self.last_inference_seconds=time.perf_counter()-start
        return validate_output(x,y)
