"""Isolated native-48k trial of a pinned joint noise/room-trained DF3 model.

This does not register an application preset. It never downloads dependencies,
uses no generative synthesis, and refuses partial checkpoint loading.
"""
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


def validate_blend(value):
    if isinstance(value,bool) or not np.isfinite(value) or not 0<=value<=.3:
        raise ValueError('An explicit preservation repair blend must be finite and between 0 and .3')
    return float(value)


def fresh_evidence_directory(folder):
    folder=Path(folder)
    if folder.exists() and (not folder.is_dir() or any(folder.iterdir())):
        raise ValueError('Use a fresh evidence directory; historical audio is preserved')
    folder.mkdir(parents=True,exist_ok=True)
    return folder


def synthetic_room(clean,sr):
    """Declared synthetic room control, never fitted to Finnegan or Adobe."""
    x=validate_audio(clean,sr)
    rir=np.zeros(round(sr*.8),dtype='float32');rir[0]=1
    rir[round(.014*sr)]=.25;rir[round(.037*sr)]=.12
    rng=np.random.default_rng(20261009)
    first=round(.05*sr)
    t=np.arange(len(rir)-first)/sr
    tail=rng.standard_normal(len(t))*np.exp(-np.log(1000)*t/.65)
    tail*=rng.uniform(size=len(t))<.05
    tail*=.35/np.sqrt(np.sum(tail*tail))
    rir[first:]=tail
    # Direct causal accumulation avoids FFT roundoff before the first arrival.
    from scipy.signal import fftconvolve
    late=fftconvolve(x,rir[first:])[:max(0,len(x)-first)]
    room=x.copy()
    for delay,gain in ((round(.014*sr),.25),(round(.037*sr),.12)):
        if delay<len(x):room[delay:]+=x[:-delay]*gain
    if first<len(x):room[first:]+=late.astype('float32')
    return room,dict(synthetic_not_measured_room=True,seed=20261009,
                     direct_gain=1,early_taps_seconds=[.014,.037],early_tap_gains=[.25,.12],
                     late_start_seconds=.05,late_decay_parameter_seconds=.65,
                     late_rir_l2_norm=.35,late_tap_density=.05,rir_length_seconds=.8,
                     tail_truncated_to_source_sample_count=True)


def si_sdr(clean,output):
    clean=np.asarray(clean,dtype='float64');output=np.asarray(output,dtype='float64')
    alpha=np.dot(clean,output)/max(np.dot(clean,clean),1e-30)
    target=alpha*clean
    return float(10*np.log10(max(np.dot(target,target),1e-30)/
                            max(np.dot(output-target,output-target),1e-30)))


