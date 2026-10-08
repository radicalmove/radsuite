"""Same-settings cached ASR for the sole repaired candidate; analysis only."""
import json
from pathlib import Path
import re
import subprocess
import sys
import tempfile
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[3]
sys.path.insert(0,str(ROOT/'tools/radcast'))
import analysis as qa
from asr_compare import edit_distance


def run(report_saved=False):
    trial=json.loads((HERE/'trial.qa.json').read_text())
    assert trial['accepted_for_listening'] and not trial['watchdog'] and not trial['watchdog_against_baseline']
    baseline=Path(trial['baseline_report']).parents[1]/'comparison/asr-prior-settings.json'
    old=json.loads(baseline.read_text())
    whisper=Path('/opt/homebrew/bin/whisper-cli');model=Path('/Users/rcd58/.radcast/whispercpp-models/ggml-small.bin')
    assert qa.sha(model)==old['model_sha256']
    presence=json.loads((ROOT/'docs/radcast/studio-experiment/presence-probe/asr-comparison.json').read_text())
    assert qa.sha(whisper)==presence['binary_sha256']
    assert qa.sha(trial['output'])==trial['output_sha256']
    assert not (HERE/'asr-comparison.json').exists()
    folder=HERE/'asr'
    wav=folder/'adaptive_analysis16k.wav';prefix=folder/'adaptive'
    cmd=[str(whisper),'-m',str(model),'-f',str(wav),'-l','en','-t','4','-nf','-nt','-otxt','-of',str(prefix)]
    if report_saved:
        # Recover metadata only from the single already completed recognition.
        # Verify its analysis input against the unchanged full master.
        assert folder.is_dir()
        log=(folder/'adaptive.log').read_text()
        assert 'output_txt: saving output' in log and 'whisper_print_timings:    total time' in log
        with tempfile.TemporaryDirectory(prefix='radcast-asr-input-check-') as temp:
            check=Path(temp)/'check.wav'
            qa.run(['ffmpeg','-v','error','-i',trial['output'],'-ac','1','-ar','16000','-c:a','pcm_s16le',check])
            assert qa.sha(check)==qa.sha(wav)
    else:
        assert not folder.exists();folder.mkdir()
        qa.run(['ffmpeg','-v','error','-i',trial['output'],'-ac','1','-ar','16000','-c:a','pcm_s16le',wav])
        p=subprocess.run(cmd,capture_output=True);(folder/'adaptive.log').write_bytes(p.stdout+b'\nSTDERR\n'+p.stderr);p.check_returncode()
    text=prefix.with_suffix('.txt').read_text();words=lambda t:re.findall(r"[a-z]+(?:'[a-z]+)?",t.lower())
    original=words(old['files']['original']['transcript']);new=words(text);edits=edit_distance(original,new)
    result=dict(command=cmd,model=str(model),model_sha256=qa.sha(model),binary=str(whisper),binary_sha256=qa.sha(whisper),
        original_proxy_path=str(baseline),original_proxy_sha256=qa.sha(baseline),analysis_copy_sha256=qa.sha(wav),
        input_master_sha256=trial['output_sha256'],transcript=text,word_count=len(new),original_proxy_words=len(original),
        proxy_edits=edits,proxy_edit_rate=edits/len(original),baseline_proxy_edits=old['files']['model_dry20']['word_edit_distance_to_original_proxy'],
        transcript_path=str(prefix.with_suffix('.txt')),transcript_sha256=qa.sha(prefix.with_suffix('.txt')),
        log_path=str(folder/'adaptive.log'),log_sha256=qa.sha(folder/'adaptive.log'),
        target_phrase_present='reading anything to do with the criminal law a lot more simple' in ' '.join(new),
        introductory_words_present='some more criminal justice terminology' in ' '.join(new),
        metadata_recovered_from_saved_run=report_saved,
        recovery_note='Recognition completed successfully once. Initial JSON finalization raised KeyError: proxy_edits because the historical baseline names that field word_edit_distance_to_original_proxy. Saved input, transcript and log retained; no recognition rerun.' if report_saved else None,
        note='Original cached ASR is not verified ground truth. Proxy edits do not rank dryness, naturalness, identity or lisp.16k copy is analysis-only.')
    (HERE/'asr-comparison.json').write_text(json.dumps(result,indent=2)+'\n')
    print(json.dumps(dict(proxy_edits=edits,original_proxy_words=len(original),word_count=len(new),introductory_words_present=result['introductory_words_present'],target_phrase_present=result['target_phrase_present']),indent=2),flush=True)


if __name__=='__main__':
    import argparse
    p=argparse.ArgumentParser();p.add_argument('--report-saved',action='store_true')
    run(p.parse_args().report_saved)
