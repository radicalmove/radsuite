"""Bounded native-48k USES controls; no downloads, presets, retries or repairs.

The learned processors are instantiated only by run_controls. Unit tests inject
simple signal transforms, and never load or execute a learned model.
"""
import argparse
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import resource
import sys
import time

import numpy as np
import soundfile as sf

import analysis as qa
import model_trial
import uses_trial


RATE = 48000
CONDITIONS = ("clean", "early", "late", "composite")
CONTROL_MANIFEST_SHA256 = "13286c07a6080da71b54a53e0d3f139e18b9b90b196ccbaa3a6765fb8986ccf9"
ROOM_BENEFIT_DB = 1.0
EARLY_OVER_TREBLE_DB = 1.0
LATE_MINIMUM_VS_TREBLE_DB = -1.0
RIR = dict(synthetic_not_measured_room=True, seed=20261009, direct_gain=1,
           early_taps_seconds=[.014, .037], early_tap_gains=[.25, .12],
           late_start_seconds=.05, late_decay_parameter_seconds=.65,
           late_rir_l2_norm=.35, late_tap_density=.05, rir_length_seconds=.8,
           tail_truncated_to_source_sample_count=True)


def fresh_directory(folder):
    folder = Path(folder)
    if folder.exists():
        raise ValueError("Use a fresh evidence directory; historical files are preserved")
    folder.mkdir(parents=True, exist_ok=False)
    return folder


def load_verified_controls(manifest_path, expected_sha256=CONTROL_MANIFEST_SHA256):
    """Verify the complete old manifest and all files used as input/evidence."""
    manifest_path = Path(manifest_path)
    if qa.sha(manifest_path) != expected_sha256:
        raise ValueError("Controls manifest hash changed")
    manifest = json.loads(manifest_path.read_text())
    source_manifest = Path(manifest["source_manifest"])
    if qa.sha(source_manifest) != manifest["source_manifest_sha256"]:
        raise ValueError("Clean source manifest hash changed")
    for source in json.loads(source_manifest.read_text())["files"]:
        if qa.sha(source["path"]) != source["sha256"]:
            raise ValueError(f"Licensed source input hash changed: {source['path']}")
    if set(manifest["speakers"]) != {"p232", "p257"}:
        raise ValueError("The controls require the two pinned CSTR speakers")
    audio = {}
    for speaker, record in manifest["speakers"].items():
        if record["synthetic_rir"] != RIR:
            raise ValueError("Saved synthetic room coefficients differ from the approved controls")
        audio[speaker] = {}
        for name in ("clean", "synthetic_room", "clean_model", "synthetic_room_model"):
            artifact = record["paths"][name]
            if qa.sha(artifact["path"]) != artifact["sha256"]:
                raise ValueError(f"Controls input hash changed: {speaker}/{name}")
            samples, rate = sf.read(artifact["path"], dtype="float32")
            samples = uses_trial.validate_audio(samples, rate)
            if len(samples) != artifact["sample_count"]:
                raise ValueError(f"Saved control sample count changed: {speaker}/{name}")
            audio[speaker][name] = samples
        if len({len(x) for x in audio[speaker].values()}) != 1:
            raise ValueError(f"Saved control counts are not paired: {speaker}")
    return manifest, audio


def prepare_inputs(clean, saved_composite):
    clean = uses_trial.validate_audio(clean, RATE)
    saved_composite = uses_trial.validate_audio(saved_composite, RATE)
    if clean.shape != saved_composite.shape:
        raise ValueError("Clean and saved composite must have identical sample counts")
    reconstructed, _ = model_trial.synthetic_room(clean, RATE)
    if float(np.max(abs(reconstructed - saved_composite))) > 2e-7:
        raise ValueError("Saved composite does not match the declared room and common source gain")
    early, late = uses_trial.room_conditions(clean, RATE)
    signals = {"clean": clean, "early": early, "late": late, "composite": saved_composite}
    peak = max(float(abs(x).max()) for x in signals.values())
    gain = min(1.0, .8 / max(peak, 1e-12))
    return {key: (x * gain).astype("float32") for key, x in signals.items()}, gain


