"""Production Treble with per-recording calibration and guarded fallback."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import tempfile
import numpy as np
import soundfile as sf
import analysis as qa
import studio
import speech_cleanup as sc
from treble_model import TrebleModel, validate_audio, verify_artifacts
from treble_preservation import apply

CONFIG=dict(preset_id='studio_treble',revision='r1',maximum_original=.2,
            calibration_original_weight=.2,whole_recording=True,
            highpass_hz=45,target_lufs=studio.CONFIG['target_lufs'],true_peak_dbfs=-1.5)

def model_path():
    return Path(os.environ.get('RADSUITE_STUDIO_TREBLE_MODEL',str(Path.home()/'.radcast/models/treble-ism-eng120-epoch119')))

def protection_reference(raw,hp):
    return (.8*raw+.2*hp).astype('float32')

def with_frame_gains(x,gains):
    gains=np.asarray(gains)
    if not len(gains) or not np.isfinite(gains).all():raise ValueError('Invalid calibration gain curve')
    curve=np.interp(np.arange(len(x)),np.arange(len(gains))*960+480,gains,left=gains[0],right=gains[-1])
    return (x*10**(curve/20)).astype('float32')

def array_identity(x,dtype):
    a=np.asarray(x,dtype=dtype)
    return dict(sha256=hashlib.sha256(a.tobytes()).hexdigest(),count=a.size,dtype=dtype)

def render(input_path,output_path,model_dir=None):
    input_path=Path(input_path);output_path=Path(output_path)
    if input_path.resolve()==output_path.resolve():raise ValueError('Output must differ from original')
    output_path.parent.mkdir(parents=True,exist_ok=True)
    info=sf.info(input_path) if input_path.suffix.lower()=='.wav' else None
    if info and info.samplerate==48000:
        source,sr=sf.read(input_path,dtype='float32',always_2d=True);source=source.mean(axis=1)
    else:source,sr,_=qa.decode(input_path,48000)
    source=validate_audio(source,sr);hp=studio.transparent_cleanup(source,sr)
    masks=qa.masks(source,sr);source_metrics=qa.signal_metrics(source,sr)
    model_dir=Path(model_dir) if model_dir is not None else model_path()
    identities={};baseline_metrics=None;baseline_mastering=None
    warnings=[];fallback=False;diagnosis=None;model_info=None;presence_gain=None;level_info=None;rejected=[];baseline_defects=[]
    with tempfile.TemporaryDirectory(prefix='radcast-treble-') as temp:
        folder=Path(temp)
        def evaluate(x,reference=source,reference_metrics=source_metrics):
            path=folder/'qa.wav';studio.write_float(path,x,sr)
            measured=qa.signal_metrics(x,sr,masks);measured.update(qa.loudness(path));measured.update(qa.alignment(reference,x,sr))
            return measured,qa.watchdog(reference_metrics,measured)
        try:
            print('RADCAST_ENHANCE_PROGRESS 0/1',flush=True)
            model=TrebleModel(model_dir);raw=model.cleanup(hp,sr)
            model_info=dict(model.info,inference_seconds=model.last_inference_seconds)
            reference=protection_reference(raw,hp)
            identities.update(raw_model=array_identity(raw,"<f4"),protection_reference=array_identity(reference,"<f4"))
            calibrated=studio.pause_control(reference,source,sr)
            presence_gain=sc.presence_gain(calibrated,sr)
            calibrated=sc.presence(calibrated,sr,gain_db=presence_gain)
            trace={};calibrated,level_info=sc.level_speech(calibrated,source,sr,masks[0],trace=trace)
            identities['calibrated_frame_gain_db']=array_identity(trace['frame_gain_db'],"<f8")
            baseline,baseline_mastering=studio.master(calibrated,sr,folder)
            baseline_metrics,_=evaluate(baseline)
            y,weights,diagnosis=apply(source,hp,raw,sr,protection_reference=reference)
            y=studio.pause_control(y,source,sr)
            y=sc.presence(y,sr,gain_db=presence_gain)
            y=with_frame_gains(y,trace['frame_gain_db'])
            y,mastering=studio.master(y,sr,folder)
            measured,rejected=evaluate(y)
            _,baseline_defects=evaluate(y,baseline,baseline_metrics)
            if rejected or baseline_defects:raise ValueError(f'Preservation check failed: {rejected}; reference: {baseline_defects}')
        except (Exception,SystemExit) as exc:
            fallback=True;warnings.append(dict(code='conservative_fallback',detail=f'{type(exc).__name__}: {exc}'))
            y=studio.pause_control(hp,source,sr)
            y,mastering=studio.master(y,sr,folder,compression=False)
            measured,defects=evaluate(y)
            if defects:raise ValueError(f'Conservative fallback failed preservation check: {defects}')
        studio.write_float(output_path,y,sr)
    report=dict(schema_version=1,config=dict(CONFIG,presence_gain_db=presence_gain,leveling=level_info),
        input=str(input_path.resolve()),input_sha256=qa.sha(input_path),output=str(output_path.resolve()),output_sha256=qa.sha(output_path),
        sample_rate=sr,working_format='float32 PCM',model_path=str(model_dir),model=model_info,
        calibrated_stem_identities=identities,baseline_metrics=baseline_metrics,baseline_mastering=baseline_mastering,
        controller=diagnosis,fallback=fallback,warnings=warnings,watchdog=[],
        rejected_path_watchdog=rejected,watchdog_against_baseline=baseline_defects,
        metrics=measured,source_metrics=source_metrics,mastering=mastering,
        effective_stages=(['highpass_45hz','guarded_pauses','scalar_master'] if fallback else
        ['highpass_45hz','treble_epoch119','adaptive_faint_speech_protection','guarded_pauses','calibrated_presence','calibrated_leveling','light_compression','scalar_master']))
    output_path.with_suffix('.qa.json').write_text(json.dumps(report,indent=2,allow_nan=False))
    print('RADCAST_ENHANCE_PROGRESS 1/1',flush=True)
    return report

def main():
    p=argparse.ArgumentParser();p.add_argument('input',nargs='?');p.add_argument('output',nargs='?');p.add_argument('--model-dir');p.add_argument('--check-runtime',action='store_true');p.add_argument('--verify-export',nargs=3);p.add_argument('--origin');p.add_argument('--edited',action='store_true');p.add_argument('--removed-seconds',type=float,default=0)
    a=p.parse_args()
    if a.check_runtime:
        TrebleModel(a.model_dir or model_path());return
    if a.verify_export:r=studio.verify_export(*a.verify_export,origin=a.origin,edited=a.edited,removed_seconds=a.removed_seconds)
    else:
        if not a.input or not a.output:p.error('Input and output are required')
        r=render(a.input,a.output,a.model_dir)
    print(json.dumps(dict(fallback=r['fallback'],warnings=r['warnings'])))

if __name__=='__main__':main()
