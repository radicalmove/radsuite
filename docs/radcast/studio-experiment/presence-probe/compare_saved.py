"""Static audition controls and same-settings ASR for saved full outputs only."""
import json
import re
import subprocess
import sys
from pathlib import Path
import numpy as np
import soundfile as sf
ROOT=Path(__file__).resolve().parents[4]
sys.path.insert(0,str(ROOT/'tools/radcast'))
import analysis as qa
import studio
from asr_compare import edit_distance

HERE=Path(__file__).resolve().parent
MODEL_ROOT=ROOT/'docs/radcast/studio-experiment/model-feasibility/treble-ism-eng120-epoch119'
RATE=48000


def run():
    trial=json.loads((HERE/'trial.qa.json').read_text())
    if not trial['accepted_for_listening'] or trial['watchdog'] or trial['watchdog_against_baseline']:
        raise ValueError('No accepted presence candidate')
    base=json.loads(Path(trial['baseline_report']).read_text())
    old=json.loads((MODEL_ROOT/'comparison/comparison.json').read_text())
    paths={'previous':Path(base['output']),'presence':Path(trial['output']),
           'Adobe':Path(old['inputs']['Adobe']['path'])}
    assert qa.sha(paths['previous'])==trial['baseline_master_sha256']
    assert qa.sha(paths['presence'])==trial['output_sha256']
    assert qa.sha(paths['Adobe'])==old['inputs']['Adobe']['sha256']
    source,_=sf.read(trial['prepared_original']['path'],dtype='float32');masks=qa.masks(source,RATE)
    audio={k:qa.decode(p,RATE)[0] for k,p in paths.items()}
    folders=('listening-rms','listening-lufs','asr')
    if any((HERE/f).exists() for f in folders):raise ValueError('Preserve comparison history')
    for f in folders:(HERE/f).mkdir()
    result=dict(inputs={k:dict(path=str(v),sha256=qa.sha(v)) for k,v in paths.items()},
                masks='Same full-original-derived 20ms masks sliced for each group',
                Adobe_offset_seconds=-2.239,groups={},note='All clips cut from full recordings. Static down-only audition gains; no enhancement or EQ per excerpt. No quality score.')
    for label,(start,end) in dict(target_phrase=(20,32),quiet_articulation=(19.5,22.5),
                                  louder_passage=(70,80),phrase_ending=(38.5,41.8)).items():
        clips={k:x[round((start+(-2.239 if k=='Adobe' else 0))*RATE):round((end+(-2.239 if k=='Adobe' else 0))*RATE)] for k,x in audio.items()}
        sm=(masks[0][round(start*50):round(end*50)],masks[1][round(start*50):round(end*50)])
        levels={k:qa.signal_metrics(x,RATE,sm)['speech_rms_dbfs'] for k,x in clips.items()}
        target=min(levels.values());gains={k:target-v for k,v in levels.items()}
        peak=max(np.max(abs(x*10**(gains[k]/20))) for k,x in clips.items())
        head=min(0.,-3-qa.db(peak));group=dict(timeline=[start,end],common_headroom_db=head,target_speech_rms_dbfs=target,files={})
        for k,x in clips.items():
            y=(x*10**((gains[k]+head)/20)).astype('float32');p=HERE/'listening-rms'/f'{label}_{k}.wav'
            studio.write_float(p,y,RATE);assert len(y)==round((end-start)*RATE)
            group['files'][k]=dict(path=str(p),sha256=qa.sha(p),static_gain_db=gains[k]+head,
                sample_count=len(y),metrics=qa.signal_metrics(y,RATE,sm),loudness=qa.loudness(p))
        result['groups'][label]=group
        if label=='target_phrase':
            # LUFS comparison is a separate static control, never another
            # processed candidate. Preserve RMS controls above unchanged.
            target_lufs=min(r['loudness']['integrated_lufs'] for r in group['files'].values())
            lufs_group=dict(timeline=[start,end],target_lufs=target_lufs,files={})
            for k,r in group['files'].items():
                y,_=sf.read(r['path'],dtype='float32');gain=target_lufs-r['loudness']['integrated_lufs'];assert gain<=0
                y=(y*10**(gain/20)).astype('float32');p=HERE/'listening-lufs'/f'target_phrase_{k}.wav';studio.write_float(p,y,RATE)
                lufs_group['files'][k]=dict(path=str(p),sha256=qa.sha(p),additional_static_gain_db=gain,
                    total_static_gain_db=r['static_gain_db']+gain,sample_count=len(y),loudness=qa.loudness(p),metrics=qa.signal_metrics(y,RATE,sm))
            result['target_lufs_control']=lufs_group
    (HERE/'comparison.json').write_text(json.dumps(result,indent=2,allow_nan=False))
    print('RMS/LUFS matched controls saved',flush=True)
    prior=json.loads((MODEL_ROOT/'comparison/asr-prior-settings.json').read_text())
    model=Path('/Users/rcd58/.radcast/whispercpp-models/ggml-small.bin')
    whisper=Path('/opt/homebrew/bin/whisper-cli')
    words=lambda text:re.findall(r"[a-z]+(?:'[a-z]+)?",text.lower())
    original=words(prior['files']['original']['transcript'])
    asr=dict(model=str(model),model_sha256=qa.sha(model),binary=str(whisper),binary_sha256=qa.sha(whisper),
        original_proxy_path=str(MODEL_ROOT/'comparison/asr-prior-settings.json'),
        original_proxy_sha256=qa.sha(MODEL_ROOT/'comparison/asr-prior-settings.json'),files={},
        note='Same cached small model/settings; 16k copies are analysis-only. Original-ASR edits are not WER, identity or lisp tests.')
    assert asr['model_sha256']==prior['model_sha256']
    for k in ('previous','presence'):
        wav=HERE/'asr'/f'{k}_analysis16k.wav';prefix=HERE/'asr'/k
        qa.run(['ffmpeg','-v','error','-i',paths[k],'-ac','1','-ar','16000','-c:a','pcm_s16le',wav])
        cmd=[str(whisper),'-m',str(model),'-f',str(wav),'-l','en','-t','4','-nf','-nt','-otxt','-of',str(prefix)]
        p=subprocess.run(cmd,capture_output=True);(HERE/'asr'/f'{k}.log').write_bytes(p.stdout+b'\nSTDERR\n'+p.stderr);p.check_returncode()
        text=prefix.with_suffix('.txt').read_text();w=words(text);edits=edit_distance(original,w)
        asr['files'][k]=dict(transcript=text,word_count=len(w),proxy_edits=edits,proxy_edit_rate=edits/len(original),
                            command=cmd,analysis_copy_sha256=qa.sha(wav))
        print(k,'ASR proxy edits',edits,'/',len(original),flush=True)
    (HERE/'asr-comparison.json').write_text(json.dumps(asr,indent=2,allow_nan=False))


if __name__=='__main__':run()
