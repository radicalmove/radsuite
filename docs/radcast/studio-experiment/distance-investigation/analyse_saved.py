"""Diagnose the existing preservation blend. No model inference or new preset.

Read verified original-derived float stems; export static audition copies only.
Refuse to replace a prior run. Residuals are waveform differences, not reverb.
"""
import json
import sys
from pathlib import Path
import numpy as np
import soundfile as sf

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[3]
sys.path.insert(0, str(ROOT / 'tools/radcast'))
import analysis as qa
import studio
from model_trial import si_sdr

MODEL = ROOT / 'docs/radcast/studio-experiment/model-feasibility/treble-ism-eng120-epoch119'
RATE = 48000
REGIONS = dict(target_phrase=(20, 32), exact_words=(23.47, 28.38),
               quiet_articulation=(19.5, 22.5), louder_passage=(70, 80),
               phrase_ending=(38.5, 41.8))


def read_record(record):
    path = Path(record['path'])
    assert qa.sha(path) == record['sha256'], str(path)
    x, rate = sf.read(path, dtype='float32')
    assert rate == RATE and x.ndim == 1 and np.isfinite(x).all()
    assert len(x) == record['sample_count']
    assert sf.info(path).subtype == 'FLOAT'
    return x


def difference(raw, blended):
    """Least-squares scalar removes volume; residual has no source attribution."""
    x = raw.astype('float64'); y = blended.astype('float64')
    alpha = float(np.dot(x, y) / np.dot(x, x))
    residual = y - alpha * x
    fraction = float(np.linalg.norm(residual) / np.linalg.norm(alpha * x))
    return dict(fitted_static_gain_db=qa.db(alpha),
                waveform_correlation=float(np.corrcoef(x, y)[0, 1]),
                difference_rms_fraction_after_scalar=fraction,
                difference_rms_db_after_scalar=qa.db(fraction))


