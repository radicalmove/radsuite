"""One offline adaptive-preservation probe; no model inference or preset change."""
import json
from pathlib import Path
import tempfile
import numpy as np
from scipy.ndimage import maximum_filter1d
import soundfile as sf
import analysis as qa
import studio
import speech_cleanup as sc
from model_trial import validate_audio, fresh_evidence_directory
from presence_probe import read_float, verified_baseline

RATE=48000
GAIN_SHA='c5207f910a49b619221cfa96141bdde7a947001e256190397bbb04624dc4bab3'
CONFIG=dict(onset_relative_db=-9.,full_relative_db=-12.,maximum_original=.20,
            frame_seconds=.02,expansion_frames_each_side=8,smoothing_frames=5,
            energy_guard_max_loss_db=.1,offline_whole_recording_statistics=True)


def activation(relative_gain):
    return .2*np.clip((-np.asarray(relative_gain)-9)/3,0,1)


def sample_curve(weights,count,sr):
    if not len(weights):return np.zeros(count)
    size=round(sr*.02)
    return np.interp(np.arange(count),np.arange(len(weights))*size+size/2,
                     weights,left=weights[0],right=weights[-1])


def intervals(mask):
    d=np.diff(np.r_[False,mask,False].astype(int))
    return list(zip(np.flatnonzero(d==1),np.flatnonzero(d==-1)))


def apply(source,highpassed,model,sr):
    source=validate_audio(source,sr);highpassed=validate_audio(highpassed,sr);model=validate_audio(model,sr)
    if source.shape!=model.shape or highpassed.shape!=model.shape:
        raise ValueError('Original-derived stems must have identical sample counts')
    n=len(source)//960;weights=np.zeros(n);records=[]
    detector=qa.speech_window_gain(source,model,sr) if n>=50 else dict(speech_window_gain_median_db=None)
    median=detector['speech_window_gain_median_db']
    if median is not None:
        for start,gain in zip(detector['speech_window_starts_seconds'],detector['speech_window_gain_db']):
            relative=gain-median;weight=float(activation(relative))
            if weight<=0:continue
            first=round(start*50);last=min(n,first+50)
            weights[first:last]=np.maximum(weights[first:last],weight)
            records.append(dict(start_seconds=start,end_seconds=start+1,
                                gain_db=gain,relative_gain_db=relative,original_weight=weight))
        weights=maximum_filter1d(weights,size=17,mode='constant',cval=0)
        weights=np.convolve(weights,np.ones(5)/5,mode='same')
    weights=np.clip(weights,0,.2)
    def mix(w):
        curve=sample_curve(w,len(source),sr)
        return ((1-curve)*model+curve*highpassed).astype('float32')
    y=mix(weights);speech,_=qa.masks(source,sr)
    before=qa.frames(model,sr);original=qa.frames(source,sr)
    eligible=speech&(original>1e-5)&(before>1e-8)
    def conflicts(output):
        after=qa.frames(output,sr)
        delta=20*np.log10(np.maximum(after,1e-30)/np.maximum(before,1e-30))
        return eligible&(delta<-.1),delta
    bad,delta=conflicts(y);initial_bad=int(bad.sum());bypassed=[]
    for start,end in intervals(weights>0):
        if np.any(bad[max(0,start-1):min(n,end+1)]):
            weights[start:end]=0
            bypassed.append(dict(start_frame=int(start),end_frame_exclusive=int(end),
                                 start_seconds=start*.02,end_seconds=end*.02))
    if bypassed:y=mix(weights)
    remaining,delta=conflicts(y)
    if np.any(remaining):raise ValueError('Post-bypass blend still reduces protected model-frame energy')
    if y.shape!=source.shape or not np.isfinite(y).all():raise ValueError('Invalid preservation output')
    return y,weights,dict(config=CONFIG.copy(),median_eligible_model_gain_db=median,
        triggered_windows=records,initial_energy_guard_conflict_frames=initial_bad,
        energy_guard_bypassed_intervals=bypassed,remaining_energy_guard_conflicts=int(remaining.sum()),
        minimum_protected_candidate_vs_model_frame_gain_db=float(delta[eligible].min()) if np.any(eligible) else None,
        active_frame_count=int(np.count_nonzero(weights)),frame_count=n,
        active_intervals=[dict(start_seconds=int(a)*.02,end_seconds=int(b)*.02) for a,b in intervals(weights>0)],
        sample_count=len(y),note='Source-periodicity/modulation and frame energy are protection proxies, not semantic speech or reverb estimates.')


def level_with_saved_gain(x,gains):
    curve=np.interp(np.arange(len(x)),np.arange(len(gains))*960+480,gains,left=gains[0],right=gains[-1])
    return (x*10**(curve/20)).astype('float32')


