"""ASR only, exact prior flags; retains earlier timestamp-enabled diagnostic run."""
import json,re,subprocess
import compare_saved as c
q=c.qa;here=c.HERE;folder=here/'asr-prior-settings';folder.mkdir(exist_ok=False)
inputs=json.loads((here/'comparison.json').read_text())['inputs']
o={'model':str(c.MODEL),'model_sha256':q.sha(c.MODEL),'whisper':str(c.WHISPER),'whisper_sha256':q.sha(c.WHISPER),'settings':['-l','en','-t','4','-nf','-nt','-otxt'],'note':'Exact prior asr_compare.py flags. Original is an unverified ASR proxy; differences are not WER, preservation truth, identity, or listening quality. Adobe edits confound full-file differences. Earlier asr-comparison.json omitted -nt and is retained as a separately labelled diagnostic; do not merge scores. No enhancement run.','files':{}}
for k,v in inputs.items():
 wav=here/'asr'/(k+'_analysis16k.wav');prefix=folder/k
 cmd=[str(c.WHISPER),'-m',str(c.MODEL),'-f',str(wav),'-l','en','-t','4','-nf','-nt','-otxt','-of',str(prefix)]
 res=subprocess.run(cmd,capture_output=True);(folder/(k+'.log')).write_bytes(res.stdout+b'\nSTDERR\n'+res.stderr);res.check_returncode()
 txt=prefix.with_suffix('.txt').read_text();o['files'][k]={'transcript':txt,'transcript_path':str(prefix.with_suffix('.txt')),'transcript_sha256':q.sha(prefix.with_suffix('.txt')),'analysis_wav_path':str(wav),'analysis_wav_sha256':q.sha(wav),'command':cmd,'log_path':str(folder/(k+'.log')),'target_phrase_present':['reading anything' in txt.lower(),'criminal law' in txt.lower(),'more simple' in txt.lower()]}
 print('Exact prior ASR complete',k,flush=True)
ref=re.findall(r"[a-z]+(?:'[a-z]+)?",o['files']['original']['transcript'].lower())
for v in o['files'].values():
 words=re.findall(r"[a-z]+(?:'[a-z]+)?",v['transcript'].lower());v['word_count']=len(words);v['word_edit_distance_to_original_proxy']=c.edit_distance(ref,words);v['word_difference_rate_to_original_proxy']=v['word_edit_distance_to_original_proxy']/max(1,len(ref))
(here/'asr-prior-settings.json').write_text(json.dumps(o,indent=2));print('Done',flush=True)