def run():
    assert not (HERE / 'analysis.json').exists(), 'Preserve prior analysis'
    assert not (HERE / 'listening-rms').exists(), 'Preserve prior clips'
    baseline_path = MODEL / 'finnegan-repair-dry20/trial.qa.json'
    baseline = json.loads(baseline_path.read_text())
    primary = json.loads((MODEL / 'finnegan-primary/trial.qa.json').read_text())
    assert baseline['explicit_dry_blend'] == .2 and not baseline['watchdog']
    assert baseline['accepted_for_listening'] and not primary['accepted_for_listening']
    names = ('prepared', 'highpassed', 'model_raw', 'cleanup')
    audio = {k: read_record(baseline['paths'][k]) for k in names}
    assert baseline['paths']['model_raw']['sha256'] == primary['paths']['model_raw']['sha256']
    assert qa.sha(primary['paths']['model_raw']['path']) == baseline['paths']['model_raw']['sha256']
    assert len({len(x) for x in audio.values()}) == 1
    expected = (.8 * audio['model_raw'] + .2 * audio['highpassed']).astype('float32')
    replay_error = float(np.max(abs(audio['cleanup'] - expected)))
    assert replay_error == 0
    speech, pause = qa.masks(audio['prepared'], RATE)
    sample_mask = np.repeat(speech, 960)
    result = dict(date='2026-10-09', sample_rate=RATE, full_sample_count=len(expected),
        new_model_inferences=0, new_full_candidates=0, native_application_changed=False,
        baseline_manifest=dict(path=str(baseline_path), sha256=qa.sha(baseline_path)),
        inputs={k: baseline['paths'][k] for k in names}, exact_blend_replay_max_error=replay_error,
        rejected_unblended_master='Preserved; raw-stem clips below are diagnostics only, not approved full outputs.',
        definitions=dict(difference='Blended minus least-squares-scaled model. Includes voice, noise, room, phase and processing changes; not isolated reverberation.',
          mask='Unchanged full-original-derived protected 20ms energy proxy; not semantic VAD.',
          attribution='Finite gain-normalized correlation cannot establish apparent distance or Adobe algorithm.'),
        regions={}, known_clean_controls={})
    rms_dir = HERE / 'listening-rms'; rms_dir.mkdir()
    for label, (start, end) in REGIONS.items():
        cut = slice(round(start * RATE), round(end * RATE))
        frame_cut = slice(round(start * 50), round(end * 50))
        sm = (speech[frame_cut], pause[frame_cut])
        xs = {k: x[cut] for k, x in audio.items()}
        m = sample_mask[cut]
        metrics = {k: qa.signal_metrics(x, RATE, sm) for k, x in xs.items()}
        r = dict(original_timeline_seconds=[start, end],
            blend_difference=difference(xs['model_raw'][m], xs['cleanup'][m]),
            original_to_model_difference=difference(xs['highpassed'][m], xs['model_raw'][m]),
            spectral_share_change_db={k: metrics['cleanup']['bands_db_relative'][k] - metrics['model_raw']['bands_db_relative'][k] for k in qa.BANDS},
            stage_metrics=metrics)
        result['regions'][label] = r
        if label == 'exact_words': continue
        clips = {'model_unblended': xs['model_raw'], 'original20': xs['cleanup']}
        levels = {k: qa.signal_metrics(x, RATE, sm)['speech_rms_dbfs'] for k, x in clips.items()}
        # Cleanup stems precede speech leveling and are quiet. Use only a
        # static audition scalar, then common headroom; never modify stems.
        target = -26.5; gains = {k: target - v for k, v in levels.items()}
        peak = max(np.max(abs(x * 10 ** (gains[k] / 20))) for k, x in clips.items())
        head = min(0., -3 - qa.db(peak)); files = {}
        for k, x in clips.items():
            gain = gains[k] + head
            y = (x * 10 ** (gain / 20)).astype('float32')
            path = rms_dir / f'{label}_{k}.wav'; studio.write_float(path, y, RATE)
            actual = qa.signal_metrics(y, RATE, sm)['speech_rms_dbfs']
            assert abs(actual - (target + head)) < 1e-5
            assert len(y) == round((end - start) * RATE) and qa.db(np.max(abs(y))) <= -3 + 1e-5
            files[k] = dict(path=str(path), sha256=qa.sha(path), sample_count=len(y),
                static_gain_db=gain, speech_rms_dbfs=actual, loudness=qa.loudness(path))
        r['rms_listening'] = files
        if label == 'target_phrase':
            lufs_dir = HERE / 'listening-lufs'; lufs_dir.mkdir()
            target_lufs = min(f['loudness']['integrated_lufs'] for f in files.values())
            lufs_files = {}
            for k, record in files.items():
                y, _ = sf.read(record['path'], dtype='float32')
                gain = target_lufs - record['loudness']['integrated_lufs']; assert gain <= 0
                y = (y * 10 ** (gain / 20)).astype('float32')
                path = lufs_dir / f'target_phrase_{k}.wav'; studio.write_float(path, y, RATE)
                lufs_files[k] = dict(path=str(path), sha256=qa.sha(path), sample_count=len(y),
                    total_static_gain_db=record['static_gain_db'] + gain, loudness=qa.loudness(path))
            spread = max(r['loudness']['integrated_lufs'] for r in lufs_files.values()) - min(r['loudness']['integrated_lufs'] for r in lufs_files.values())
            assert spread <= .02
            r['lufs_listening'] = dict(files=lufs_files, measured_spread_lu=spread)
    controls_path = MODEL / 'controls/controls.json'
    controls = json.loads(controls_path.read_text())
    result['controls_manifest'] = dict(path=str(controls_path), sha256=qa.sha(controls_path))
    for speaker, record in controls['speakers'].items():
        xs = {k: read_record(v) for k, v in record['paths'].items()}
        clean = xs['clean']; raw = xs['synthetic_room_model']; room = xs['synthetic_room']
        score = si_sdr(clean, raw)
        assert abs(score - record['paired_room']['output_si_sdr_db']) < 1e-10
        blend = (.8 * raw + .2 * room).astype('float32')
        blended_score = si_sdr(clean, blend)
        result['known_clean_controls'][speaker] = dict(inputs=record['paths'],
            raw_model_si_sdr_db=score, original20_si_sdr_db=blended_score,
            change_db=blended_score - score, input_room_si_sdr_db=si_sdr(clean, room),
            note='One previously declared synthetic room; no new inference. Paired signal error includes all distortions, not isolated reverb or listening quality.')
    verification = json.loads((ROOT / 'docs/radcast/studio-experiment/presence-probe/verification.json').read_text())
    result['reference_integrity'] = verification['reference_integrity']
    for record in result['reference_integrity'].values():
        assert qa.sha(record['path']) == record['sha256']
    result['code_sha256'] = {str(p): qa.sha(p) for p in (Path(__file__), Path(qa.__file__), Path(studio.__file__), ROOT / 'tools/radcast/model_trial.py')}
    (HERE / 'analysis.json').write_text(json.dumps(result, indent=2, allow_nan=False) + '\n')
    print(json.dumps(dict(regions={k: v['blend_difference'] for k, v in result['regions'].items()},
        controls={k: {s: r[s] for s in ('raw_model_si_sdr_db', 'original20_si_sdr_db', 'change_db')} for k, r in result['known_clean_controls'].items()},
        exact_blend_replay_max_error=replay_error, reference_hashes_unchanged=True), indent=2))


if __name__ == '__main__': run()