def select_verified_prefix(audio, boundaries, first_utterances=2):
    """Select complete utterances after verifying every original file/hash."""
    if isinstance(first_utterances, bool) or first_utterances != 2:
        raise ValueError("Approved controls use the first two complete utterances")
    count = len(audio["clean"])
    if len(boundaries) < first_utterances:
        raise ValueError("Source boundaries do not contain two complete utterances")
    offset = 0
    for boundary in boundaries:
        start = boundary.get("start_sample")
        samples = boundary.get("samples")
        if (isinstance(start, bool) or not isinstance(start, int) or start != offset
                or isinstance(samples, bool) or not isinstance(samples, int) or samples <= 0
                or not isinstance(boundary.get("filename"), str)
                or not isinstance(boundary.get("transcript"), str)):
            raise ValueError("Invalid or discontinuous source boundaries")
        offset += samples
    if offset != count or any(len(x) != count for x in audio.values()):
        raise ValueError("Source boundaries do not cover the paired full source sample counts")
    selected_boundaries = boundaries[:first_utterances]
    end = sum(boundary["samples"] for boundary in selected_boundaries)
    return ({name: samples[:end].copy() for name, samples in audio.items()},
            dict(first_utterances=first_utterances, start_sample=0, end_sample=end,
                 sample_count=end, full_source_sample_count=count, duration_seconds=end / RATE,
                 source_boundaries=selected_boundaries,
                 context="First two complete licensed utterances; no midword cut or inference segmentation"))


def preservation_metrics(reference, output):
    """Unchanged watchdog on source masks and a static gain-matched analysis copy.

    The saved output is untouched. Spectrum shares, local gain and normalized
    envelope keep their existing rules; global model gain is reported separately.
    """
    masks = qa.masks(reference, RATE)
    source = qa.signal_metrics(reference, RATE, masks)
    raw = qa.signal_metrics(output, RATE, masks)
    alignment = qa.alignment(reference, output, RATE)
    before = source["speech_rms_dbfs"]
    after = raw["speech_rms_dbfs"]
    compensation = (before - after) if before is not None and after is not None else 0.0
    gain = 10 ** (compensation / 20)
    normalized = qa.signal_metrics(output * gain, RATE, masks)
    normalized.update(alignment)
    # Clipping belongs to actual PCM, never to the gain-matched analysis copy.
    normalized["clipped_samples"] = raw["clipped_samples"]
    normalized["sample_peak_dbfs"] = raw["sample_peak_dbfs"]
    return dict(source_metrics=source, raw_output_metrics=raw,
                normalized_output_metrics=normalized,
                analysis_gain_compensation_db=float(compensation),
                alignment=alignment, watchdog=qa.watchdog(source, normalized),
                eligibility="Paired source-frame acoustic proxy; no transcript, speaker-identity or listening certification")


def prerequisite_gate(speakers):
    reasons = []
    if not speakers:
        reasons.append("No completed speaker controls")
    for speaker, record in speakers.items():
        conditions = record.get("conditions", {})
        for name in CONDITIONS:
            condition = conditions.get(name)
            if condition is None or "paired" not in condition:
                reasons.append(f"{speaker}/{name}: incomplete control")
                continue
            if condition.get("technical_issues"):
                reasons.append(f"{speaker}/{name}: {condition['technical_issues']}")
        clean = conditions.get("clean", {})
        if "preservation" not in clean or clean["preservation"].get("watchdog"):
            reasons.append(f"{speaker}/clean: preservation watchdog is not clear")
        for name in ("early", "late"):
            improvement = conditions.get(name, {}).get("paired", {}).get("improvement_db")
            if improvement is None or not np.isfinite(improvement) or improvement < ROOM_BENEFIT_DB:
                reasons.append(f"{speaker}/{name}: paired improvement below {ROOM_BENEFIT_DB:.1f} dB")
    return dict(passed=not reasons, reasons=reasons,
                minimum_early_and_late_improvement_db=ROOM_BENEFIT_DB)


