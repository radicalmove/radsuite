"""Deterministic RADcast audio measurements. Energy masks are proxies, not VAD.
Native-rate analysis; any comparison resampling is analysis-only. No input writes.
"""
import argparse
import hashlib
import json
import os
import subprocess
from pathlib import Path
import numpy as np
from scipy import signal

BANDS = {'sub': (20, 80), 'body': (80, 350), 'mid': (350, 1500),
         'presence': (1500, 4000), 'sibilants': (4000, 8000), 'air': (8000, 16000)}
THRESHOLDS = dict(duration_seconds=.025, lag_seconds=.015, block_lag_spread_seconds=.025,
                  true_peak_dbfs=-1.3, pause_increase_db=3, envelope_correlation=.9,
                  bass_increase_db=3, sibilant_loss_db=6, air_loss_db=10, spectral_change_db=6, minimum_spectral_share_db=-50)

def run(args):
    args = [str(v) for v in args]
    if args[0] in ('ffmpeg','ffprobe'):
        args[0] = os.environ.get('RADSUITE_STUDIO_' + args[0].upper(), args[0])
    return subprocess.run(args, check=True, capture_output=True)

def db(value):
    return float(20 * np.log10(max(float(value), 1e-12)))

def sha(path):
    h = hashlib.sha256()
    with open(path, 'rb') as f:
        for block in iter(lambda: f.read(1024*1024), b''): h.update(block)
    return h.hexdigest()

def decode(path, sr=None):
    meta = json.loads(run(['ffprobe','-v','error','-show_streams','-show_format','-of','json',path]).stdout)
    stream = next(v for v in meta['streams'] if v['codec_type'] == 'audio')
    rate = int(stream['sample_rate']) if sr is None else sr
    raw = run(['ffmpeg','-v','error','-i',path,'-map','0:a:0','-ac','1','-ar',rate,
               '-f','f32le','-c:a','pcm_f32le','pipe:1']).stdout
    return np.frombuffer(raw, dtype='<f4').copy(), rate, meta

def loudness(path):
    result = run(['ffmpeg','-hide_banner','-nostats','-i',path,'-af',
                  'loudnorm=I=-20.75:TP=-1.5:LRA=8:print_format=json','-f','null','-'])
    s = result.stderr.decode()
    obj = json.loads(s[s.rfind('{'):s.rfind('}')+1])
    def number(v):
        n = float(v)
        return n if np.isfinite(n) else None
    return dict(integrated_lufs=number(obj['input_i']), true_peak_dbfs=number(obj['input_tp']),
                loudness_range_lu=number(obj['input_lra']))

def frames(x, sr):
    size = max(1, round(sr*.02))
    n = len(x)//size
    return np.sqrt(np.mean(x[:n*size].reshape(n,size).astype(np.float64)**2, axis=1))

def masks(x, sr):
    r = frames(x, sr)
    if not len(r): return np.array([],bool), np.array([],bool)
    # Keep uncertain transition/breath frames out of the pause mask.
    threshold = max(min(np.percentile(r,20)*2.5, np.percentile(r,85)*.5), np.percentile(r,85)*.12, 1e-6)
    speech = r > threshold
    pause = signal.convolve((r < threshold*.6).astype(int), np.ones(9), mode='same') >= 9
    return speech, pause

def signal_metrics(x, sr, source_masks=None):
    if not len(x) or not np.isfinite(x).all(): raise ValueError('empty/nonfinite audio')
    x = np.asarray(x,dtype=np.float64)
    rms = frames(x,sr)
    speech,pause = masks(x,sr) if source_masks is None else source_masks
    n = min(len(rms),len(speech),len(pause))
    def level(mask):
        return db(np.sqrt(np.mean(rms[:n][mask[:n]]**2))) if np.any(mask[:n]) else None
    sp,pa = level(speech),level(pause)
    size = round(sr*.02)
    # Spectrum averaged only across speech frames, excluding room/noise pauses.
    blocks = x[:n*size].reshape(n,size)[speech[:n]]
    if not len(blocks): blocks = x[:n*size].reshape(n,size)
    fft_size = max(2048, 2**int(np.ceil(np.log2(size))))
    p = np.mean(abs(np.fft.rfft(blocks * np.hanning(size), n=fft_size))**2, axis=0)
    f = np.fft.rfftfreq(fft_size,1/sr)
    total = max(p.sum(),1e-24)
    band = {k: float(10*np.log10(max(p[(f>=lo)&(f<hi)].sum()/total,1e-24)))
            for k,(lo,hi) in BANDS.items()}
    roll = float(f[min(np.searchsorted(np.cumsum(p),total*.95), len(f)-1)])
    return dict(pcm_sha256=hashlib.sha256(np.asarray(x,dtype='<f4').tobytes()).hexdigest(), duration_seconds=len(x)/sr, rms_dbfs=db(np.sqrt(np.mean(x*x))),
                sample_peak_dbfs=db(np.max(abs(x))), clipped_samples=int(np.sum(abs(x)>=1)),
                speech_rms_dbfs=sp, pause_rms_dbfs=pa,
                speech_pause_contrast_db=None if sp is None or pa is None else sp-pa,
                speech_frames=int(speech[:n].sum()), pause_frames=int(pause[:n].sum()),
                mask_method='20 ms source-energy proxy; 180 ms guarded pauses; not semantic VAD',
                bands_db_relative=band, rolloff_95_hz=roll,
                hf_bands_db_relative={f'{lo}-{hi}':float(10*np.log10(max(p[(f>=lo)&(f<hi)].sum()/total,1e-24))) for lo,hi in [(6000,7000),(7000,8000),(8000,9000),(9000,10000),(10000,12000),(12000,16000)]})

