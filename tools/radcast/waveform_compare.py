"""Aligned waveform/room-tail inspection. Measures proxies, never infers Adobe's algorithm."""
import json
import argparse
from pathlib import Path
import numpy as np
from scipy import signal
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
import analysis as qa

def frame_rms(x,sr,ms=5):
    size=round(sr*ms/1000);n=len(x)//size
    return np.sqrt(np.mean(x[:n*size].reshape(n,size).astype('float64')**2,axis=1))

def compare(report_path,outdir,studio_key="studio_v1"):
    files=json.loads(Path(report_path).read_text())['files'];root=Path(outdir);root.mkdir(parents=True,exist_ok=True)
    sr=48000;start=10;duration=30
    audio={};meta={}
    for name in ('original',studio_key,'reference'):
        r=files[name];x,rate,_=qa.decode(r['path'])
        offset=start if name=='original' else r['matched_excerpt']['output_start_seconds']
        audio[name]=x[round(offset*rate):round((offset+duration)*rate)]
        meta[name]=dict(path=r['path'],source_start_seconds=offset,sha256=qa.sha(r['path']))
    source=audio['original'];mask,_=qa.masks(source,sr)
    # Gain match using identical original speech frames, not each recording's noise floor.
    def speech_level(x):
        f=qa.frames(x,sr);return np.sqrt(np.mean(f[:len(mask)][mask]**2))
    target=speech_level(source)
    gains={name:target/speech_level(x) for name,x in audio.items()}
    matched={name:x*gains[name] for name,x in audio.items()}
    filt=signal.butter(4,[300,4000],btype='bandpass',fs=sr,output='sos')
    band={name:signal.sosfilt(filt,x) for name,x in matched.items()}
    rms={name:frame_rms(x,sr) for name,x in band.items()}
    r=rms['original'];anchors=[]
    # Abrupt falling speech-band edge followed by a low-energy gap. Reject
    # isolated consonant dips and avoid selecting events from processed files.
    threshold=np.percentile(r,70)
    for i in range(15,len(r)-110):
        before=np.sqrt(np.mean(r[i-10:i]**2))
        after=np.sqrt(np.mean(r[i+8:i+24]**2))
        later=np.sqrt(np.mean(r[i+24:i+60]**2))
        if before>threshold and after<before*.35 and later<before*.35:
            if not anchors or i-anchors[-1]>100:anchors.append(i)
    stats={};events=[]
    for name,rr in rms.items():
        early=[];late=[]
        for i in anchors:
            direct=np.sqrt(np.mean(rr[i-10:i]**2))
            early.append(qa.db(np.sqrt(np.mean(rr[i+8:i+24]**2))/direct))
            late.append(qa.db(np.sqrt(np.mean(rr[i+24:i+60]**2))/direct))
        stats[name]=dict(gain_match_db=qa.db(gains[name]),
                        median_40_120ms_gap_to_preceding_speech_db=float(np.median(early)) if early else None,
                        median_120_300ms_gap_to_preceding_speech_db=float(np.median(late)) if late else None,
                        events_40_120ms_db=early,events_120_300ms_db=late)
    for j,i in enumerate(anchors):
        events.append(dict(original_time_seconds=start+i*.005,ratios={name:dict(early_db=stats[name]['events_40_120ms_db'][j],late_db=stats[name]['events_120_300ms_db'][j]) for name in stats}))
    result=dict(method='30 s matched excerpt; identical source-defined speech mask; speech-RMS gain matched; 300–4000 Hz causal bandpass; 5 ms RMS; 40–120/120–300 ms gap energy relative to preceding 50 ms. Gap-energy proxies include breath/phonetic detail, not isolated reverb or measured RT60.',
                event_count=len(anchors),statistics=stats,events=events,inputs=meta,
                spectral_band_share_differences_vs_original={name:{k:files[name]['matched_excerpt']['output']['bands_db_relative'][k]-files[name]['matched_excerpt']['source']['bands_db_relative'][k] for k in qa.BANDS} for name in (studio_key,'reference')})
    (root/'waveform-inspection.json').write_text(json.dumps(result,indent=2))
    colors={'original':'#777777',studio_key:'#326bcb','reference':'#39834d'};labels={'original':'Original',studio_key:('Studio r2' if 'r2' in studio_key else 'Studio v1'),'reference':'Adobe reference'}
    fig,axes=plt.subplots(3,1,figsize=(12,8),layout='constrained',sharex=True)
    for ax,(name,x) in zip(axes,matched.items()):
        # Draw 2 ms min/max waveform envelopes to retain peaks rather than decimate.
        size=96;n=len(x)//size;blocks=x[:n*size].reshape(n,size);t=start+np.arange(n)*size/sr
        ax.fill_between(t,blocks.min(axis=1),blocks.max(axis=1),color=colors[name],alpha=.8)
        for i in anchors:ax.axvline(start+i*.005,color='#d66',alpha=.5,lw=.7)
        ax.set(ylabel=labels[name],ylim=(-1,1));ax.grid(alpha=.15)
    axes[0].set_title('Same speech, matched active-speech RMS; red lines mark energy falls, not verified phrase endings')
    axes[-1].set_xlabel('Original timeline (seconds)');fig.savefig(root/'waveforms-aligned.png',dpi=160);plt.close(fig)
    fig,axes=plt.subplots(3,1,figsize=(12,9),layout='constrained',sharex=True,sharey=True)
    for ax,(name,x) in zip(axes,matched.items()):
        f,t,z=signal.stft(x,fs=sr,nperseg=1024,noverlap=896,boundary=None)
        db=20*np.log10(np.maximum(abs(z),1e-8))
        image=ax.pcolormesh(start+t,f/1000,db,shading='auto',vmin=-85,vmax=-25,cmap='magma')
        ax.set(ylim=(.1,10),ylabel=labels[name]+'\nFrequency (kHz)')
    axes[0].set_title('Identical spectrogram scale and speech-RMS matching: word structure and high-frequency detail')
    axes[-1].set_xlabel('Original timeline (seconds)');fig.colorbar(image,ax=axes,label='STFT magnitude (dBFS)');fig.savefig(root/'spectrograms-aligned.png',dpi=160);plt.close(fig)
    if anchors:
        # Choose the event with largest early-gap reduction in reference relative
        # to Studio; show all measurements in JSON to expose selection bias.
        j=int(np.argmax(np.array(stats[studio_key]['events_40_120ms_db'])-np.array(stats['reference']['events_40_120ms_db'])))
        i=anchors[j];at=i*.005;lo=max(0,at-.3);hi=min(duration,at+.6)
        fig,axes=plt.subplots(2,1,figsize=(11,7),layout='constrained')
        for name,x in matched.items():
            a=round(lo*sr);b=round(hi*sr);size=96;n=(b-a)//size;blocks=x[a:a+n*size].reshape(n,size);t=start+lo+np.arange(n)*size/sr
            axes[0].plot(t,blocks.max(axis=1),color=colors[name],label=labels[name],alpha=.8)
            rr=rms[name];tt=start+np.arange(len(rr))*.005;keep=(tt>=start+lo)&(tt<=start+hi)
            # Normalize to each recording's own preceding speech level.
            direct=np.sqrt(np.mean(rr[i-10:i]**2))
            axes[1].plot(tt[keep],20*np.log10(np.maximum(rr[keep]/direct,1e-6)),color=colors[name],label=labels[name])
        for ax in axes:ax.axvline(start+at,color='#d66',ls='--');ax.grid(alpha=.2);ax.legend()
        axes[0].set(ylabel='Positive waveform peak, 2 ms blocks',title=f'Speech edge at {start+at:.3f} s: quiet word boundary, not an isolated room-decay measurement')
        axes[1].set(ylabel='300–4000 Hz envelope relative to preceding speech (dB)',xlabel='Original timeline (seconds)',ylim=(-65,8))
        fig.savefig(root/'speech-edge-zoom.png',dpi=160);plt.close(fig)
    print(json.dumps(dict(events=result['event_count'],statistics=stats,band_differences=result['spectral_band_share_differences_vs_original']),indent=2))

if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--report',required=True);p.add_argument('--out',required=True);p.add_argument('--studio-key',default='studio_v1');args=p.parse_args();compare(args.report,args.out,args.studio_key)
