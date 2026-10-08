"""Matched diagnostics for the rejected attempt; never export a delivery MP3."""
import json
from pathlib import Path
import sys
import numpy as np
import soundfile as sf
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[3]
sys.path.insert(0,str(ROOT/'tools/radcast'))
import analysis as qa
import studio
from adaptive_preservation import apply
from model_trial import si_sdr


def run():
    assert not (HERE/'comparison.json').exists()
    trial=json.loads((HERE/'trial.qa.json').read_text())
    assert not trial['accepted_for_listening'] and not trial['watchdog']
    base=json.loads(Path(trial['baseline_report']).read_text())
    model_root=Path(trial['baseline_report']).parents[1]
    refs=json.loads((ROOT/'docs/radcast/studio-experiment/distance-investigation/analysis.json').read_text())['reference_integrity']
    for r in refs.values():assert qa.sha(r['path'])==r['sha256']
    paths={'old':Path(base['output']),'attempt':Path(trial['output']),
           'Adobe':Path(refs['reference']['path']),'original':Path(base['paths']['prepared']['path'])}
    for key,expected in (('old',base['output_sha256']),('attempt',trial['output_sha256']),
                          ('original',base['paths']['prepared']['sha256'])):
        assert qa.sha(paths[key])==expected
    audio={k:qa.decode(p,48000)[0] for k,p in paths.items()}
    masks=qa.masks(audio['original'],48000)
    result=dict(state='rejected_attempt_diagnostics_only',inputs={k:dict(path=str(p),sha256=qa.sha(p)) for k,p in paths.items()},
        groups={},known_controls={},new_model_inferences=0,delivery_export=False,
        note='Static audition copies from a rejected full attempt; these do not grant full-source listening acceptance.')
    out=HERE/'diagnostic-rms';out.mkdir()
    for name,(start,end) in dict(target_phrase=(20,32),quiet=(19.5,22.5),loud=(70,80),ending=(38.5,41.8)).items():
        sm=(masks[0][round(start*50):round(end*50)],masks[1][round(start*50):round(end*50)])
        clips={k:audio[k][round((start+(-2.239 if k=='Adobe' else 0))*48000):round((end+(-2.239 if k=='Adobe' else 0))*48000)] for k in ('old','attempt','Adobe')}
        levels={k:qa.signal_metrics(x,48000,sm)['speech_rms_dbfs'] for k,x in clips.items()}
        target=min(levels.values());gain={k:target-v for k,v in levels.items()}
        peak=max(np.max(abs(x*10**(gain[k]/20))) for k,x in clips.items());head=min(0,-3-qa.db(peak))
        files={}
        for k,x in clips.items():
            y=(x*10**((gain[k]+head)/20)).astype('float32');p=out/f'{name}_{k}.wav';studio.write_float(p,y,48000)
            files[k]=dict(path=str(p),sha256=qa.sha(p),sample_count=len(y),static_gain_db=gain[k]+head,
                          speech_rms_dbfs=qa.signal_metrics(y,48000,sm)['speech_rms_dbfs'],loudness=qa.loudness(p))
        assert max(f['speech_rms_dbfs'] for f in files.values())-min(f['speech_rms_dbfs'] for f in files.values())<1e-5
        result['groups'][name]=dict(timeline=[start,end],files=files)
        if name=='target_phrase':
            folder=HERE/'diagnostic-lufs';folder.mkdir();target=min(f['loudness']['integrated_lufs'] for f in files.values());ls={}
            for k,f in files.items():
                x,_=sf.read(f['path'],dtype='float32');g=target-f['loudness']['integrated_lufs'];assert g<=0
                y=(x*10**(g/20)).astype('float32');p=folder/f'target_phrase_{k}.wav';studio.write_float(p,y,48000)
                ls[k]=dict(path=str(p),sha256=qa.sha(p),sample_count=len(y),total_static_gain_db=f['static_gain_db']+g,loudness=qa.loudness(p))
            result['target_lufs']=ls
    folder=HERE/'diagnostic-faint';folder.mkdir()
    # One whole-recording typical-gain calibration. Do not independently RMS
    # match faint clips: that would conceal the rejected signal loss.
    gains={'old':0.,'attempt':-trial['metrics_against_baseline']['speech_window_gain_median_db'],
           'original':base['metrics']['speech_window_gain_median_db']}
    for name,(start,end) in dict(opening=(0,8),missed_material=(2,3.5),internal=(66,69),ending=(137,143.49),intro=(0,12)).items():
        clips={k:(audio[k][round(start*48000):round(end*48000)]*10**(gains[k]/20)).astype('float32') for k in gains}
        head=-3-qa.db(max(np.max(abs(x)) for x in clips.values()));files={}
        for k,x in clips.items():
            y=(x*10**(head/20)).astype('float32');p=folder/f'{name}_{k}.wav';studio.write_float(p,y,48000)
            files[k]=dict(path=str(p),sha256=qa.sha(p),sample_count=len(y),global_calibration_gain_db=gains[k],common_audition_gain_db=head)
        result['groups'][name]=dict(timeline=[start,end],files=files,note='Common audition gain after global typical-gain calibration; local faint loss remains audible. No local RMS matching. Missing-material excerpt magnifies very faint content; no semantic speech classification.')
    controls=json.loads((model_root/'controls/controls.json').read_text())
    for speaker,r in controls['speakers'].items():
        a={}
        for k,f in r['paths'].items():
            assert qa.sha(f['path'])==f['sha256'];x,sr=sf.read(f['path'],dtype='float32');assert sr==48000;a[k]=x
        result['known_controls'][speaker]={}
        for label in ('clean','synthetic_room'):
            y,w,info=apply(a[label],a[label],a[label+'_model'],48000)
            result['known_controls'][speaker][label]=dict(active_frames=info['active_frame_count'],
                energy_bypass_intervals=info['energy_guard_bypassed_intervals'],
                output_identical_to_saved_model=bool(np.array_equal(y,a[label+'_model'])),
                paired_clean_si_sdr_db=si_sdr(a['clean'],y))
    (HERE/'comparison.json').write_text(json.dumps(result,indent=2,allow_nan=False)+'\n')
    print(json.dumps(dict(target_lufs={k:r['loudness'] for k,r in result['target_lufs'].items()},controls=result['known_controls'],export=False),indent=2),flush=True)


if __name__=='__main__':run()