def render(baseline_report,outdir):
    baseline_report=Path(baseline_report)
    base,source,old_levelled,old_master=verified_baseline(baseline_report)
    if base['explicit_dry_blend']!=.2:raise ValueError('Expected the frozen dry20 baseline')
    for module in (qa,studio,sc):
        if qa.sha(module.__file__)!=base['code_sha256'][Path(module.__file__).name]:
            raise ValueError('Baseline processing helper changed')
    audio={}
    for key in ('highpassed','model_raw','cleanup','pause_control','presence'):
        record=base['paths'][key]
        if qa.sha(record['path'])!=record['sha256']:raise ValueError('Baseline stem hash changed')
        audio[key]=read_float(record['path'])
        if len(audio[key])!=len(source):raise ValueError('Stage count changed')
    gain_path=baseline_report.parent/'level_frame_gain_db.npy'
    if qa.sha(gain_path)!=GAIN_SHA:raise ValueError('Saved leveling gain hash changed')
    gains=np.load(gain_path,allow_pickle=False)
    if len(gains)!=len(source)//960 or not np.isfinite(gains).all():raise ValueError('Invalid saved gains')
    def downstream(cleanup):
        pause=studio.pause_control(cleanup,source,RATE)
        presence=sc.presence(pause,RATE,gain_db=base['presence']['gain_db'])
        levelled=level_with_saved_gain(presence,gains)
        return pause,presence,levelled
    old=downstream(audio['cleanup'])
    errors={k:float(np.max(abs(x-expected))) for k,x,expected in zip(
        ('pause_control','presence','levelled'),old,(audio['pause_control'],audio['presence'],old_levelled))}
    with tempfile.TemporaryDirectory(prefix='radcast-adaptive-baseline-') as temp:
        reproduced,old_mastering=studio.master(old[-1],RATE,Path(temp))
    errors['master']=float(np.max(abs(reproduced-old_master)))
    if max(errors.values())>3e-7:raise ValueError('Frozen baseline replay differs')
    folder=fresh_evidence_directory(outdir);paths={}
    def save(name,x,role='processing_stem'):
        path=folder/(name+'.wav');studio.write_float(path,x,RATE)
        paths[name]=dict(path=str(path.resolve()),sha256=qa.sha(path),sample_count=len(x),role=role)
        return path
    cleanup,weights,diagnosis=apply(source,audio['highpassed'],audio['model_raw'],RATE)
    np.save(folder/'blend_frame_weights.npy',weights)
    (folder/'controller.json').write_text(json.dumps(diagnosis,indent=2,allow_nan=False)+'\n')
    save('cleanup',cleanup)
    target=slice(20*RATE,32*RATE)
    target_exact=np.array_equal(cleanup[target],audio['model_raw'][target])
    if not target_exact:
        (folder/'stopped.json').write_text(json.dumps(dict(reason='Recovery reaches preferred target; isolation failed. No exclusions or tuning.',target_cleanup_identical=False),indent=2))
        raise ValueError('Adaptive recovery reaches preferred target; round stopped')
    pause,presence,levelled=downstream(cleanup)
    save('pause_control',pause);save('presence',presence);save('levelled',levelled)
    with tempfile.TemporaryDirectory(prefix='radcast-adaptive-master-') as temp:
        y,mastering=studio.master(levelled,RATE,Path(temp))
    path=save('master_attempt',y,'attempted_master')
    masks=qa.masks(source,RATE);source_metrics=qa.signal_metrics(source,RATE)
    measured=qa.signal_metrics(y,RATE,masks);measured.update(qa.loudness(path));measured.update(qa.alignment(source,y,RATE))
    defects=qa.watchdog(source_metrics,measured)
    before=qa.signal_metrics(old_master,RATE,masks);before.update(qa.loudness(base['output']))
    relative=qa.signal_metrics(y,RATE,masks);relative.update(qa.loudness(path));relative.update(qa.alignment(old_master,y,RATE))
    baseline_defects=qa.watchdog(before,relative);accepted=not defects and not baseline_defects
    if accepted:
        delivered=folder/'CRJU160_adaptive_original_preservation.wav';path.rename(delivered);path=delivered
        paths['master_attempt'].update(path=str(path.resolve()),role='qa_accepted_listening_candidate')
    report=dict(schema_version=1,experiment='adaptive_original_preservation',
        input=base['input'],input_sha256=base['input_sha256'],output=str(path.resolve()),output_sha256=qa.sha(path),
        sample_rate=RATE,working_format='float32 PCM',baseline_report=str(baseline_report.resolve()),
        baseline_report_sha256=qa.sha(baseline_report),baseline_master_sha256=base['output_sha256'],
        original_derived_inputs={k:base['paths'][k] for k in ('prepared','highpassed','model_raw')},
        saved_level_gain=dict(path=str(gain_path.resolve()),sha256=GAIN_SHA),baseline_replay_max_errors=errors,
        frozen_presence=base['presence'],frozen_downstream_config=base['downstream_config'],mastering=mastering,
        new_model_inference=False,native_application_changed=False,fallback=False,
        full_candidate_count=1,remaining_render_budget=0,target_cleanup_identical_to_raw_model=bool(target_exact),
        stages=['verified_original_derived_48k_float_stems','adaptive_original_preservation',
                'frozen_pause_policy','frozen_presence_gain','frozen_level_gain','light_compression','final_scalar_master'],
        controller=diagnosis,paths=paths,accepted_for_listening=accepted,watchdog=defects,
        watchdog_against_baseline=baseline_defects,metrics=measured,metrics_against_baseline=relative,
        source_metrics=source_metrics,baseline_metrics=before,
        code_sha256={Path(m.__file__).name:qa.sha(m.__file__) for m in (qa,studio,sc)},adapter_sha256=qa.sha(__file__),
        note='One frozen-level calibration experiment, not production adaptive gain or human voice approval.')
    (folder/'trial.qa.json').write_text(json.dumps(report,indent=2,allow_nan=False)+'\n')
    print(json.dumps(dict(accepted=accepted,watchdog=defects,baseline_watchdog=baseline_defects,
                         replay_errors=errors,active_intervals=diagnosis['active_intervals'],
                         energy_bypasses=diagnosis['energy_guard_bypassed_intervals'],output=str(path)),indent=2),flush=True)
    return report


if __name__=='__main__':
    import argparse
    p=argparse.ArgumentParser();p.add_argument('--baseline-report',required=True);p.add_argument('--outdir',required=True)
    a=p.parse_args();render(a.baseline_report,a.outdir)
