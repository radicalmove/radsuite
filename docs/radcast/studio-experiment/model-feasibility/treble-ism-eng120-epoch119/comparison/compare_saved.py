"""Saved signal comparison only; no model import/inference. Fresh outputs required."""
import json,sys,re,subprocess
from pathlib import Path
import numpy as np
ROOT=Path(__file__).resolve().parents[6]
sys.path.insert(0,str(ROOT/'tools/radcast'))
import analysis as qa
from studio import write_float
HERE=Path(__file__).resolve().parent
TRIAL=HERE.parent; R3=ROOT/'docs/radcast/studio-experiment/revision3'
SR=48000; OFFSET=-2.239
WHISPER=Path('/opt/homebrew/bin/whisper-cli'); MODEL=Path('/Users/rcd58/.radcast/whispercpp-models/ggml-small.bin')

def corr(a,b):return float(np.corrcoef(a,b)[0,1])
def edit_distance(a,b):
 row=list(range(len(b)+1))
 for i,x in enumerate(a,1):
  new=[i]
  for j,y in enumerate(b,1):new.append(min(new[-1]+1,row[j]+1,row[j-1]+(x!=y)))
  row=new
 return row[-1]
def load(path):
 x,s,_=qa.decode(path,SR);assert s==SR;return x

def run():
 for folder in ['matched','diagnostics','asr']:
  (HERE/folder).mkdir(exist_ok=False)
 primary=json.loads((TRIAL/'finnegan-primary/trial.qa.json').read_text())
 repair=json.loads((TRIAL/'finnegan-repair-dry20/trial.qa.json').read_text())
 assert repair['accepted_for_listening'] and not repair['watchdog']
 rawsha=primary['paths']['model_raw']['sha256']; assert rawsha==repair['paths']['model_raw']['sha256']
 manifest=json.loads((R3/'manifest.json').read_text())
 # Manifest layout is checked by explicit source identity below.
 paths={'original':Path(primary['input']), 'B':R3/'CRJU160_studio_r3_bounded_tail2db.wav',
        'model_dry20':Path(repair['output']),
        'Adobe':Path('/Users/rcd58/OpenAI Projects/AI Explainer Video/Build/CRJU160_1.1.1_rev1/final_mix_v2-enhanced-v2.wav')}
 assert qa.sha(paths['original'])==primary['input_sha256']
 signals={k:load(p) for k,p in paths.items()}; x=signals['original']; n=len(x)
 assert len(signals['B'])==len(signals['model_dry20'])==n
 maskfile=TRIAL/'finnegan-primary/source_masks.npz'; saved=np.load(maskfile)
 masks=(saved['speech'],saved['pause']); computed=qa.masks(x,SR)
 assert all(np.array_equal(a,b) for a,b in zip(masks,computed))
 inputs={k:{'path':str(p),'sha256':qa.sha(p),'sample_count':len(signals[k]),'duration_seconds':len(signals[k])/SR} for k,p in paths.items()}
 out={'schema_version':1,'inputs':inputs,'original_source_masks':{'path':str(maskfile),'sha256':qa.sha(maskfile),'method':'Full original-derived fixed 20ms masks; source mask is sliced without re-estimating each excerpt.'},
  'identity':{'primary_model_raw_sha256':rawsha,'repair_model_raw_sha256':rawsha,'equal':True},
  'interpretation':'Numeric preservation/level/spectral checks do not measure room dryness, speaker identity, or listening quality. Adobe is an edited opaque reference, not clean ground truth. No Adobe mechanism inferred.',
  'offset':{'Adobe_seconds':OFFSET,'source':'Prior waveform-inspection/FINDINGS.md (10–40 and70–80 s common content before later edits); fixed before this comparison, not fitted to phase/correlation.', 'full_reference_not_aligned':'Adobe is shorter/edited; full-file ASR comparison is descriptive only.'},
  'full_metrics':{},'excerpts':{},'stage_diagnostics':{}}
 for k in ['original','B','model_dry20']:
  out['full_metrics'][k]=dict(qa.signal_metrics(signals[k],SR,masks),**qa.loudness(paths[k]))
  if k!='original':out['full_metrics'][k].update(qa.alignment(x,signals[k],SR))
 out['full_metrics']['Adobe_native_descriptive_only']=dict(duration_seconds=len(signals['Adobe'])/SR,**qa.loudness(paths['Adobe']))
 regions={'target_phrase':(20,32),'quiet_articulation':(19.5,22.5),'louder_passage':(70,80),'phrase_ending_context':(38.5,41.8),'leading_handle_context':(0,8),'trailing_handle_context':(137,143.49)}
 def matching(group,arrs,start,end,folder,descriptions=None):
  ns=round((end-start)*SR); speech=masks[0][round(start/.02):round(end/.02)]; pause=masks[1][round(start/.02):round(end/.02)]
  levels={k:qa.signal_metrics(v,SR,(speech,pause))['speech_rms_dbfs'] for k,v in arrs.items()}; assert all(v is not None for v in levels.values())
  target=min(levels.values()); gains={k:target-v for k,v in levels.items()}
  scaled={k:v*np.float32(10**(gains[k]/20)) for k,v in arrs.items()}
  peak=max(float(np.max(abs(v))) for v in scaled.values()); head=min(0,-3-qa.db(peak))
  info={'source_timeline_seconds':[start,end],'match_method':'Downward-only static gain to lowest common original-mask speech RMS; then identical sample-peak headroom to <=−3 dBFS. No LUFS normalization, limiting, or dynamic leveling.', 'target_speech_rms_before_common_headroom_dbfs':target,'common_headroom_gain_db':head,'files':{}}
  for k,v in scaled.items():
   assert len(v)==ns and gains[k]<=1e-9 and head<=0
   path=HERE/folder/(group+'_'+k+'_matched.wav'); v=(v*np.float32(10**(head/20))).astype(np.float32); write_float(path,v,SR)
   info['files'][k]={'path':str(path),'sha256':qa.sha(path),'sample_count':len(v),'offset_seconds':OFFSET if k=='Adobe' else 0,'original_speech_rms_dbfs':levels[k],'match_gain_db':gains[k],'total_static_gain_db':gains[k]+head,'matched_metrics':qa.signal_metrics(v,SR,(speech,pause))}
   if descriptions:info['files'][k]['role']=descriptions.get(k,'processing diagnostic')
  return info
 for name,(a,b) in regions.items():
  arrs={k:v[round(a*SR):round(b*SR)] for k,v in signals.items() if k!='Adobe'}
  if 'handle' not in name:
   arrs['Adobe']=signals['Adobe'][round((a+OFFSET)*SR):round((b+OFFSET)*SR)]
  info=matching(name,arrs,a,b,'matched')
  if 'Adobe' in arrs:info['fixed_offset_envelope_correlation_original_Adobe']=corr(qa.frames(arrs['original'],SR),qa.frames(arrs['Adobe'],SR))
  else:info['Adobe_excluded']='No verified equivalent whole handle: leading clip includes negative reference time and extra faint original content; trailing region follows later edits and reference omits original trailing content. No padding/substitute segment.'
  out['excerpts'][name]=info
 # Stage comparisons use the same source masks. These are saved stems and algebraic gain replay, not new enhancement/model candidates.
 bdir=R3/'diagnostics-bounded-tail'; rdir=TRIAL/'finnegan-repair-dry20'
 stages={'original':x,'B_cleanup':load(bdir/'cleanup.wav'),'model_cleanup_dry20':load(rdir/'cleanup.wav'), 'B_presence':load(bdir/'presence.wav'),'model_presence':load(rdir/'presence.wav'), 'B_levelled':load(bdir/'levelled.wav'),'model_levelled':load(rdir/'levelled.wav'), 'model_raw_rejected':load(TRIAL/'finnegan-primary/model_raw.wav')}
 gains=np.load(bdir/'level_frame_gain_db.npy'); centers=np.arange(len(gains))*.02+.01
 curve=np.interp(np.arange(n)/SR,centers,gains,left=gains[0],right=gains[-1])
 replay=(stages['model_presence']*10**(curve/20)).astype(np.float32)
 stages['model_presence_frozen_B_gain']=replay
 verify=(stages['B_presence']*10**(curve/20)).astype(np.float32)
 assert np.max(abs(verify-stages['B_levelled']))<1e-6
 for k,v in stages.items():
  assert len(v)==n
  out['stage_diagnostics'][k]={'full_source_mask_metrics':qa.signal_metrics(v,SR,masks),'speech_window_gains':qa.speech_window_gain(x,v,SR)}
  if k not in ['original','model_presence_frozen_B_gain']:
   stem=(TRIAL/'finnegan-primary/model_raw.wav') if k=='model_raw_rejected' else ((bdir/(k.removeprefix('B_')+'.wav')) if k.startswith('B_') else (rdir/(('cleanup' if k=='model_cleanup_dry20' else k.removeprefix('model_'))+'.wav')))
   out['stage_diagnostics'][k].update(path=str(stem),sha256=qa.sha(stem))
 replaypath=HERE/'diagnostics/model_presence_frozen_B_level_gain.wav';write_float(replaypath,replay,SR)
 out['stage_diagnostics']['model_presence_frozen_B_gain'].update(path=str(replaypath),sha256=qa.sha(replaypath),role='Diagnostic replay of EXACT saved B leveling curve onto saved repair presence stem. Omits compression/mastering. Isolates leveling choice partially; preceding cleanup/pause/adaptive presence still differ. Not a candidate.')
 out['B_gain_replay_validation']={'saved_gain_path':str(bdir/'level_frame_gain_db.npy'),'sha256':qa.sha(bdir/'level_frame_gain_db.npy'),'max_abs_B_replay_error':float(np.max(abs(verify-stages['B_levelled'])))}
 for name in ['target_phrase','quiet_articulation','leading_handle_context','trailing_handle_context']:
  a,b=regions[name]
  keys=['original','B_cleanup','model_cleanup_dry20','model_raw_rejected'] if 'handle' in name else ['B_presence','model_presence','B_levelled','model_levelled','model_presence_frozen_B_gain']
  out.setdefault('diagnostic_excerpts',{})[name]=matching(name,{k:stages[k][round(a*SR):round(b*SR)] for k in keys},a,b,'diagnostics')
 # Partial handle content ASR below came from read-only original MP3 inspection; not verified words.
 out['rejection_diagnosis']={'primary_rejected':True,'primary_watchdog':primary['watchdog'],'origin':'Raw model suppresses protected periodic/modulated low-level signal; highpass preserved it. Downstream leveling partly restores but cannot recover erased articulation. Semantic speech cannot be proven by mask.', 'raw_worst_relative_gain_db':out['stage_diagnostics']['model_raw_rejected']['speech_window_gains']['worst_speech_window_gain_relative_db'],'primary_final_worst_relative_gain_db':primary['metrics']['worst_speech_window_gain_relative_db'],'repair_final_worst_relative_gain_db':repair['metrics']['worst_speech_window_gain_relative_db'],'readonly_original_handle_ASR_proxy':{'0–8s':"Let's see if everyone's supercane. So some more criminal justice terminology.",'137–143.49s':"So, I've put a lot of work into this."},'ASR_caveat':'Context-limited cached small-model transcripts can hallucinate faint speech. Not verified human words; full-file and prior B ASR differ.'}
 (HERE/'comparison.json').write_text(json.dumps(out,indent=2))
 print('Matched clips, diagnostics, fixed-mask evidence saved',flush=True)
 asr={'model':str(MODEL),'model_sha256':qa.sha(MODEL),'whisper':str(WHISPER),'whisper_sha256':qa.sha(WHISPER),'settings':['-l','en','-t','4','-nf'],'note':'Original transcript is an ASR proxy, not verified truth. Differences are NOT WER, identity, or quality. Adobe is edited; missing/filler/reordered words confound full-file differences. Analysis-only16kPCM does not enter enhancement.','files':{}}
 for k,p in paths.items():
  wav=HERE/'asr'/(k+'_analysis16k.wav'); prefix=HERE/'asr'/k
  qa.run(['ffmpeg','-y','-v','error','-i',p,'-ac','1','-ar','16000','-c:a','pcm_s16le',wav])
  cmd=[str(WHISPER),'-m',str(MODEL),'-f',str(wav),'-l','en','-t','4','-nf','-otxt','-of',str(prefix)]
  res=subprocess.run(cmd,capture_output=True);(HERE/'asr'/(k+'.log')).write_bytes(res.stdout+b'\nSTDERR\n'+res.stderr);res.check_returncode()
  txt=prefix.with_suffix('.txt').read_text();asr['files'][k]={'transcript':txt,'transcript_path':str(prefix.with_suffix('.txt')),'analysis_wav_sha256':qa.sha(wav),'command':cmd,'log_path':str(HERE/'asr'/(k+'.log')),'target_phrase_present':['reading anything' in txt.lower(),'criminal law' in txt.lower(),'more simple' in txt.lower()]}
  print('ASR complete',k,flush=True)
 ref=re.findall(r"[a-z]+(?:'[a-z]+)?",asr['files']['original']['transcript'].lower())
 for v in asr['files'].values():
  words=re.findall(r"[a-z]+(?:'[a-z]+)?",v['transcript'].lower());v['word_count']=len(words);v['word_edit_distance_to_original_proxy']=edit_distance(ref,words);v['word_difference_rate_to_original_proxy']=v['word_edit_distance_to_original_proxy']/max(1,len(ref))
 (HERE/'asr-comparison.json').write_text(json.dumps(asr,indent=2))
 print('Done',flush=True)
if __name__=='__main__':run()
