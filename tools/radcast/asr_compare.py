"""Optional offline ASR QA: original transcript is a proxy, not ground truth.
Analysis-only 16 kHz conversion does not enter Studio's processing pipeline.
"""
import argparse
import json
import re
import tempfile
from pathlib import Path
import analysis as qa

def edit_distance(a,b):
    row=list(range(len(b)+1))
    for i,x in enumerate(a,1):
        new=[i]
        for j,y in enumerate(b,1):new.append(min(new[-1]+1,row[j]+1,row[j-1]+(x!=y)))
        row=new
    return row[-1]

def main():
    p=argparse.ArgumentParser();p.add_argument('--manifest',required=True);p.add_argument('--model',required=True);p.add_argument('--whisper',default='whisper-cli');p.add_argument('--out',required=True)
    args=p.parse_args();paths=json.loads(Path(args.manifest).read_text());results={}
    with tempfile.TemporaryDirectory(prefix='radcast-asr-') as d:
        for name,path in paths.items():
            folder=Path(d);wav=folder/(name+'.wav');prefix=folder/name
            qa.run(['ffmpeg','-y','-v','error','-i',path,'-ac','1','-ar','16000','-c:a','pcm_s16le',wav])
            qa.run([args.whisper,'-m',args.model,'-f',wav,'-l','en','-t','4','-nf','-nt','-otxt','-of',prefix])
            transcript=prefix.with_suffix('.txt').read_text()
            words=re.findall(r"[a-z]+(?:'[a-z]+)?",transcript.lower())
            results[name]=dict(transcript=transcript,words=words)
    original=results['original']['words']
    for name,r in results.items():
        r['word_edit_distance_to_original']=edit_distance(original,r['words'])
        r['word_difference_rate_to_original']=r['word_edit_distance_to_original']/max(1,len(original))
        r['word_count']=len(r.pop('words'))
    Path(args.out).write_text(json.dumps(dict(model=str(Path(args.model).resolve()),model_sha256=qa.sha(args.model),
        note='Same local model/settings. Original ASR is not a verified transcript; difference rate is NOT measured WER or speaker identity. ASR can miss a lisp/warbling.',files=results),indent=2))
    print({k:dict(count=v['word_count'],difference=v['word_difference_rate_to_original']) for k,v in results.items()})

if __name__=='__main__':main()
