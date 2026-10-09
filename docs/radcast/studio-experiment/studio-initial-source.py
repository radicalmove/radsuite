"""Opt-in studio_v1: lossless, source-preserving local speech cleanup.
Requires cached DeepFilterNet3; never downloads models. QA failure uses transparent
source cleanup, and reports the fallback. No generative synthesis or fixed low-pass.
"""
import argparse
import importlib.metadata
import json
import os
import time
from pathlib import Path
import tempfile
import shutil
import struct
from contextlib import contextmanager
import numpy as np
from scipy import signal
import soundfile as sf
import analysis as qa

WORKING_RATE=48000
EXPECTED_MODEL_HASHES={
    'checkpoints/model_120.ckpt.best':'23b92884f63ccf54bb026014604625ab231657b6480df65db4095c4c171e6003',
    'config.ini':'415eb925d44990d938fb739f514aa3662c1ec0ea836cff044fa1291b82cb4290'}
STAGES=['decode_float_48k','highpass_45hz','deepfilternet3_delay_compensated',
        'upper_band_source_preservation','guarded_pause_attenuation',
        'light_compression','final_linear_loudness_true_peak']
CONFIG=dict(preset_id='studio_v1',highpass_hz=45,attenuation_limit_db=18,
            dry_low_band=.126,dry_high_band=.75,blend_transition_hz=[3500,7000],
            pause_max_attenuation_db=12,compression_ratio=1.15,compression_threshold=.18,
            target_lufs=-20.75,true_peak_target_dbfs=-1.5,eq_boost_db=0,deess_max_db=0)

def canonicalize_wav(path):
    # libsndfile adds a PEAK creation timestamp to float WAVs. Zero this metadata
    # in generated files so identical PCM produces reproducible artifact hashes.
    with open(path,'r+b') as f:
        header=f.read(12)
        if header[:4]!=b'RIFF' or header[8:]!=b'WAVE':return
        while True:
            chunk=f.read(8)
            if len(chunk)!=8:break
            name,size=struct.unpack('<4sI',chunk);start=f.tell()
            if name==b'PEAK' and size>=8:
                f.seek(start+4);f.write(struct.pack('<I',0));break
            f.seek(start+size+(size%2))

def write_float(path,x,sr):
    sf.write(path,x,sr,subtype='FLOAT')
    canonicalize_wav(path)


def transparent_cleanup(x,sr):
    return signal.sosfilt(signal.butter(2,45,fs=sr,btype='highpass',output='sos'),x).astype('float32')

def preservation_blend(dry,enhanced,sr):
    if len(dry)!=len(enhanced) or not np.isfinite(enhanced).all():
        raise ValueError('model output has wrong length or nonfinite samples')
    f,_,a=signal.stft(dry,fs=sr,nperseg=1024,noverlap=768,boundary='zeros')
    _,_,b=signal.stft(enhanced,fs=sr,nperseg=1024,noverlap=768,boundary='zeros')
    weight=np.clip((f-3500)/(7000-3500),0,1)*.75
    # DF's own 18 dB attenuation limit already preserves 12.6% dry in all bands.
    _,y=signal.istft(b*(1-weight[:,None])+a*weight[:,None],fs=sr,nperseg=1024,noverlap=768)
    return y[:len(dry)].astype('float32')

def pause_control(x,source,sr,max_db=12):
    _,pause=qa.masks(source,sr)
    if not np.any(pause):return x.copy()
    size=round(sr*.02)
    gains=np.where(pause,10**(-max_db/20),1.)
    # Only guarded silence interiors; 60 ms smoothing prevents gate clicks.
    gains=signal.convolve(gains,np.ones(7)/7,mode='same')
    gains[:3]=1;gains[-3:]=1
    curve=np.interp(np.arange(len(x)),np.arange(len(gains))*size+size/2,gains,left=1,right=1)
    return (x*curve).astype('float32')

@contextmanager
def pinned_model_workspace(path):
    path=Path(path)
    for filename,expected in EXPECTED_MODEL_HASHES.items():
        artifact=path/filename
        if not artifact.is_file() or qa.sha(artifact)!=expected:
            raise ValueError(f'Unrecognized Studio v1 model/config: {filename}; use pinned DeepFilterNet3 artifacts')
    with tempfile.TemporaryDirectory(prefix='radcast-pinned-df3-') as folder:
        isolated=Path(folder);(isolated/'checkpoints').mkdir()
        for filename in EXPECTED_MODEL_HASHES:
            shutil.copyfile(path/filename,isolated/filename)
        yield isolated


