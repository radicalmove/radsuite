"""One guarded full-source USES candidate after successful controls only."""
import json
from pathlib import Path
import tempfile
import numpy as np
import soundfile as sf
import analysis as qa
import studio
import speech_cleanup as sc
from model_trial import fresh_evidence_directory
from uses_trial import UsesModel,validate_audio

BASELINE_REPORT_SHA='5855b9cac8caeed1229c094bccfaadb1a28177255b783b844685a98b5b06a4a9'
TASK_EVIDENCE_DIR=Path(__file__).resolve().parents[2]/'docs/radcast/studio-experiment/uses-qualification'


def qualified_controls(report):
    if not report.get('accepted_for_finnegan') or not report.get('qualification',{}).get('passed'):
        raise ValueError('Controls did not qualify; no Finnegan candidate permitted')
    if report.get('inference_counts')!={'uses':8,'new_treble':4}:
        raise ValueError('Required same-input clean/room/baseline controls are incomplete')
    return report


def validate_baseline_identity(base,digest):
    if digest!=BASELINE_REPORT_SHA or base.get('explicit_dry_blend')!=.2:
        raise ValueError('Only the reviewed pinned dry20 report may supply the guard reference')
    if not base.get('accepted_for_listening') or base.get('fallback') or base.get('watchdog'):
        raise ValueError('Expected the previously accepted dry20 reference')


def require_equivalence(info):
    digest=info.get('equivalence_report_sha256')
    if (info.get('execution_backend')!='bounded_features' or info.get('trained_backend_equivalence_verified') is not True
            or not isinstance(digest,str) or len(digest)!=64):
        raise ValueError('Passed identity-bound numerical backend evidence is required')


def claim_attempt(directory,record):
    path=Path(directory)/'finnegan-attempt.json'
    with path.open('x') as f:f.write(json.dumps(record,indent=2,allow_nan=False)+'\n')
    return path


