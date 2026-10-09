"""Evidence-only analysis of saved controls; never imports/executes a DF model."""
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import numpy as np
import soundfile as sf
from scipy import signal

ROOT=Path(__file__).resolve().parent
REPO=ROOT.parents[5]
sys.path.insert(0,str(REPO/'tools/radcast'))
import analysis as qa
import studio

def windows(reference,output,clean,speech,sr):
    """Existing voiced-window test with eligibility fixed to known-clean audio."""
    step=max(1,sr//6000);rate=sr/step
    filtered=signal.sosfilt(signal.butter(3,[70,1000],fs=sr,btype='bandpass',output='sos'),clean)[::step]
    hop=round(rate*.02);size=round(rate*.03)
    rc=qa.frames(clean,sr);rr=qa.frames(reference,sr);ro=qa.frames(output,sr)
    n=min(len(rc),len(rr),len(ro),len(speech))
    filtered=np.pad(filtered,(0,size))
    blocks=np.lib.stride_tricks.sliding_window_view(filtered,size)[np.arange(n)*hop]
    spec=np.fft.rfft(blocks*np.hanning(size),n=2**int(np.ceil(np.log2(size*2))))
    ac=np.fft.irfft(abs(spec)**2,axis=1)
    lo=max(1,round(rate/400));hi=round(rate/70)
    confidence=ac[:,lo:hi].max(axis=1)/np.maximum(ac[:,0],1e-20)
    voiced=(confidence>.45)&(rc[:n]>1e-5)&speech[:n]
    gains=[];starts=[]
    for start in range(0,n-49,25):
        end=start+50;mask=voiced[start:end]
        if mask.sum()<5:continue
        if np.percentile(rc[start:end],90)<np.percentile(rc[start:end],10)*1.4:continue
        before=np.sqrt(np.mean(rr[start:end][mask]**2))
        after=np.sqrt(np.mean(ro[start:end][mask]**2))
        gains.append(qa.db(after/max(before,1e-20)));starts.append(start*.02)
    median=float(np.median(gains)) if gains else None
    return dict(speech_window_gain_db=gains,speech_window_starts_seconds=starts,
        speech_window_gain_median_db=median,
        worst_speech_window_gain_relative_db=None if median is None else float(min(gains)-median),
        speech_window_method='Existing 1 s / .5 s voiced-modulation proxy, with eligibility fixed to known-clean speech mask and periodicity/modulation; relative to median')

def aligned_metrics(reference,output,clean,speech,sr):
    result=qa.alignment(reference,output,sr)
    lag=round(result['lag_seconds']*sr)
    rr=reference[max(0,-lag):];oo=output[max(0,lag):]
    cc=clean[max(0,-lag):]
    n=min(len(rr),len(oo),len(cc))
    # All current controls have measured zero lag; fail instead of silently
    # shifting clean frame eligibility if that invariant is violated.
    if lag:raise ValueError('Nonzero control lag: inspect matching masks before proceeding')
    result.update(windows(rr[:n],oo[:n],cc[:n],speech,sr))
    return result

def words(text):return re.findall(r"[a-z]+(?:'[a-z]+)?",text.lower())

def edits(reference,hypothesis):
    a=words(reference);b=words(hypothesis)
    dp=[[0]*(len(b)+1) for _ in range(len(a)+1)]
    for i in range(len(a)+1):dp[i][0]=i
    for j in range(len(b)+1):dp[0][j]=j
    for i in range(1,len(a)+1):
        for j in range(1,len(b)+1):
            dp[i][j]=min(dp[i-1][j]+1,dp[i][j-1]+1,dp[i-1][j-1]+(a[i-1]!=b[j-1]))
    operations=[];i=len(a);j=len(b)
    while i or j:
        if i and j and dp[i][j]==dp[i-1][j-1]+(a[i-1]!=b[j-1]):
            if a[i-1]!=b[j-1]:operations.append(dict(kind='substitution',reference=a[i-1],hypothesis=b[j-1],reference_word_index=i-1))
            i-=1;j-=1
        elif i and dp[i][j]==dp[i-1][j]+1:
            operations.append(dict(kind='deletion',reference=a[i-1],reference_word_index=i-1));i-=1
        else:
            operations.append(dict(kind='insertion',hypothesis=b[j-1],reference_word_index=i));j-=1
    operations.reverse()
    return dict(reference_words=len(a),hypothesis_words=len(b),edit_distance=dp[-1][-1],
        word_error_rate=dp[-1][-1]/max(1,len(a)),operations=operations,
        normalization='Lowercase alphabetic words; apostrophes retained; punctuation ignored; no number/contraction equivalence expansion')

def main():
    source=ROOT.parent/'controls/controls.json'
    original=json.loads(source.read_text())
    result=dict(schema_version=1,source_controls=str(source),source_controls_sha256=qa.sha(source),
        method='Saved original PCM only; all spectra/levels use the identical known-clean source-energy masks',
        no_model_execution=True,speakers={})
    originals={}
    asrdir=ROOT/'asr';asrdir.mkdir()
    matchdir=ROOT/'matched';matchdir.mkdir()
    model=Path('/Users/rcd58/.radcast/whispercpp-models/ggml-small.bin')
    whisper=Path('/opt/homebrew/bin/whisper-cli')
    assert qa.sha(model)=='1be3a9b2063867b937e64e2ec7483364a79917e157fa98c5d94b5c1fffea987b'
    result['asr']=dict(model=str(model),model_sha256=qa.sha(model),binary=str(whisper),binary_sha256=qa.sha(whisper),
        settings=['-l','en','-t','4','-nf','-nt','-otxt'],analysis_input_rate=16000,
        reference='Official selected-utterance transcripts, concatenated in the recorded source-boundary order',
        limitation='Text WER cannot establish absence of lisp, warbling, breath loss or preserved speaker identity')
    for speaker,record in original['speakers'].items():
        audio={};metrics={};sr=48000
        for name,info in record['paths'].items():
            assert qa.sha(info['path'])==info['sha256']
            originals[info['path']]=info['sha256']
            x,rate=sf.read(info['path'],dtype='float32');assert rate==sr and x.ndim==1
            audio[name]=x
        assert len({len(x) for x in audio.values()})==1
        clean=audio['clean'];mask=qa.masks(clean,sr)
        for name,x in audio.items():
            m=qa.signal_metrics(x,sr,mask);m.update(qa.loudness(record['paths'][name]['path']))
            m.update(path=record['paths'][name]['path'],sha256=record['paths'][name]['sha256'],native_rate=sr,sample_count=len(x))
            metrics[name]=m
        comparisons={}
        for label,refname,outname in [('room_input_vs_clean','clean','synthetic_room'),
                ('clean_model_vs_clean','clean','clean_model'),
                ('room_model_vs_clean','clean','synthetic_room_model'),
                ('room_model_vs_room_input','synthetic_room','synthetic_room_model')]:
            out=dict(metrics[outname]);out.update(aligned_metrics(audio[refname],audio[outname],clean,mask[0],sr))
            comparisons[label]=dict(reference=refname,output=outname,alignment_and_local_retention=out,
                watchdog=qa.watchdog(metrics[refname],out))
        levels={name:10**(m['speech_rms_dbfs']/20) for name,m in metrics.items()}
        target=min(levels.values());gains={name:min(1,target/max(level,1e-20)) for name,level in levels.items()}
        peak=max(float(np.max(abs(audio[name]*gain))) for name,gain in gains.items())
        common=min(1,10**(-3/20)/max(peak,1e-20))
        matched={}
        for name,x in audio.items():
            path=matchdir/f'{speaker}_{name}_matched.wav';y=(x*gains[name]*common).astype('float32')
            studio.write_float(path,y,sr)
            matched[name]=dict(path=str(path),sha256=qa.sha(path),gain_db=qa.db(gains[name]),
                common_headroom_gain_db=qa.db(common),duration_seconds=len(y)/sr,
                speech_rms_dbfs=qa.signal_metrics(y,sr,mask)['speech_rms_dbfs'])
        reference=' '.join(b['transcript'] for b in record['source_boundaries'])
        asr={}
        for name,info in record['paths'].items():
            wav=asrdir/f'{speaker}_{name}_analysis16k.wav';prefix=asrdir/f'{speaker}_{name}'
            qa.run(['ffmpeg','-y','-v','error','-i',info['path'],'-ac','1','-ar','16000','-c:a','pcm_s16le',wav])
            command=[str(whisper),'-m',str(model),'-f',str(wav),'-l','en','-t','4','-nf','-nt','-otxt','-of',str(prefix)]
            completed=subprocess.run(command,capture_output=True,check=True)
            prefix.with_suffix('.stderr.log').write_bytes(completed.stderr)
            text=prefix.with_suffix('.txt').read_text()
            asr[name]=dict(transcript=text,**edits(reference,text))
            print(speaker,name,'WER',asr[name]['word_error_rate'],flush=True)
        result['speakers'][speaker]=dict(source_boundaries=record['source_boundaries'],official_reference_text=reference,
            raw_metrics=metrics,comparisons=comparisons,matched_clips=matched,asr=asr,
            warning_interpretation='Clean-reference violations may be residual synthetic room or model coloration; room-input violations indicate changes but desired room removal also changes spectra/envelopes. Compare warnings already present in the untreated room input; none alone proves an introduced defect or audible improvement.')
    for path,digest in originals.items():assert qa.sha(path)==digest
    result['all_original_control_hashes_reverified_unchanged']=True
    result['validation_script_sha256']=qa.sha(__file__)
    result['analysis_source_sha256']=qa.sha(REPO/'tools/radcast/analysis.py')
    result['acceptance_limitation']='These controls may support one Finnegan feasibility render, never human listening success or promotion. No warning thresholds were changed.'
    (ROOT.parent/'controls-extended-qa.json').write_text(json.dumps(result,indent=2,allow_nan=False)+'\n')
    print('Completed saved-control validation',flush=True)

if __name__=='__main__':main()
