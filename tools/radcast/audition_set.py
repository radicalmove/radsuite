"""Static level matching of already-rendered full masters; no enhancement here."""
import argparse,json
from pathlib import Path
import numpy as np
import analysis as qa
import studio

def make(root):
    root=Path(root);m=json.loads((root/'manifest.json').read_text());source,sr,_=qa.decode(m['original']['path'])
    paths={'r2':(root/'baseline_r2_delay4.wav',0),
           'shorter_wpe':(root/'CRJU160_studio_r3_wpe65_delay2.wav',0),
           'bounded_tail':(root/'CRJU160_studio_r3_bounded_tail2db.wav',0),
           'reference':(Path(m['reference']['path']),-2.239)}
    audio={key:qa.decode(path)[0] for key,(path,_) in paths.items()}
    folder=root/'listening-all';folder.mkdir(exist_ok=True);manifest={}
    for label,(start,end) in {'target_phrase':(20,32),'quiet_articulation':(19.5,22.5),'louder_passage':(70,80),'phrase_ending_context':(38.5,41.8)}.items():
        mask=qa.masks(source[round(start*sr):round(end*sr)],sr)[0];clips={};levels={}
        for key,x in audio.items():
            offset=paths[key][1];c=x[round((start+offset)*sr):round((end+offset)*sr)];clips[key]=c
            r=qa.frames(c,sr);levels[key]=float(np.sqrt(np.mean(r[mask]**2)))
        target=min(levels.values());gains={key:target/max(level,1e-12) for key,level in levels.items()}
        peak=max(float(np.max(abs(c*gains[key]))) for key,c in clips.items());common=min(1.,10**(-3/20)/max(peak,1e-12))
        manifest[label]={}
        for key,c in clips.items():
            out=folder/f'{label}_{key}_matched.wav'
            if out.exists():raise ValueError(f'Listening evidence already exists: {out}; use a new output set')
            studio.write_float(out,c*gains[key]*common,sr)
            manifest[label][key]=dict(path=str(out.resolve()),sha256=qa.sha(out),source_timeline=[start,end],
                file_offset_seconds=paths[key][1],duration_seconds=len(c)/sr,match_gain_db=qa.db(gains[key]),common_headroom_gain_db=qa.db(common))
    (root/'listening-all-manifest.json').write_text(json.dumps(manifest,indent=2))

if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--root',required=True);args=p.parse_args();make(args.root)