def model_cleanup(x,model_dir):
    path=Path(model_dir)
    if not (path/'config.ini').is_file() or not list((path/'checkpoints').glob('*')):
        raise FileNotFoundError('Cached DeepFilterNet3 config/checkpoint missing; no automatic download')
    for package in ('deepfilternet','deepfilterlib'):
        if importlib.metadata.version(package)!='0.5.6':
            raise ValueError(f'Studio v1 requires {package}==0.5.6')
    import torch
    from df.enhance import init_df,enhance
    torch.set_num_threads(min(4,os.cpu_count() or 1));torch.manual_seed(0)
    with pinned_model_workspace(path) as isolated:
        model,state,_=init_df(str(isolated),post_filter=False,log_level='ERROR',log_file=None,epoch=120)
    if state.sr()!=WORKING_RATE:raise ValueError('Model must operate at 48 kHz')
    start=time.perf_counter()
    y=enhance(model,state,torch.from_numpy(x[None].copy()),pad=True,atten_lim_db=18)
    return y.numpy()[0],time.perf_counter()-start

def master(x,sr,folder,compression=True):
    raw=folder/'cleanup.wav';write_float(raw,x,sr)
    processed=folder/'compressed.wav'
    if compression:
        qa.run(['ffmpeg','-y','-v','error','-i',raw,'-af',
                'acompressor=threshold=0.18:ratio=1.15:attack=20:release=220:makeup=1',
                '-ar',sr,'-c:a','pcm_f32le',processed])
    else:
        processed=raw
    m=qa.loudness(processed)
    # Constant final gain avoids loudnorm dynamic pumping/resampling. Headroom
    # constraint takes precedence over exact LUFS. No waveform clipping limiter.
    gain=0 if m['integrated_lufs'] is None else CONFIG['target_lufs']-m['integrated_lufs']
    if m['true_peak_dbfs'] is not None:gain=min(gain,-1.5-m['true_peak_dbfs']-.05)
    y,_=sf.read(processed,dtype='float32')
    return (y*10**(gain/20)).astype('float32'),dict(measured_before=m,final_gain_db=gain)

def render(input_path,output_path,model_dir=None,tail=False):
    input_path=Path(input_path);output_path=Path(output_path)
    output_path.parent.mkdir(parents=True,exist_ok=True)
    # ffmpeg decode handles compressed inputs once; prepared float WAV bypasses decoding.
    info=sf.info(input_path) if input_path.suffix.lower()=='.wav' else None
    if info and info.samplerate==48000:
        source,sr=sf.read(input_path,dtype='float32',always_2d=True)
        source=source.mean(axis=1)
    else:source,sr,_=qa.decode(input_path,48000)
    dry=transparent_cleanup(source,sr)
    model_dir=Path(model_dir or os.environ.get('RADSUITE_STUDIO_V1_MODEL',
             str(Path.home()/'Library/Caches/DeepFilterNet/DeepFilterNet3' if os.sys.platform=='darwin'
                 else Path.home()/'.cache/DeepFilterNet/DeepFilterNet3')))
    warnings=[];fallback=False;elapsed=None
    try:
        y,elapsed=model_cleanup(dry,model_dir)
        y=preservation_blend(dry,y,sr)
        if tail:
            # Meaningful alternate architecture: modest late-tail attenuation.
            # Local algorithm, no imported mutable RADcast helper.
            f,_,spec=signal.stft(y,fs=sr,nperseg=1024,noverlap=768,boundary='zeros')
            power=abs(spec)**2
            late=signal.lfilter([.03],[1,-.97],np.pad(power,((0,0),(4,0)))[:,:power.shape[1]],axis=1)
            gain=np.sqrt(np.clip(1-.3*late/np.maximum(power,1e-12),.7,1))
            gain[f>=4000]=1
            _,y=signal.istft(spec*gain,fs=sr,nperseg=1024,noverlap=768)
            y=y[:len(source)].astype('float32')
    except (Exception, SystemExit) as exc:
        warnings.append(dict(code='model_unavailable',detail=f'{type(exc).__name__}: {exc}'))
        y=dry;fallback=True
    source_metrics=qa.signal_metrics(source,sr)
    with tempfile.TemporaryDirectory(prefix='radcast-studio-') as temp:
        folder=Path(temp)
        y=pause_control(y,source,sr)
        y,mastering=master(y,sr,folder)
        def evaluate(audio):
            test=folder/'qa.wav';write_float(test,audio,sr)
            out=qa.signal_metrics(audio,sr,qa.masks(source,sr));out.update(qa.loudness(test))
            out.update(qa.alignment(source,audio,sr))
            return out,qa.watchdog(source_metrics,out)
        measured,defects=evaluate(y)
        rejected=[]
        if defects and not fallback:
            rejected=defects;fallback=True
            warnings.append(dict(code='preservation_fallback',detail='Model path failed watchdog; using source cleanup',rejected=defects))
            y=pause_control(dry,source,sr)
            y,mastering=master(y,sr,folder,compression=False)
            measured,defects=evaluate(y)
        if defects:
            # Fail closed: no highly damaged/failing output delivered as Studio.
            raise ValueError(f'Studio source fallback failed QA: {defects}')
        write_float(output_path,y,sr)
    versions={p:importlib.metadata.version(p) for p in ('numpy','scipy','soundfile')}
    for p in ('deepfilternet','deepfilterlib','torch','torchaudio'):
        try:versions[p]=importlib.metadata.version(p)
        except importlib.metadata.PackageNotFoundError:pass
    result=dict(schema_version=1,config=dict(CONFIG,tail_reduction=.3 if tail else 0,tail_power_floor=.7 if tail else None),stages=STAGES[:4]+(['late_tail_suppression'] if tail else [])+STAGES[4:],
                tail_variant=tail,input=str(input_path.resolve()),output=str(output_path.resolve()),
                input_sha256=qa.sha(input_path),output_sha256=qa.sha(output_path),
                sample_rate=sr,working_format='float32 PCM',versions=versions,
                model_path=str(model_dir),model_files={str(p.relative_to(model_dir)):qa.sha(p) for p in sorted(model_dir.rglob('*')) if p.is_file() and p.suffix in ('.ini','.best','.ckpt')},
                inference_seconds=elapsed,real_time_factor=None if elapsed is None else elapsed/(len(source)/sr),
                fallback=fallback,warnings=warnings,watchdog=defects,rejected_path_watchdog=rejected,
                metrics=measured,source_metrics=source_metrics,mastering=mastering)
    output_path.with_suffix('.qa.json').write_text(json.dumps(result,indent=2,allow_nan=False))
    print(f"RADCAST_ENHANCE_PROGRESS 1/1",flush=True)
    return result