def controls(manifest_path,evidence_dir,model_dir):
    import soundfile as sf
    import studio
    folder=fresh_evidence_directory(evidence_dir)
    manifest=json.loads(Path(manifest_path).read_text())
    model=TrebleModel(model_dir)
    result=dict(model=model.info,source_manifest=str(Path(manifest_path).resolve()),
                source_manifest_sha256=qa.sha(manifest_path),
                clean_source_note='Published clean recording; not asserted perfectly anechoic',speakers={})
    for speaker in sorted({r['speaker'] for r in manifest['files']}):
        items=[r for r in manifest['files'] if r['speaker']==speaker]
        parts=[];boundaries=[];offset=0
        for record in items:
            if qa.sha(record['path'])!=record['sha256']:raise ValueError('Clean source hash changed')
            x,sr=sf.read(record['path'],dtype='float32')
            parts.append(validate_audio(x,sr))
            boundaries.append(dict(filename=record['filename'],start_sample=offset,samples=len(x),
                                   transcript=record['transcript']))
            offset+=len(x)
        clean=np.concatenate(parts)
        room,rir=synthetic_room(clean,RATE)
        # Same constant downward headroom gain in paired clean/room inputs.
        gain=min(1,.8/max(np.max(np.abs(clean)),np.max(np.abs(room)),1e-12))
        clean*=gain;room*=gain
        signals={'clean':clean,'synthetic_room':room}
        timings={}
        for key in ('clean','synthetic_room'):
            signals[key+'_model']=model.cleanup(signals[key],RATE)
            timings[key]=model.last_inference_seconds
        paths={}
        for key,audio in signals.items():
            path=folder/f'{speaker}_{key}.wav';studio.write_float(path,audio,RATE)
            paths[key]=dict(path=str(path.resolve()),sha256=qa.sha(path),sample_count=len(audio))
        mask=qa.masks(clean,RATE)
        baseline=qa.signal_metrics(clean,RATE)
        cleaned=qa.signal_metrics(signals['clean_model'],RATE,mask)
        cleaned.update(qa.alignment(clean,signals['clean_model'],RATE))
        preservation=qa.watchdog(baseline,cleaned)
        input_si=si_sdr(clean,room);output_si=si_sdr(clean,signals['synthetic_room_model'])
        result['speakers'][speaker]=dict(duration_seconds=len(clean)/RATE,source_boundaries=boundaries,
            common_input_gain=gain,synthetic_rir=rir,paths=paths,
            clean_input_preservation=dict(metrics=cleaned,source_metrics=baseline,watchdog=preservation,
                                          gain_invariant_si_sdr_db=si_sdr(clean,signals['clean_model'])),
            paired_room=dict(input_si_sdr_db=input_si,output_si_sdr_db=output_si,
                             improvement_db=output_si-input_si),inference_seconds=timings)
    import resource
    result['peak_process_rss_bytes']=resource.getrusage(resource.RUSAGE_SELF).ru_maxrss
    result['platform_rss_units']='bytes on macOS'
    result['caution']='Paired signal error and preservation warnings are diagnostic evidence, not speaker identity or listening approval'
    (folder/'controls.json').write_text(json.dumps(result,indent=2,allow_nan=False))
    print(json.dumps({s:dict(paired_room=r['paired_room'],clean_watchdog=r['clean_input_preservation']['watchdog'])
                      for s,r in result['speakers'].items()},indent=2),flush=True)
    return result


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


