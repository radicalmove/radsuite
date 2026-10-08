"""One bounded tonal experiment resuming a verified original-source float stage.

No model inference, resampling, new dependencies, adaptive gain or app preset.
"""
import json
from pathlib import Path
import tempfile
import numpy as np
from scipy import signal
import soundfile as sf
import analysis as qa
import studio
from model_trial import fresh_evidence_directory

RATE=48000
FILTERS=((180.,.8,1.),(3000.,.8,2.),(5200.,1.,1.5))


def bell(center,q,gain,sr):
    a=10**(gain/40);w=2*np.pi*center/sr;alpha=np.sin(w)/(2*q)
    b=np.array([1+alpha*a,-2*np.cos(w),1-alpha*a])
    d=np.array([1+alpha/a,-2*np.cos(w),1-alpha/a])
    return np.r_[b/d[0],d/d[0]]


def coefficients(sr):
    if sr!=RATE:raise ValueError('Presence probe requires native mono 48000 Hz PCM')
    return np.array([bell(*f,sr) for f in FILTERS])


def response_db(frequencies,sr):
    f=np.asarray(frequencies,dtype='float64')
    if not np.isfinite(f).all() or np.any((f<0)|(f>=sr/2)):
        raise ValueError('Response frequencies must be finite and below Nyquist')
    _,h=signal.sosfreqz(coefficients(sr),worN=f,fs=sr)
    return 20*np.log10(np.maximum(np.abs(h),1e-30))


def apply(audio,sr):
    x=np.asarray(audio)
    if sr!=RATE or x.ndim!=1 or not len(x) or not np.isfinite(x).all():
        raise ValueError('Presence input must be finite, nonempty mono 48000 Hz PCM')
    y=signal.sosfilt(coefficients(sr),x).astype('float32')
    if y.shape!=x.shape or not np.isfinite(y).all():raise ValueError('Invalid filtered output')
    return y


def read_float(path):
    info=sf.info(path)
    if info.samplerate!=RATE or info.channels!=1 or info.subtype!='FLOAT':
        raise ValueError('Saved processing stage must be mono 48000 Hz FLOAT WAV')
    x,sr=sf.read(path,dtype='float32')
    if not len(x) or not np.isfinite(x).all():raise ValueError('Invalid saved audio')
    return x


def verified_baseline(report_path):
    report=json.loads(Path(report_path).read_text())
    if not report.get('accepted_for_listening') or report.get('fallback') or report.get('watchdog'):
        raise ValueError('Baseline must be QA accepted, nonfallback audio')
    if qa.sha(report['output'])!=report['output_sha256']:
        raise ValueError('Baseline master hash changed')
    for key in ('prepared','levelled'):
        p=report['paths'][key]
        if qa.sha(p['path'])!=p['sha256']:raise ValueError(f'Baseline {key} hash changed')
    if qa.sha(report['input'])!=report['input_sha256']:raise ValueError('Original source hash changed')
    source=read_float(report['paths']['prepared']['path'])
    levelled=read_float(report['paths']['levelled']['path']);base=read_float(report['output'])
    if len(source)!=len(levelled) or len(source)!=len(base):raise ValueError('Baseline stage counts differ')
    return report,source,levelled,base