def qualification_gate(speakers):
    prerequisite = prerequisite_gate(speakers)
    reasons = list(prerequisite["reasons"])
    comparisons = {}
    for speaker, record in speakers.items():
        baseline = record.get("new_treble", {}).get("conditions", {})
        comparisons[speaker] = {}
        for name, floor in (("early", EARLY_OVER_TREBLE_DB), ("late", LATE_MINIMUM_VS_TREBLE_DB)):
            trial = record.get("conditions", {}).get(name, {})
            current = baseline.get(name, {})
            a = trial.get("paired", {}).get("output_si_sdr_db")
            b = current.get("paired", {}).get("output_si_sdr_db")
            if a is None or b is None or not np.isfinite(a) or not np.isfinite(b):
                reasons.append(f"{speaker}/{name}: no completed same-input Treble comparison")
                continue
            delta = a - b
            comparisons[speaker][name] = dict(uses_minus_treble_si_sdr_db=delta, minimum_db=floor)
            if current.get("technical_issues"):
                reasons.append(f"{speaker}/Treble/{name}: {current['technical_issues']}")
            if delta < floor:
                reasons.append(f"{speaker}/{name}: USES minus Treble below {floor:.1f} dB")
    return dict(passed=not reasons, reasons=reasons, comparisons=comparisons,
                thresholds=dict(early_and_late_vs_untreated_db=ROOM_BENEFIT_DB,
                                early_vs_treble_db=EARLY_OVER_TREBLE_DB,
                                late_minimum_vs_treble_db=LATE_MINIMUM_VS_TREBLE_DB),
                scope="Experimental synthetic-room evidence only; a pass permits one Finnegan trial, not voice-quality approval")


def _save_audio(path, audio):
    """Save a float working stage before any analysis can fail."""
    import studio
    audio = np.asarray(audio)
    if audio.ndim in (1, 2) and len(audio):
        if path.exists():
            raise ValueError(f"Refusing to overwrite stage: {path}")
        sf.write(path, audio, RATE, subtype="FLOAT")
        studio.canonicalize_wav(path)
    else:
        path = path.with_suffix(".npy")
        if path.exists():
            raise ValueError(f"Refusing to overwrite stage: {path}")
        np.save(path, audio, allow_pickle=False)
    return dict(path=str(path.resolve()), sha256=qa.sha(path),
                pcm_sha256=hashlib.sha256(np.asarray(audio, dtype="<f4").tobytes()).hexdigest(),
                sample_count=len(audio) if audio.ndim else 0, shape=list(audio.shape),
                sample_rate=RATE, finite=bool(np.isfinite(audio).all()),
                dtype=str(audio.dtype), format="float32 WAV" if path.suffix == ".wav" else "NumPy array")


def _assess(reference, input_audio, output):
    output = np.asarray(output)
    issues = []
    if output.ndim != 1:
        issues.append("mono_shape")
    if output.shape != reference.shape:
        issues.append("sample_count")
    if not output.size or not np.isfinite(output).all():
        issues.append("empty_or_nonfinite")
    if output.dtype != np.float32:
        issues.append("working_dtype")
    if output.size and np.any(abs(output) >= 1):
        issues.append("clipping")
    result = dict(technical_issues=issues)
    if issues:
        return result
    preservation = preservation_metrics(reference, output)
    alignment = preservation["alignment"]
    if abs(alignment["lag_seconds"] or 0) > qa.THRESHOLDS["lag_seconds"]:
        issues.append("timing")
    if (alignment["block_lag_spread_seconds"] or 0) > qa.THRESHOLDS["block_lag_spread_seconds"]:
        issues.append("timing_drift")
    input_score = model_trial.si_sdr(reference, input_audio)
    output_score = model_trial.si_sdr(reference, output)
    result.update(preservation=preservation,
                  paired=dict(input_si_sdr_db=input_score, output_si_sdr_db=output_score,
                              improvement_db=output_score - input_score))
    return result