def _lag(x,y,sr,maxlag=10):
    # 1 kHz low-passed analysis copy to keep long-file cross correlation bounded.
    step = max(1,sr//1000)
    xa = signal.resample_poly(x,1,step)
    ya = signal.resample_poly(y,1,step)
    xa -= xa.mean(); ya -= ya.mean()
    corr = signal.correlate(ya,xa,mode='full',method='fft')
    lags = signal.correlation_lags(len(ya),len(xa))
    keep = abs(lags)<=maxlag*sr/step
    lag = int(lags[keep][np.argmax(corr[keep])]) * step
    return lag

def alignment(x,y,sr):
    lag = _lag(x,y,sr)
    xa = x[max(0,-lag):]; ya = y[max(0,lag):]
    n = min(len(xa),len(ya)); xa=xa[:n];ya=ya[:n]
    def correlation(a,b):
        return float(np.corrcoef(a,b)[0,1]) if np.std(a)>1e-10 and np.std(b)>1e-10 else None
    block = sr*20
    lags = [_lag(x[i:i+block],y[i:i+block],sr,10)/sr for i in range(0,min(len(x),len(y))-sr,block)]
    return dict(lag_seconds=lag/sr, waveform_correlation=correlation(xa,ya),
                envelope_correlation=correlation(frames(xa,sr),frames(ya,sr)),
                block_lags_seconds=lags,
                block_lag_spread_seconds=float(np.ptp(lags)) if lags else 0.)

def watchdog(source,out):
    warnings=[]
    def add(code,detail): warnings.append(dict(code=code,detail=detail))
    if abs(out['duration_seconds']-source['duration_seconds'])>THRESHOLDS['duration_seconds']: add('duration','Duration changed by >25 ms')
    if abs(out.get('lag_seconds') or 0)>THRESHOLDS['lag_seconds']: add('timing','Aligned lag exceeds 15 ms')
    if (out.get('block_lag_spread_seconds') or 0)>THRESHOLDS['block_lag_spread_seconds']: add('timing_drift','Block lag varies by >25 ms')
    if out.get('clipped_samples',0): add('clipping','Samples at or above full scale')
    if out.get('true_peak_dbfs') is not None and out['true_peak_dbfs']>THRESHOLDS['true_peak_dbfs']: add('true_peak','True peak exceeds -1.3 dBTP tolerance for -1.5 target')
    if out.get('envelope_correlation') is not None and out['envelope_correlation']<THRESHOLDS['envelope_correlation']: add('envelope','Speech envelope correlation below 0.90')
    if source.get('pause_rms_dbfs') is not None and out.get('pause_rms_dbfs') is not None and out['pause_rms_dbfs']-source['pause_rms_dbfs']>3: add('pause_noise','Pause level increased >3 dB')
    a=source['bands_db_relative']; b=out['bands_db_relative']
    if (b['body']-a['body']>3 and b['body']>-50) or (b['sub']-a['sub']>3 and b['sub']>-50): add('bass','Low-frequency energy share increased >3 dB')
    if (a['sibilants']>-50 and b['sibilants']-a['sibilants'] < -6) or (a['air']>-50 and b['air']-a['air'] < -10): add('hf_collapse','Sibilant share fell >6 dB or air share >10 dB')
    for k in ('body','presence','sibilants'):
        if max(a[k],b[k])>-50 and abs(b[k]-a[k])>6: add('spectral_balance',f'{k} energy share changed >6 dB')
    if out.get('speech_rms_dbfs') is None or out['speech_rms_dbfs'] < -65: add('destructive','No usable speech energy')
    return warnings

def measure(path, original=None):
    x,sr,meta = decode(path)
    stream = next(v for v in meta['streams'] if v['codec_type']=='audio')
    result = dict(path=str(Path(path).resolve()),sha256=sha(path),sample_rate=sr,
                  channels=int(stream['channels']),codec=stream['codec_name'],
                  bitrate=int(stream.get('bit_rate',meta['format'].get('bit_rate',0))))
    result.update(signal_metrics(x,sr)); result.update(loudness(path))
    if original is not None:
        orig,orig_sr,_ = decode(original)
        if orig_sr!=sr: orig=signal.resample_poly(orig,sr,orig_sr)
        align = alignment(orig,x,sr);result.update(align)
        comparable = abs(len(orig)-len(x))/sr<.1 and align['block_lag_spread_seconds']<.05
        result['timing_comparable']=bool(comparable)
        # Matched 30 s excerpt before observed later edits. Not a whole-file
        # preservation claim; refine local lag and use identical source masks.
        start=10; span=30
        local_lag=align['block_lags_seconds'][0] if align['block_lags_seconds'] else align['lag_seconds']
        target_start=round((start+local_lag)*sr)
        if target_start>=0 and len(x)>=target_start+span*sr and len(orig)>=(start+span)*sr:
            excerpt_source=orig[start*sr:(start+span)*sr]
            excerpt_output=x[target_start:target_start+span*sr]
            local=alignment(excerpt_source,excerpt_output,sr)
            result['matched_excerpt']=dict(source_start_seconds=start,output_start_seconds=target_start/sr,duration_seconds=span,
                alignment=local,source=signal_metrics(excerpt_source,sr),
                output=signal_metrics(excerpt_output,sr,masks(excerpt_source,sr)),
                note='First stable-offset segment only; energy masks do not establish consonant or speaker identity preservation.')
        if comparable:
            lag=round(align['lag_seconds']*sr)
            xx=x[max(0,lag):];oo=orig[max(0,-lag):]
            result.update(signal_metrics(xx[:min(len(xx),len(oo))],sr,masks(oo[:min(len(xx),len(oo))],sr)))
            # Report actual native duration, not overlap length.
            result['duration_seconds']=len(x)/sr
            base=signal_metrics(orig,sr)
            result['watchdog']=watchdog(base,result)
        else:
            result['watchdog']=None
            result['comparison_note']='Edited/different timing: pause metrics use own mask; global correlations and spectrum cannot establish preservation.'
    return result

def report(results):
    lines=['RADcast calibration measurements','', 'All levels dB; speech/pause masks are energy proxies, not ASR/VAD.',
           'Spectral bands are speech-energy shares, not absolute EQ gain. Edited files are not timing-comparable.','',
           '| File | Hz | Duration | LUFS | dBTP | Speech/pause contrast | Rolloff95 |',
           '|---|---:|---:|---:|---:|---:|---:|']
    for label,r in results.items():
        def fmt(v): return 'n/a' if v is None else f'{v:.2f}'
        lines.append(f"| {label} | {r['sample_rate']} | {fmt(r['duration_seconds'])} | {fmt(r['integrated_lufs'])} | {fmt(r['true_peak_dbfs'])} | {fmt(r['speech_pause_contrast_db'])} | {fmt(r['rolloff_95_hz'])} |")
    for label,r in results.items():
        lines += ['',label, f"Path: {r['path']}",f"SHA256: {r['sha256']}",
                  f"Bands (dB of total speech spectrum): {r['bands_db_relative']}",
                  f"Alignment: lag={r.get('lag_seconds')}, block spread={r.get('block_lag_spread_seconds')}, envelope={r.get('envelope_correlation')}",
                  f"QA: {r.get('watchdog')}",r.get('comparison_note','')]
    return '\n'.join(lines)+'\n'

def main():
    p=argparse.ArgumentParser();p.add_argument('--manifest',required=True);p.add_argument('--out',required=True)
    args=p.parse_args();paths=json.loads(Path(args.manifest).read_text())
    original=paths['original'];results={k:measure(v,None if k=='original' else original) for k,v in paths.items()}
    dest=Path(args.out);dest.parent.mkdir(parents=True,exist_ok=True)
    dest.with_suffix('.json').write_text(json.dumps(dict(schema_version=1,thresholds=THRESHOLDS,files=results),indent=2,allow_nan=False))
    dest.with_suffix('.txt').write_text(report(results))
    print(report(results).split('\n\noriginal')[0])

if __name__=='__main__': main()
