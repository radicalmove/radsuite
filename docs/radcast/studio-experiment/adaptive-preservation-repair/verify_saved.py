"""Independent saved-artifact audit; no enhancement inference or candidate render."""
import ast
import json
from pathlib import Path
import sys
import tempfile
import numpy as np
import soundfile as sf
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[3]
sys.path.insert(0,str(ROOT/'tools/radcast'))
import analysis as qa
import studio
import speech_cleanup as sc


def checked(record):
    assert qa.sha(record['path'])==record['sha256'],record['path']
    return record


def read(path):
    info=sf.info(path);x,sr=sf.read(path,dtype='float32')
    assert sr==48000 and info.channels==1 and info.subtype=='FLOAT'
    assert np.isfinite(x).all()
    return x


def run():
    target=HERE/'verification.json';assert not target.exists()
    trial=json.loads((HERE/'trial.qa.json').read_text());export=json.loads((HERE/'export.qa.json').read_text())
    base=json.loads(Path(trial['baseline_report']).read_text());c=json.loads((HERE/'comparison.json').read_text())
    modelroot=Path(trial['baseline_report']).parents[1]
    assert qa.sha(trial['baseline_report'])==trial['baseline_report_sha256']
    assert qa.sha(trial['output'])==trial['output_sha256'] and qa.sha(export['output'])==export['output_sha256']
    assert qa.sha(ROOT/'tools/radcast/adaptive_preservation.py')==trial['adapter_sha256']
    for name,digest in trial['code_sha256'].items():assert qa.sha(ROOT/'tools/radcast'/name)==digest
    checked(trial['saved_level_gain']);checked(trial['decision_only_protection_reference'])
    for records in (trial['original_derived_inputs'],trial['paths']):
        for record in records.values():checked(record)
    refs=json.loads((ROOT/'docs/radcast/studio-experiment/distance-investigation/analysis.json').read_text())['reference_integrity']
    for record in refs.values():checked(record)
    runtime=json.loads((modelroot/'runtime-identity.json').read_text())
    for record in runtime['files']:checked(record)
    rejected=HERE.parent/'adaptive-preservation'
    snapshot=json.loads((rejected/'implementation-snapshot.json').read_text())
    for name,digest in snapshot['files'].items():assert qa.sha(rejected/name)==digest
    assert snapshot['files']['adaptive_preservation.py']==json.loads((rejected/'trial.qa.json').read_text())['adapter_sha256']
    source=read(trial['original_derived_inputs']['prepared']['path'])
    raw=read(trial['original_derived_inputs']['model_raw']['path']);hp=read(trial['original_derived_inputs']['highpassed']['path'])
    x={k:read(r['path']) for k,r in trial['paths'].items()};master=x['master_attempt'];old=read(base['output'])
    weights=np.load(HERE/'blend_frame_weights.npy',allow_pickle=False)
    assert np.isfinite(weights).all() and len(weights)==len(source)//960
    assert weights.min()>=0 and weights.max()<=.2
    curve=np.interp(np.arange(len(source)),np.arange(len(weights))*960+480,weights,left=weights[0],right=weights[-1])
    expected=((1-curve)*raw+curve*hp).astype('float32')
    assert np.array_equal(expected,x['cleanup'])
    assert np.array_equal(x['cleanup'][20*48000:32*48000],raw[20*48000:32*48000])
    masks=qa.masks(source,48000);protect=masks[0]|qa.masks(read(trial['decision_only_protection_reference']['path']),48000)[0]
    before=qa.frames(raw,48000);eligible=protect&(qa.frames(source,48000)>1e-5)&(before>1e-8)
    delta=20*np.log10(np.maximum(qa.frames(x['cleanup'],48000),1e-30)/np.maximum(before,1e-30))
    assert not np.any(eligible&(delta<-.1))
    pause=studio.pause_control(x['cleanup'],source,48000)
    presence=sc.presence(pause,48000,gain_db=trial['frozen_presence']['gain_db'])
    gains=np.load(trial['saved_level_gain']['path'],allow_pickle=False)
    gaincurve=np.interp(np.arange(len(source)),np.arange(len(gains))*960+480,gains,left=gains[0],right=gains[-1])
    levelled=(presence*10**(gaincurve/20)).astype('float32')
    with tempfile.TemporaryDirectory(prefix='radcast-audit-master-') as folder:
        replay,_=studio.master(levelled,48000,Path(folder))
    errors={k:float(np.max(abs(a-b))) for k,a,b in [('pause',pause,x['pause_control']),('presence',presence,x['presence']),('levelled',levelled,x['levelled']),('master',replay,master)]}
    assert max(errors.values())==0
    for reference in (source,old):
        rm=qa.signal_metrics(reference,48000,masks);ym=qa.signal_metrics(master,48000,masks)
        ym.update(qa.loudness(trial['output']));ym.update(qa.alignment(reference,master,48000))
        assert not qa.watchdog(rm,ym)
    audio={k:qa.decode(checked(r)['path'],48000)[0] for k,r in c['inputs'].items()}
    clips=[];rms_spreads={}
    # The original comparison's 'ending' key was overwritten by its separate
    # faint ending. Recover its three already saved RMS clips independently;
    # future comparison code uses short_ending. No audio is rewritten.
    for name,(start,end) in dict(target_phrase=(20,32),quiet=(19.5,22.5),loud=(70,80),ending=(38.5,41.8)).items():
        sm=tuple(m[round(start*50):round(end*50)] for m in masks)
        rawclips={k:a[round((start+(-2.239 if k=='Adobe' else 0))*48000):round((end+(-2.239 if k=='Adobe' else 0))*48000)] for k,a in audio.items() if k!='original'}
        levels={k:qa.signal_metrics(a,48000,sm)['speech_rms_dbfs'] for k,a in rawclips.items()}
        gains={k:min(levels.values())-v for k,v in levels.items()}
        head=min(0,-3-qa.db(max(np.max(abs(a*10**(gains[k]/20))) for k,a in rawclips.items())))
        measured=[]
        for k,a in rawclips.items():
            path=HERE/'listening-rms'/f'{name}_{k}.wav';y=read(path)
            assert np.array_equal(y,(a*10**((gains[k]+head)/20)).astype('float32'))
            measured.append(qa.signal_metrics(y,48000,sm)['speech_rms_dbfs'])
            clips.append(dict(path=str(path),sha256=qa.sha(path),samples=len(y),static_replay_exact=True))
        rms_spreads[name]=max(measured)-min(measured);assert rms_spreads[name]<1e-5
    for name,g in c['groups'].items():
        if 'note' not in g:continue
        start,end=g['timeline']
        for k,r in g['files'].items():
            checked(r);y=read(r['path']);a=(audio[k][round(start*48000):round(end*48000)]*10**(r['global_calibration_gain_db']/20)).astype('float32')
            assert np.array_equal(y,(a*10**(r['common_audition_gain_db']/20)).astype('float32'))
            assert len(y)==r['sample_count']
            clips.append(dict(path=r['path'],sha256=qa.sha(r['path']),samples=len(y),static_replay_exact=True))
    lufs={}
    for k,r in c['target_lufs'].items():
        checked(r);y=read(r['path']);prior=c['groups']['target_phrase']['files'][k]
        gain=r['total_static_gain_db']-prior['static_gain_db'];assert gain<=0
        assert np.array_equal(y,(read(prior['path'])*10**(gain/20)).astype('float32'))
        lufs[k]=qa.loudness(r['path']);assert lufs[k]==r['loudness']
        clips.append(dict(path=r['path'],sha256=r['sha256'],samples=len(y),static_replay_exact=True))
    assert len(clips)==len(list(HERE.glob('listening-*/*.wav')))==30
    for path in HERE.glob('*.json'):json.loads(path.read_text())
    for path in [*HERE.glob('*.py'),ROOT/'tools/radcast/adaptive_preservation.py',ROOT/'tools/radcast/test_adaptive_preservation.py']:ast.parse(path.read_text())
    assert trial['accepted_for_listening'] and export['accepted_for_listening']
    assert not trial['watchdog'] and not trial['watchdog_against_baseline'] and not export['watchdog']
    result=dict(state='saved_artifacts_verified_human_listening_pending',runtime_files_unchanged=len(runtime['files']),protected_references_unchanged=len(refs),
        first_attempt_snapshots_unchanged=True,adapter_sha256=trial['adapter_sha256'],input_stems_unchanged=True,
        adaptive_cleanup_replay_exact=True,target_cleanup_bit_identical_to_raw=True,downstream_replay_max_errors=errors,
        remaining_energy_guard_conflicts=0,original_and_baseline_watchdogs=[],
        sample_count=len(master),sample_rate=48000,working_format='FLOAT',
        active_sample_count=int(np.count_nonzero(curve)),unblended_sample_fraction=float(np.mean(curve==0)),
        mean_original_weight=float(curve.mean()),maximum_original_weight=float(curve.max()),
        sample_activity_note='Interpolation extends nonzero frame support about10ms each side; blend percentages are not echo removal measurements.',
        master_loudness=qa.loudness(trial['output']),mp3_sha256=qa.sha(export['output']),export_watchdog=[],
        audition_clips=clips,rms_spread_db=rms_spreads,target_lufs=lufs,
        target_lufs_spread=max(v['integrated_lufs'] for v in lufs.values())-min(v['integrated_lufs'] for v in lufs.values()),
        comparison_metadata_note='Three short ending RMS clips independently verified here. Original comparison ending key was replaced by faint ending metadata; future script uses short_ending. Audio and original comparison preserved.',
        new_candidate_renders=0,new_model_inferences=0)
    target.write_text(json.dumps(result,indent=2,allow_nan=False)+'\n')
    print(json.dumps({k:v for k,v in result.items() if k not in ('audition_clips','target_lufs')},indent=2),flush=True)


if __name__=='__main__':run()