def render_trial(input_path,evidence_dir,model_dir,dry_blend=0):
    """Whole-context experiment; explicit QA rejection, no substituted fallback."""
    import soundfile as sf
    import speech_cleanup as sc
    import studio
    import resource
    dry_blend=validate_blend(dry_blend)
    folder=fresh_evidence_directory(evidence_dir)
    input_path=Path(input_path)
    source,sr,_=qa.decode(input_path,RATE)
    validate_audio(source,sr)
    speech,pause=qa.masks(source,sr)
    np.savez(folder/'source_masks.npz',speech=speech,pause=pause,sample_rate=sr,frame_seconds=.02)
    paths={}
    def save(name,audio,role='processing_stem'):
        path=folder/(name+'.wav');studio.write_float(path,audio,sr)
        paths[name]=dict(path=str(path.resolve()),sha256=qa.sha(path),role=role,sample_count=len(audio))
        return path
    save('prepared',source,'input')
    dry=studio.transparent_cleanup(source,sr);save('highpassed',dry)
    model=TrebleModel(model_dir)
    y=model.cleanup(dry,sr);save('model_raw',y)
    if dry_blend:y=((1-dry_blend)*y+dry_blend*dry).astype('float32')
    save('cleanup',y)
    y=studio.pause_control(y,source,sr,max_db=studio.CONFIG['pause_max_attenuation_db']);save('pause_control',y)
    presence=dict(gain_db=sc.presence_gain(y,sr),maximum_gain_db=2,center_hz=2400,q=.8,target_presence_to_mid_db=-11)
    y=sc.presence(y,sr,gain_db=presence['gain_db']);save('presence',y)
    trace={}
    y,leveling=sc.level_speech(y,source,sr,speech,trace=trace)
    np.save(folder/'level_frame_gain_db.npy',trace['frame_gain_db'])
    save('levelled',y)
    with tempfile.TemporaryDirectory(prefix='radcast-model-master-') as d:
        y,mastering=studio.master(y,sr,Path(d),compression=True)
    validate_output(source,y)
    master_path=save('master_attempt',y,'attempted_final_master')
    source_metrics=qa.signal_metrics(source,sr)
    measured=qa.signal_metrics(y,sr,(speech,pause));measured.update(qa.loudness(master_path))
    measured.update(qa.alignment(source,y,sr))
    defects=qa.watchdog(source_metrics,measured)
    # A rejected attempt stays visibly named as diagnostic evidence. A passing
    # file is moved once into its traceable listening name in this fresh folder.
    output=master_path
    if not defects:
        output=folder/f'CRJU160_modeltrial_ISM120_best119_dry{round(dry_blend*100):02d}.wav'
        master_path.rename(output)
        paths['master_attempt']['path']=str(output.resolve())
        paths['master_attempt']['role']='qa_accepted_listening_candidate'
    stages=['decode_float_48k','highpass_45hz','joint_room_noise_df3_ism_best119']
    if dry_blend:stages.append('explicit_original_preservation_blend')
    stages+=['guarded_pause_attenuation','bounded_presence_correction','speech_leveling',
             'light_compression','final_linear_loudness_true_peak']
    report=dict(schema_version=1,experiment_id='treble_ism_eng120_best119',
        input=str(input_path.resolve()),input_sha256=qa.sha(input_path),output=str(output.resolve()),
        output_sha256=qa.sha(output),sample_rate=sr,working_format='float32 PCM',
        stages=stages,effective_stages=stages,model=model.info,explicit_dry_blend=dry_blend,
        native_application_changed=False,fallback=False,accepted_for_listening=not defects,
        watchdog=defects,metrics=measured,source_metrics=source_metrics,
        mastering=mastering,presence=presence,leveling=leveling,
        downstream_config={k:studio.CONFIG[k] for k in (
            'highpass_hz','pause_max_attenuation_db','compression_ratio','compression_threshold',
            'target_lufs','true_peak_target_dbfs','eq_boost_db','deess_max_db',
            'leveling_max_gain_db','leveling_max_cut_db')},paths=paths,
        code_sha256={Path(module.__file__).name:qa.sha(module.__file__)
                     for module in (qa,studio,sc)},
        adapter_sha256=qa.sha(__file__),
        inference_seconds=model.last_inference_seconds,
        real_time_factor=model.last_inference_seconds/(len(source)/sr),
        peak_process_rss_bytes=resource.getrusage(resource.RUSAGE_SELF).ru_maxrss,
        runtime_identity_snapshot=str((folder.parent/'runtime-identity.json').resolve()),
        note='New joint cleanup block replaces prior room/DF/HF-blend block; effect is not attributed to checkpoint change alone. QA acceptance is not human approval.')
    report_path=folder/'trial.qa.json'
    report_path.write_text(json.dumps(report,indent=2,allow_nan=False))
    print(json.dumps({k:report[k] for k in ('output','accepted_for_listening','watchdog','inference_seconds')},indent=2),flush=True)
    return report


if __name__=='__main__':
    import argparse
    parser=argparse.ArgumentParser()
    parser.add_argument('--model-dir',required=True)
    mode=parser.add_mutually_exclusive_group(required=True)
    mode.add_argument('--controls-source')
    mode.add_argument('--input')
    parser.add_argument('--evidence-dir',required=True)
    parser.add_argument('--dry-blend',type=float,default=0)
    args=parser.parse_args()
    if args.controls_source:
        if args.dry_blend:parser.error('Control mode records unblended model output')
        controls(args.controls_source,args.evidence_dir,args.model_dir)
    else:render_trial(args.input,args.evidence_dir,args.model_dir,args.dry_blend)