def render(baseline_report,outdir):
    baseline_report=Path(baseline_report)
    base,source,levelled,master=verified_baseline(baseline_report)
    f=np.geomspace(20,23999,12000);g=response_db(f,RATE)
    if np.max(g)>3.5 or np.max(g[f<80])>.5 or np.min(g)<-.01:
        raise ValueError('Frozen extra tonal response violates bounds')
    folder=fresh_evidence_directory(outdir);paths={}
    def save(name,x,role='diagnostic'):
        p=folder/(name+'.wav');studio.write_float(p,x,RATE)
        paths[name]=dict(path=str(p.resolve()),sha256=qa.sha(p),role=role,sample_count=len(x))
        return p
    # Resume a source-derived uncompressed stage, never a delivered enhanced
    # WAV/MP3. Compression is reproduced exactly once on this signal.
    with tempfile.TemporaryDirectory(prefix='radcast-presence-compress-') as d:
        p=Path(d)/'compressed.wav'
        qa.run(['ffmpeg','-v','error','-i',base['paths']['levelled']['path'],'-af',
                'acompressor=threshold=0.18:ratio=1.15:attack=20:release=220:makeup=1',
                '-ar',RATE,'-c:a','pcm_f32le',p])
        compressed=read_float(p)
    previous_gain=base['mastering']['final_gain_db']
    reproduced=(compressed*10**(previous_gain/20)).astype('float32')
    error=float(np.max(np.abs(reproduced-master)))
    if error>3e-7:raise ValueError('Frozen compressor does not reproduce baseline within float rounding')
    save('compressed',compressed)
    filtered=apply(compressed,RATE);save('extra_tonal_pre_master',filtered)
    with tempfile.TemporaryDirectory(prefix='radcast-presence-master-') as d:
        y,mastering=studio.master(filtered,RATE,Path(d),compression=False)
    path=save('master_attempt',y,'attempted_master')
    masks=qa.masks(source,RATE);source_metrics=qa.signal_metrics(source,RATE)
    metrics=qa.signal_metrics(y,RATE,masks);metrics.update(qa.loudness(path));metrics.update(qa.alignment(source,y,RATE))
    defects=qa.watchdog(source_metrics,metrics)
    before=qa.signal_metrics(master,RATE,masks);before.update(qa.loudness(base['output']))
    vs_base=qa.signal_metrics(y,RATE,masks);vs_base.update(qa.loudness(path));vs_base.update(qa.alignment(master,y,RATE))
    base_defects=qa.watchdog(before,vs_base)
    accepted=not defects and not base_defects
    if accepted:
        delivered=folder/'CRJU160_presence_body1_definition2_detail1p5.wav';path.rename(delivered);path=delivered
        paths['master_attempt'].update(path=str(path.resolve()),role='qa_accepted_listening_candidate')
    p=base['presence'];_,h=signal.sosfreqz([bell(p['center_hz'],p['q'],p['gain_db'],RATE)],worN=f,fs=RATE)
    nominal=g+20*np.log10(np.maximum(abs(h),1e-30))
    response=dict(frequencies_hz=f.tolist(),extra_gain_db=g.tolist(),nominal_sum_of_old_and_extra_eq_db=nominal.tolist(),
        extra_peak_gain_db=float(g.max()),extra_sub80_peak_gain_db=float(g[f<80].max()),nominal_sum_peak_gain_db=float(nominal.max()),
        note='Nominal EQ sum is not a complete transfer function: baseline compression/model/dynamics intervene.')
    (folder/'response.json').write_text(json.dumps(response,indent=2,allow_nan=False))
    report=dict(schema_version=1,experiment='bounded_presence_probe',input=base['input'],input_sha256=base['input_sha256'],
        baseline_report=str(baseline_report.resolve()),baseline_report_sha256=qa.sha(baseline_report),
        baseline_master=str(Path(base['output']).resolve()),baseline_master_sha256=base['output_sha256'],
        resume_stage=base['paths']['levelled'],prepared_original=base['paths']['prepared'],
        output=str(path.resolve()),output_sha256=qa.sha(path),sample_rate=RATE,working_format='float32 PCM',
        filters=[dict(center_hz=c,q=q,gain_db=a) for c,q,a in FILTERS],baseline_presence=base['presence'],
        effective_stages=base['effective_stages'][:-1]+['extra_bounded_tonal_probe','final_linear_loudness_true_peak'],
        baseline_compressor_reproduction_max_abs_error=error,baseline_reproduction_tolerance=3e-7,
        mastering=mastering,baseline_final_gain_db=previous_gain,final_gain_difference_db=mastering['final_gain_db']-previous_gain,
        explicit_original_blend=base['explicit_dry_blend'],baseline_model=base['model'],new_model_inference=False,
        new_compression_passes=1,compression_input_stage='original_derived_precompression_levelled',
        accepted_for_listening=accepted,watchdog=defects,watchdog_against_baseline=base_defects,fallback=False,
        metrics=metrics,metrics_against_baseline=vs_base,baseline_metrics=before,source_metrics=source_metrics,paths=paths,
        code_sha256={p.name:qa.sha(p) for p in (Path(__file__),Path(qa.__file__),Path(studio.__file__))},
        extra_response_peak_db=response['extra_peak_gain_db'],nominal_eq_sum_peak_db=response['nominal_sum_peak_gain_db'],
        native_application_changed=False,note='One fixed tonal calibration probe; no inferred Adobe EQ, universal preset or human approval.')
    (folder/'trial.qa.json').write_text(json.dumps(report,indent=2,allow_nan=False))
    print(json.dumps({k:report[k] for k in ('output','accepted_for_listening','watchdog','watchdog_against_baseline','baseline_compressor_reproduction_max_abs_error','mastering')},indent=2),flush=True)
    return report


if __name__=='__main__':
    import argparse
    p=argparse.ArgumentParser();p.add_argument('--baseline-report',required=True);p.add_argument('--outdir',required=True)
    a=p.parse_args();render(a.baseline_report,a.outdir)
