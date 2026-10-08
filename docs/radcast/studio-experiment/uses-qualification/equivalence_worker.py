"""One backend of a saved trained equivalence audit; separate RSS process."""
import json
from pathlib import Path
import sys
import numpy as np
import soundfile as sf
HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(HERE.parents[3]/'tools/radcast'))
import analysis as qa
import studio
from uses_trial import UsesModel

backend=sys.argv[1];assert backend in ('upstream','bounded')
folder=HERE/'equivalence';folder.mkdir(exist_ok=True)
assert not (folder/(backend+'.json')).exists()
manifest=HERE.parent/'model-feasibility/treble-ism-eng120-epoch119/controls/controls.json'
r=json.loads(manifest.read_text())['speakers']['p232']['paths']['clean'];assert qa.sha(r['path'])==r['sha256']
audio,rate=sf.read(r['path'],dtype='float32');x=audio[48000:153600]
model=UsesModel(HERE/'artifacts',backend=backend)
print(f'{backend}: trained2.2s native48k whole-utterance comparison running',flush=True)
y=model.cleanup(x,rate)
studio.write_float(folder/(backend+'.wav'),y,rate)
record=dict(backend=backend,model=model.info,trace=model.last_trace,source=r,
    selected_samples=[48000,153600],input_pcm_sha256=__import__('hashlib').sha256(x.tobytes()).hexdigest(),
    output_path=str(folder/(backend+'.wav')),output_sha256=qa.sha(folder/(backend+'.wav')),
    finite=bool(np.isfinite(y).all()),sample_count=len(y),new_finnegan_candidates=0)
(folder/(backend+'.json')).write_text(json.dumps(record,indent=2)+'\n')
print(json.dumps(dict(backend=backend,seconds=model.last_trace['wall_seconds'],peak_rss=model.last_trace['process_peak_rss_bytes'],samples=len(y))),flush=True)