def verify_export(source_path, export_path, report_path, edited=False, origin=None, removed_seconds=0):
    report_path=Path(report_path)
    report=json.loads(report_path.read_text())
    final=qa.measure(export_path,None if edited else source_path)
    if edited:
        defects=[]
        prepared,sr,_=qa.decode(source_path)
        expected=len(prepared)/sr-removed_seconds
        if abs(final['duration_seconds']-expected)>.025:
            defects.append(dict(code='duration',detail='Final duration differs from source minus requested edits by >25 ms'))
        if final['speech_rms_dbfs'] is None or final['speech_rms_dbfs'] < -65:
            defects.append(dict(code='destructive',detail='No usable speech in final edited export'))
        final['expected_duration_seconds']=expected
        if final['clipped_samples']:defects.append(dict(code='clipping',detail='Final export clips'))
        if final['true_peak_dbfs'] is not None and final['true_peak_dbfs']>-1.3:defects.append(dict(code='true_peak',detail='Final export exceeds true-peak target tolerance'))
        final['comparison_note']='User-requested edits change timing; preservation checked on enhancement master before editing.'
    else:
        if not final.get('timing_comparable'):
            raise ValueError('Final export has unexpected timing change')
        defects=final['watchdog']
    if defects:raise ValueError(f'Final export failed watchdog: {defects}')
    report['enhancement_master_metrics']=report.get('metrics')
    report['prepared_clip']=dict(path=str(Path(source_path).resolve()),sha256=qa.sha(source_path),role='Temporary decoded/trimmed float PCM input; may be removed after export')
    report['metrics']=final
    report['final_export_edited']=edited
    report['output']=str(Path(export_path).resolve())
    report['output_sha256']=final['sha256']
    if origin:
        report['original_source']=str(Path(origin).resolve())
        report['original_source_sha256']=qa.sha(origin)
    report_path.write_text(json.dumps(report,indent=2,allow_nan=False))
    return report


def main():
    p=argparse.ArgumentParser();p.add_argument('input',nargs='?');p.add_argument('output',nargs='?');p.add_argument('--model-dir');p.add_argument('--tail',action='store_true');p.add_argument('--verify-export',nargs=3);p.add_argument('--edited',action='store_true');p.add_argument('--origin');p.add_argument('--removed-seconds',type=float,default=0)
    args=p.parse_args()
    if args.verify_export:
        r=verify_export(*args.verify_export,edited=args.edited,origin=args.origin,removed_seconds=args.removed_seconds)
    else:
        if not args.input or not args.output:p.error('input and output are required')
        r=render(args.input,args.output,args.model_dir,args.tail)
    print(json.dumps(dict(fallback=r['fallback'],warnings=r['warnings'],metrics=r['metrics'])))

if __name__=='__main__':main()