def render(controls_report,baseline_report,outdir,model_dir):
    controls_path=Path(controls_report);controls=qualified_controls(json.loads(controls_path.read_text()))
    baseline_path=Path(baseline_report);base=json.loads(baseline_path.read_text())
    validate_baseline_identity(base,qa.sha(baseline_path))
    for module in (qa,studio,sc):
        if qa.sha(module.__file__)!=base['code_sha256'][Path(module.__file__).name]:
            raise ValueError('Previously pinned downstream helper changed')
    for key in ('prepared','highpassed'):
        r=base['paths'][key]
        if qa.sha(r['path'])!=r['sha256']:raise ValueError('Original-derived input changed')
    if qa.sha(base['input'])!=base['input_sha256'] or qa.sha(base['output'])!=base['output_sha256']:
        raise ValueError('Original or accepted reference changed')
    source,sr=sf.read(base['paths']['prepared']['path'],dtype='float32');source=validate_audio(source,sr)
    dry,dr=sf.read(base['paths']['highpassed']['path'],dtype='float32');dry=validate_audio(dry,dr)
    if len(source)!=len(dry):raise ValueError('Prepared sample count changed')
    model=UsesModel(model_dir,backend='bounded')
    if model.info!=controls.get('model'):raise ValueError('Qualified model/runtime identity differs')
    # Full features exceed local memory in the unmodified execution path.
    # A validated equivalent bounded-intermediate path is required, without
    # independently enhanced audio chunks or model-memory resets.
    require_equivalence(model.info)
    folder=fresh_evidence_directory(outdir);paths={}
    def save(name,x,role='processing_stem'):
        p=folder/(name+'.wav');studio.write_float(p,x,sr)
        paths[name]=dict(path=str(p.resolve()),sha256=qa.sha(p),sample_count=len(x),role=role)
        return p
    (folder/'started.json').write_text(json.dumps(dict(controls_report=str(controls_path.resolve()),controls_sha256=qa.sha(controls_path),model=model.info,maximum_candidate_renders=1,remaining_repair_budget=0),indent=2)+'\n')
    save('prepared',source,'original_derived_input');save('highpassed',dry)
    ledger=claim_attempt(TASK_EVIDENCE_DIR,dict(state='consumed_before_full_inference',
        outdir=str(folder.resolve()),controls_report=str(controls_path.resolve()),controls_sha256=qa.sha(controls_path),
        input_sha256=base['input_sha256'],model=model.info,maximum_full_renders=1,remaining_repair_budget=0))
    print('Full original-source USES cleanup running',flush=True)
    cleanup=model.cleanup(dry,sr);save('model_raw',cleanup);save('cleanup',cleanup)
    (folder/'inference-trace.json').write_text(json.dumps(model.last_trace,indent=2)+'\n')
    speech,pause=qa.masks(source,sr);np.savez(folder/'source_masks.npz',speech=speech,pause=pause,sample_rate=sr,frame_seconds=.02)
    y=studio.pause_control(cleanup,source,sr,max_db=base['downstream_config']['pause_max_attenuation_db']);save('pause_control',y)
    presence=base['presence'].copy();y=sc.presence(y,sr,gain_db=presence['gain_db']);save('presence',y)
    trace={};y,leveling=sc.level_speech(y,source,sr,speech,trace=trace)
    np.save(folder/'level_frame_gain_db.npy',trace['frame_gain_db']);save('levelled',y)
    with tempfile.TemporaryDirectory(prefix='radcast-uses-master-') as temp:
        y,mastering=studio.master(y,sr,Path(temp),compression=True)
    if y.shape!=source.shape or not np.isfinite(y).all():raise ValueError('Invalid candidate output')
    output=save('master_attempt',y,'attempted_master')
    masks=(speech,pause);source_metrics=qa.signal_metrics(source,sr,masks)
    measured=qa.signal_metrics(y,sr,masks);measured.update(qa.loudness(output));measured.update(qa.alignment(source,y,sr))
    defects=qa.watchdog(source_metrics,measured)
    old,_=sf.read(base['output'],dtype='float32');before=qa.signal_metrics(old,sr,masks)
    relative=qa.signal_metrics(y,sr,masks);relative.update(qa.loudness(output));relative.update(qa.alignment(old,y,sr))
    prior_defects=qa.watchdog(before,relative);accepted=not defects and not prior_defects
    if accepted:
        delivery=folder/'CRJU160_USES_dereverb_candidate.wav';output.rename(delivery);output=delivery
        paths['master_attempt'].update(path=str(output.resolve()),role='qa_accepted_listening_candidate')
    report=dict(schema_version=1,experiment='legacy_uses_native48k_dereverb',model=model.info,
        input=base['input'],input_sha256=base['input_sha256'],output=str(output.resolve()),output_sha256=qa.sha(output),
        controls_report=str(controls_path.resolve()),controls_sha256=qa.sha(controls_path),
        baseline_report=str(baseline_path.resolve()),baseline_report_sha256=qa.sha(baseline_path),baseline_master_sha256=base['output_sha256'],
        sample_rate=sr,working_format='float32 PCM',sample_count=len(y),paths=paths,
        source_metrics=source_metrics,metrics=measured,baseline_metrics=before,metrics_against_baseline=relative,
        watchdog=defects,watchdog_against_baseline=prior_defects,accepted_for_listening=accepted,
        presence=presence,leveling=leveling,level_gain_curve='Recalculated once with the unchanged original-mask leveling policy; old model-specific curve not reused.',
        downstream_config=base['downstream_config'],mastering=mastering,inference_trace=model.last_trace,
        original_blend=0.,model_cascade=False,fallback=False,native_application_changed=False,
        full_candidate_count=1,remaining_render_budget=0,remaining_repair_budget=0,
        attempt_ledger=str(ledger),attempt_ledger_sha256=qa.sha(ledger),
        code_sha256={Path(m.__file__).name:qa.sha(m.__file__) for m in (qa,studio,sc)},
        renderer_sha256=qa.sha(__file__),adapter_sha256=qa.sha(Path(__file__).with_name('uses_trial.py')),
        note='One original-derived full run; legacy USES is not USES2. Technical pass is not voice identity, articulation, proximity or Adobe parity approval.')
    (folder/'trial.qa.json').write_text(json.dumps(report,indent=2,allow_nan=False)+'\n')
    print(json.dumps(dict(accepted_for_listening=accepted,watchdog=defects,baseline_watchdog=prior_defects,output=str(output)),indent=2),flush=True)
    return report


if __name__=='__main__':
    import argparse
    p=argparse.ArgumentParser();p.add_argument('--controls-report',required=True);p.add_argument('--baseline-report',required=True);p.add_argument('--outdir',required=True);p.add_argument('--model-dir',required=True)
    a=p.parse_args();render(a.controls_report,a.baseline_report,a.outdir,a.model_dir)