def run_controls(manifest_path, evidence_dir, model_dir, *, treble_model_dir=None,
                 expected_manifest_sha256=CONTROL_MANIFEST_SHA256,
                 model_factory=None, treble_factory=None, first_utterances=2):
    manifest, old_audio = load_verified_controls(manifest_path, expected_manifest_sha256)
    selections = {}
    for speaker, audio in old_audio.items():
        old_audio[speaker], selections[speaker] = select_verified_prefix(
            audio, manifest["speakers"][speaker]["source_boundaries"], first_utterances)
    prepared = {speaker: prepare_inputs(audio["clean"], audio["synthetic_room"])
                for speaker, audio in old_audio.items()}
    folder = fresh_directory(evidence_dir)
    result = dict(schema_version=1, status="prepared", accepted_for_finnegan=False,
                  source_manifest=str(Path(manifest_path).resolve()),
                  source_manifest_sha256=expected_manifest_sha256,
                  first_utterances=first_utterances,
                  synthetic_rir=RIR, inference_counts=dict(uses=0, new_treble=0),
                  runtime=dict(executable=sys.executable, platform=sys.platform),
                  source_code_sha256={name: qa.sha(Path(__file__).parent / name) for name in
                                      ("uses_controls.py", "uses_trial.py", "analysis.py", "model_trial.py")},
                  qualification=dict(passed=False, reasons=["Controls have not completed"]),
                  speakers={}, failures=[], events=[],
                  limitations=["Clean recordings are licensed natural speech, not asserted perfectly anechoic.",
                               "Energy masks and normalized watchdogs are acoustic proxies; legitimate room changes can trigger spectral warnings.",
                               "Only clean-control watchdog warnings gate preservation; all condition clipping/count/finite/timing failures gate qualification.",
                               "SI-SDR is paired signal error, not perceptual distance, articulation or speaker identity.",
                               "Historical Treble outputs were inferred on the full old control then sliced; rescaling/slicing does not establish a new prefix model run.",
                               "New Treble early/late comparisons run only if all eight USES controls pass prerequisites."])

    def persist():
        temporary = folder / "controls.json.tmp"
        temporary.write_text(json.dumps(result, indent=2, allow_nan=False) + "\n")
        temporary.replace(folder / "controls.json")

    def progress(message):
        event = dict(timestamp_utc=datetime.now(timezone.utc).isoformat(), message=message)
        result["events"].append(event)
        print(f"{event['timestamp_utc']} {message}", flush=True)
        persist()

    try:
        for speaker, (inputs, gain) in sorted(prepared.items()):
            old = manifest["speakers"][speaker]
            record = dict(common_additional_headroom_gain=gain,
                          prior_common_input_gain=old["common_input_gain"],
                          selection=selections[speaker],
                          source_boundaries=selections[speaker]["source_boundaries"], conditions={},
                          old_treble=dict(model=manifest["model"], conditions={},
                                          reported_paired_room=old["paired_room"],
                                          reported_paired_room_scope="Historical full-source scores; not selected-prefix comparison scores",
                                          note="Historical full-source inference then selected prefix; no new clean/composite Treble inference"))
            result["speakers"][speaker] = record
            for name, audio in inputs.items():
                masks = qa.masks(audio, RATE)
                mask_path = folder / f"{speaker}_{name}_masks.npz"
                np.savez(mask_path, speech=masks[0], pause=masks[1], sample_rate=RATE, frame_seconds=.02)
                record["conditions"][name] = dict(
                    input=_save_audio(folder / f"{speaker}_{name}_input.wav", audio),
                    masks=dict(path=str(mask_path.resolve()), sha256=qa.sha(mask_path)))
            for name, old_name in (("clean", "clean_model"), ("composite", "synthetic_room_model")):
                scaled_output = (old_audio[speaker][old_name] * gain).astype("float32")
                assessment = _assess(inputs["clean"], inputs[name], scaled_output)
                record["old_treble"]["conditions"][name] = dict(
                    artifact=old["paths"][old_name], analysis_headroom_gain=gain, **assessment)
                record["old_treble"]["conditions"][name]["selected_output"] = _save_audio(
                    folder / f"{speaker}_{name}_historical_treble_output.wav", scaled_output)
                record["old_treble"]["conditions"][name]["selection"] = selections[speaker]
        persist()
        if model_factory is None:
            processor = uses_trial.UsesModel(Path(model_dir), backend="bounded")
            proof = processor.info.get("equivalence_report_sha256")
            if (not isinstance(proof, str) or len(proof) != 64
                    or any(character.lower() not in "0123456789abcdef" for character in proof)):
                raise ValueError("Verified bounded-equivalence identity is required before controls processing")
        else:
            processor = model_factory(Path(model_dir))
        result["model"] = processor.info
        result["status"] = "running"
        progress("USES initialized; eight whole-utterance native48k controls, sequential")

        def process_case(model, speaker, name, destination, label):
            source = result["speakers"][speaker]["conditions"][name]["input"]
            if qa.sha(source["path"]) != source["sha256"]:
                raise ValueError("Prepared input hash changed before inference")
            audio, rate = sf.read(source["path"], dtype="float32")
            audio = uses_trial.validate_audio(audio, rate)
            reference = prepared[speaker][0]["clean"]
            entry = destination.setdefault(name, {})
            entry["input"] = source
            progress(f"{label} start {speaker}/{name}: {len(audio)} samples, whole input")
            start = time.perf_counter()
            result["inference_counts"][label] += 1
            try:
                output = model.cleanup(audio, RATE)
                elapsed = time.perf_counter() - start
                entry["output"] = _save_audio(folder / f"{speaker}_{name}_{label}_output.wav", output)
                entry["processing"] = dict(wall_seconds=elapsed,
                                            model_inference_seconds=getattr(model, "last_inference_seconds", elapsed),
                                            trace=getattr(model, "last_trace", None), whole_input=True)
                persist()
                entry.update(_assess(reference, audio, output))
                if label == "uses" and (entry["processing"]["trace"] or {}).get("mode") != "dereverb":
                    entry["technical_issues"].append("dereverb_mode_not_confirmed")
                if label == "uses" and (entry["processing"]["trace"] or {}).get("memory_indices") != [1]:
                    entry["technical_issues"].append("dereverb_memory_not_confirmed")
                progress(f"{label} complete {speaker}/{name}: {elapsed:.3f}s; issues={entry['technical_issues']}")
            except Exception as error:
                entry["error"] = dict(type=type(error).__name__, message=str(error))
                entry["technical_issues"] = ["inference_or_analysis_error"]
                entry.setdefault("processing", dict(wall_seconds=time.perf_counter() - start, whole_input=True))
                result["failures"].append(dict(model=label, speaker=speaker, condition=name, **entry["error"]))
                progress(f"{label} failed {speaker}/{name}: {type(error).__name__}; continuing planned matrix without retry")

        for speaker in sorted(prepared):
            for name in CONDITIONS:
                process_case(processor, speaker, name, result["speakers"][speaker]["conditions"], "uses")
        result["prerequisites"] = prerequisite_gate(result["speakers"])
        persist()
        if result["prerequisites"]["passed"] and treble_model_dir is not None:
            # Release USES before the optional baseline; only one processor runs at a time.
            del processor
            if treble_factory is None:
                treble_factory = model_trial.TrebleModel
            baseline = treble_factory(Path(treble_model_dir))
            progress("New Treble same-input early/late controls initialized; four separate baseline calls")
            for speaker in sorted(prepared):
                destination = dict(model=baseline.info, conditions={})
                result["speakers"][speaker]["new_treble"] = destination
                for name in ("early", "late"):
                    process_case(baseline, speaker, name, destination["conditions"], "new_treble")
        result["qualification"] = qualification_gate(result["speakers"])
        result["accepted_for_finnegan"] = result["qualification"]["passed"]
        result["status"] = ("failed" if result["failures"] else "qualified" if result["accepted_for_finnegan"]
                            else "awaiting_baseline" if result["prerequisites"]["passed"] and treble_model_dir is None
                            else "rejected")
        load_verified_controls(manifest_path, expected_manifest_sha256)
        result["peak_process_rss"] = resource.getrusage(resource.RUSAGE_SELF).ru_maxrss
        result["rss_units"] = "bytes on macOS; kilobytes on Linux"
        progress(f"Controls finished: {result['status']}; accepted_for_finnegan={result['accepted_for_finnegan']}")
        return result
    except Exception as error:
        result["status"] = "failed"
        result["accepted_for_finnegan"] = False
        result["qualification"] = dict(passed=False, reasons=[f"{type(error).__name__}: {error}"])
        result["fatal_error"] = dict(type=type(error).__name__, message=str(error))
        persist()
        raise


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--manifest", required=True)
    parser.add_argument("--outdir", required=True)
    parser.add_argument("--model-dir", required=True)
    parser.add_argument("--treble-model", required=True)
    parser.add_argument("--first-utterances", type=int, choices=(2,), default=2)
    args = parser.parse_args()
    result = run_controls(args.manifest, args.outdir, args.model_dir, treble_model_dir=args.treble_model,
                          first_utterances=args.first_utterances)
    return 0 if result["accepted_for_finnegan"] else 2


if __name__ == "__main__":
    raise SystemExit(main())
