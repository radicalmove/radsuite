"""Fixed TFGridNet qualification, with all logical and neural calls recorded."""
import json
from pathlib import Path
import time
import numpy as np
import soundfile as sf
import analysis as qa
from uses_controls import load_verified_controls,select_verified_prefix,prepare_inputs,_save_audio,_assess,fresh_directory
from tfgrid_trial import TFGridModel,sha
from model_trial import TrebleModel


def gate(speakers,baseline=False):
    reasons=[]
    if set(speakers)!={'p232','p257'}:reasons.append('Both licensed speakers required')
    for speaker,r in speakers.items():
        for name in ('clean','early','late','composite'):
            c=r.get('conditions',{}).get(name,{})
            if 'paired' not in c:reasons.append(f'{speaker}/{name}: incomplete');continue
            if c.get('technical_issues'):reasons.append(f'{speaker}/{name}: technical issues')
            loss=c.get('processing',{}).get('trace',{}).get('join_energy_protected_worst_db')
            if loss is not None and loss < -1:reasons.append(f'{speaker}/{name}: overlap energy loss exceeds1dB')
            if name=='clean':
                if c.get('preservation',{}).get('watchdog'):reasons.append(f'{speaker}/clean: waveform watchdog')
                if c['paired']['output_si_sdr_db']<15:reasons.append(f'{speaker}/clean: clean SI-SDR below15dB')
            if name=='early' and c['paired']['improvement_db'] < -1:reasons.append(f'{speaker}/early: more than1dB worse than untreated')
            if name in ('late','composite') and c['paired']['improvement_db']<1:reasons.append(f'{speaker}/{name}: less than1dB benefit')
        if baseline:
            for name in ('late','composite'):
                old=r.get('new_treble',{}).get('conditions',{}).get(name,{})
                current=r.get('conditions',{}).get(name,{})
                if 'paired' not in old or 'paired' not in current or old.get('technical_issues'):
                    reasons.append(f'{speaker}/{name}: baseline incomplete');continue
                if current['paired']['output_si_sdr_db']-old['paired']['output_si_sdr_db']<1:
                    reasons.append(f'{speaker}/{name}: less than1dB over Treble')
    return dict(passed=not reasons,reasons=reasons,baseline_required=baseline,
        thresholds=dict(clean_si_sdr_min=15,early_improvement_min=-1,late_composite_improvement_min=1,
                        late_composite_over_treble_min=1,protected_join_energy_min_db=-1))


def run_controls(manifest_path,outdir,artifacts,treble_dir):
    m,a=load_verified_controls(manifest_path);prepared={};selections={}
    for speaker,old in a.items():
        prefix,selected=select_verified_prefix(old,m['speakers'][speaker]['source_boundaries'],2)
        prepared[speaker],_=prepare_inputs(prefix['clean'],prefix['synthetic_room']);selections[speaker]=selected
    root=Path(artifacts).resolve().parent;device=json.loads((root/'device-selection.json').read_text())
    model=TFGridModel(artifacts,device=device['selected_device']);f=fresh_directory(outdir)
    result=dict(status='running',model=model.info,device_selection_sha256=sha(root/'device-selection.json'),
        source_manifest=str(Path(manifest_path).resolve()),source_manifest_sha256=sha(manifest_path),
        source_code_sha256={n:sha(Path(__file__).with_name(n)) for n in ('tfgrid_controls.py','tfgrid_trial.py','uses_controls.py','uses_trial.py','analysis.py','model_trial.py')},
        speakers={},inference_counts=dict(tfgrid_signals=0,tfgrid_windows=0,new_treble=0),accepted_for_finnegan=False,
        note='One frozen3s/2shop model policy; paired signal measures are not perceived proximity or identity.')
    def persist():
        temp=f/'controls.json.tmp';temp.write_text(json.dumps(result,indent=2,allow_nan=False)+'\n');temp.replace(f/'controls.json')
    for s,inputs in prepared.items():
        result['speakers'][s]=dict(selection=selections[s],conditions={})
        for name,x in inputs.items():result['speakers'][s]['conditions'][name]=dict(input=_save_audio(f/f'{s}_{name}_input.wav',x))
    persist()
    def process(processor,speaker,name,store,label):
        print(f'{label} start {speaker}/{name}',flush=True);x=prepared[speaker][name]
        start=time.perf_counter();y=processor.cleanup(x,48000)
        if label=='tfgrid':
            result['inference_counts']['tfgrid_signals']+=1
            result['inference_counts']['tfgrid_windows']+=sum(d['neural_calls'] for d in processor.last_trace['window_details'])
        else:result['inference_counts']['new_treble']+=1
        store[name]=dict(input=result['speakers'][speaker]['conditions'][name]['input'],
            output=_save_audio(f/f'{speaker}_{name}_{label}_output.wav',y),
            processing=dict(wall_seconds=time.perf_counter()-start,trace=processor.last_trace if label=='tfgrid' else None))
        persist();store[name].update(_assess(prepared[speaker]['clean'],x,y));persist()
        print(f'{label} done {speaker}/{name}: {store[name]["paired"]}',flush=True)
    try:
        for s in sorted(prepared):
            store=result['speakers'][s]['conditions']
            for name in ('clean','early','late','composite'):process(model,s,name,store,'tfgrid')
        result['prerequisites']=gate(result['speakers']);persist()
        if result['prerequisites']['passed']:
            del model;old=TrebleModel(treble_dir)
            for s in sorted(prepared):
                result['speakers'][s]['new_treble']=dict(model=old.info,conditions={});store=result['speakers'][s]['new_treble']['conditions']
                for name in ('late','composite'):process(old,s,name,store,'treble')
        result['qualification']=gate(result['speakers'],baseline=True)
        result['accepted_for_finnegan']=result['qualification']['passed'];result['status']='qualified' if result['accepted_for_finnegan'] else 'rejected'
        persist();load_verified_controls(manifest_path)
        return result
    except Exception as e:
        result.update(status='failed',error=dict(type=type(e).__name__,message=str(e)));persist();raise


if __name__=='__main__':
    import argparse
    p=argparse.ArgumentParser();p.add_argument('--manifest',required=True);p.add_argument('--outdir',required=True);p.add_argument('--artifacts',required=True);p.add_argument('--treble-dir',required=True)
    a=p.parse_args();r=run_controls(a.manifest,a.outdir,a.artifacts,a.treble_dir)
    print(json.dumps(dict(status=r['status'],gate=r['qualification'],counts=r['inference_counts'])),flush=True)
