"""Supplemental saved-stem and three-way target auditions; static down-only gains."""
import json
from pathlib import Path
import numpy as np
import compare_saved as c
q=c.qa;sr=c.SR;here=c.HERE;trial=c.TRIAL;r3=c.R3
p=json.loads((here/'comparison.json').read_text());x=c.load(Path(p['inputs']['original']['path']));m=np.load(trial/'finnegan-primary/source_masks.npz');a,b=20,32;sl=slice(round(a*sr),round(b*sr));sm=(m['speech'][1000:1600],m['pause'][1000:1600])
def save(folder,arrs,roles):
 out=here/folder;out.mkdir(exist_ok=False)
 levels={k:q.signal_metrics(v,sr,sm)['speech_rms_dbfs'] for k,v in arrs.items()};target=min(levels.values());gains={k:target-v for k,v in levels.items()};vals={k:(v*10**(gains[k]/20)).astype(np.float32) for k,v in arrs.items()};head=min(0,-3-q.db(max(np.max(abs(v)) for v in vals.values())))
 info={'source_timeline_seconds':[a,b],'source_mask_file':str(trial/'finnegan-primary/source_masks.npz'),'source_mask_sha256':q.sha(trial/'finnegan-primary/source_masks.npz'),'mask':'Sliced fixed full-original protected speech/pause masks; identical for all clips.','target_speech_rms_before_headroom_dbfs':target,'common_headroom_gain_db':head,'method':'Only downward constant gains; no processing/model runs. Lowest speech RMS of THIS group; common sample peak <=−3dBFS.','files':{}}
 for k,v in vals.items():
  path=out/(k+'_target20-32_matched.wav');v=(v*10**(head/20)).astype(np.float32);c.write_float(path,v,sr);assert len(v)==576000 and gains[k]<=0 and head<=0
  info['files'][k]={'path':str(path),'sha256':q.sha(path),'role':roles[k],'sample_count':len(v),'original_speech_rms_dbfs':levels[k],'static_match_gain_db':gains[k],'common_headroom_gain_db':head,'total_static_gain_db':gains[k]+head,'matched_metrics':q.signal_metrics(v,sr,sm),'Adobe_offset_seconds':c.OFFSET if k=='Adobe' else None}
 (out/'manifest.json').write_text(json.dumps(info,indent=2));return info
paths={k:Path(v['path']) for k,v in p['inputs'].items()}
triplet={k:c.load(paths[k])[round((a+(c.OFFSET if k=='Adobe' else 0))*sr):round((b+(c.OFFSET if k=='Adobe' else 0))*sr)] for k in ['B','model_dry20','Adobe']}
t=save('listening-triplet',triplet,{'B':'Prior preferred local B (bounded-tail r3), unapproved for promotion','model_dry20':'Single accepted-for-listening preservation repair; not human approved','Adobe':'Preferred edited opaque reference; not clean ground truth'})
bdir=r3/'diagnostics-bounded-tail';rdir=trial/'finnegan-repair-dry20'
stems={'original':x[sl],'B_cleanup':c.load(bdir/'cleanup.wav')[sl],'model_cleanup_dry20':c.load(rdir/'cleanup.wav')[sl],'model_raw_rejected':c.load(trial/'finnegan-primary/model_raw.wav')[sl]}
s=save('target-stage-comparison',stems,{k:('Rejected raw model diagnostic, not a listening candidate' if k=='model_raw_rejected' else 'Saved cleanup-stage diagnostic; no downstream presence/level/compression/master') for k in stems})
print('Triplet gains', {k:v['total_static_gain_db'] for k,v in t['files'].items()});print('Cleanup diagnostic saved')
