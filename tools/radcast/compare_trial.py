"""Controlled r3 diagnostics and level-matched excerpts from full PCM masters."""
import argparse
import json
from pathlib import Path
import numpy as np
from scipy import signal
import soundfile as sf
import analysis as qa
import studio

def rms(x):return float(np.sqrt(np.mean(np.asarray(x,dtype='float64')**2)))

def write_evidence(path,x,sr):
    path=Path(path)
    if path.exists():
        existing,rate=sf.read(path,dtype='float32')
        if rate!=sr or not np.array_equal(existing,np.asarray(x,dtype='float32')):
            raise ValueError(f'Existing listening evidence differs: {path}; use a new trial directory')
        return
    studio.write_float(path,x,sr)

def compare(root):
    root=Path(root);manifest=json.loads((root/'manifest.json').read_text())
    bdir=root/'diagnostics-delay4';cdir=root/'diagnostics-delay2';results={}
    for name in ('baseline_r2_delay4','CRJU160_studio_r3_wpe65_delay2'):
        if json.loads((root/(name+'.qa.json')).read_text())['fallback']:
            raise ValueError('Controlled comparison requires accepted non-fallback chains; inspect attempted diagnostic stems separately')
    sr=48000;region=slice(21*sr,29*sr)
    for stage in ('wpe','cleanup','presence','levelled'):
        b,rate=sf.read(bdir/(stage+'.wav'),dtype='float32');c,_=sf.read(cdir/(stage+'.wav'),dtype='float32')
        results[stage]=dict(phrase_rms_change_db=qa.db(rms(c[region])/rms(b[region])),
            phrase_difference_signal_db_relative=qa.db(rms(c[region]-b[region])/rms(b[region])),
            phrase_waveform_correlation=float(np.corrcoef(b[region],c[region])[0,1]))
    bg=np.load(bdir/'level_frame_gain_db.npy');cg=np.load(cdir/'level_frame_gain_db.npy')
    gslice=slice(21*50,29*50)
    results['gain_curve']=dict(maximum_absolute_difference_db=float(np.max(abs(cg-bg))),
                              phrase_maximum_difference_db=float(np.max(abs(cg[gslice]-bg[gslice]))),
                              phrase_rms_difference_db=rms(cg[gslice]-bg[gslice]))
    # A literal common-gain diagnostic removes the adaptive leveling response
    # from the question about room cleanup. This is not the normal candidate.
    c,_=sf.read(cdir/'presence.wav',dtype='float32');n=len(c);size=round(sr*.02)
    curve=np.interp(np.arange(n),np.arange(len(bg))*size+size/2,bg,left=bg[0],right=bg[-1])
    frozen=(c*10**(curve/20)).astype('float32')
    write_evidence(root/'diagnostic_delay2_frozen_r2_level_gain.wav',frozen,sr)
    b,_=sf.read(bdir/'levelled.wav',dtype='float32')
    results['frozen_gain']=dict(phrase_rms_change_db=qa.db(rms(frozen[region])/rms(b[region])),
                               phrase_difference_signal_db_relative=qa.db(rms(frozen[region]-b[region])/rms(b[region])))
    results['baseline_reproduced']=qa.sha(root/'baseline_r2_delay4.wav')==manifest['studio_r2_wpe65']['sha256']
    results['note']='Signal difference is not isolated reverb, RT60 or quality. Frozen-gain file is a diagnostic control, not a promoted preset.'
    (root/'controlled-diagnostics.json').write_text(json.dumps(results,indent=2))
    # One identical original-derived mask per region; all gains are static for
    # audition only. Use the quietest speech RMS, so no excerpt is boosted.
    source,sr,_=qa.decode(manifest['original']['path']);source_frames=qa.frames(source,sr)
    clip_groups={'target_phrase':(20.,32.),'quiet_articulation':(19.5,22.5),'louder_passage':(70.,80.),'phrase_ending_context':(38.5,41.8)}
    masters={'r2':(root/'baseline_r2_delay4.wav',0.),'r3_delay2':(root/'CRJU160_studio_r3_wpe65_delay2.wav',0.),
             'reference':(Path(manifest['reference']['path']),-2.239)}
    audition={}
    for label,(start,end) in clip_groups.items():
        mask=qa.masks(source[round(start*sr):round(end*sr)],sr)[0]
        clips={};levels={}
        for key,(path,offset) in masters.items():
            x,rate,_=qa.decode(path);clip=x[round((start+offset)*rate):round((end+offset)*rate)]
            r=qa.frames(clip,rate);levels[key]=float(np.sqrt(np.mean(r[mask]**2)));clips[key]=clip
        target=min(levels.values());gains={k:target/max(v,1e-12) for k,v in levels.items()}
        peak=max(float(np.max(abs(clips[k]*gains[k]))) for k in clips)
        common=min(1.,10**(-3/20)/max(peak,1e-12))
        audition[label]={}
        for key,clip in clips.items():
            output=root/'listening'/f'{label}_{key}_matched.wav';output.parent.mkdir(exist_ok=True)
            write_evidence(output,clip*gains[key]*common,sr)
            audition[label][key]=dict(path=str(output.resolve()),original_timeline=[start,end],file_offset_seconds=masters[key][1],
                match_gain_db=qa.db(gains[key]),common_headroom_gain_db=qa.db(common),sha256=qa.sha(output),duration_seconds=len(clip)/sr)
    (root/'listening-manifest.json').write_text(json.dumps(audition,indent=2))
    print(json.dumps(results,indent=2))

if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--root',required=True);args=p.parse_args();compare(args.root)
