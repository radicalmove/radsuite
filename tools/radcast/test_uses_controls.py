"""Lightweight controls tests; no learned model is loaded or executed."""
import importlib
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import numpy as np
import soundfile as sf

import analysis as qa
import model_trial


class UsesControlsTests(unittest.TestCase):
    def setUp(self):
        try:
            self.controls = importlib.import_module("uses_controls")
        except ModuleNotFoundError:
            self.fail("USES controls runner is not implemented")
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.folder = Path(self.temp.name)

    def fixture(self):
        rate = 48000
        time = np.arange(rate * 2) / rate
        envelope = .04 + .15 * np.sin(np.pi * time) ** 2
        clean = (envelope * (np.sin(2 * np.pi * 180 * time)
                            + .2 * np.sin(2 * np.pi * 3400 * time))).astype("float32")
        room, rir = model_trial.synthetic_room(clean, rate)
        source = self.folder / "licensed-source.wav"
        sf.write(source, clean, rate, subtype="FLOAT")
        source_manifest = self.folder / "source.json"
        source_manifest.write_text(json.dumps({"files": [{"path": str(source), "sha256": qa.sha(source)}]}))
        manifest = {"source_manifest": str(source_manifest),
                    "source_manifest_sha256": qa.sha(source_manifest),
                    "model": {"checkpoint_epoch": 119}, "speakers": {}}
        for speaker in ("p232", "p257"):
            paths = {}
            for key, audio in (("clean", clean), ("synthetic_room", room),
                               ("clean_model", clean), ("synthetic_room_model", room)):
                path = self.folder / f"{speaker}_{key}.wav"
                sf.write(path, audio, rate, subtype="FLOAT")
                paths[key] = {"path": str(path), "sha256": qa.sha(path), "sample_count": len(audio)}
            manifest["speakers"][speaker] = {"paths": paths, "synthetic_rir": rir,
                                              "common_input_gain": 1, "source_boundaries": [
                                                  {"filename": "first.wav", "start_sample": 0, "samples": rate,
                                                   "transcript": "Ask her to bring these things with her from the store."},
                                                  {"filename": "second.wav", "start_sample": rate, "samples": rate,
                                                   "transcript": "Six spoons of fresh snow peas, five thick slabs of blue cheese."}],
                                              "paired_room": {"input_si_sdr_db": 8, "output_si_sdr_db": 8,
                                                              "improvement_db": 0}}
        path = self.folder / "controls-source.json"
        path.write_text(json.dumps(manifest))
        return path, qa.sha(path), manifest

    def test_prefix_selection_uses_two_complete_utterances_and_retains_transcripts(self):
        audio = {key: np.arange(96000, dtype="float32") for key in
                 ("clean", "synthetic_room", "clean_model", "synthetic_room_model")}
        boundaries = [
            {"filename": "first.wav", "start_sample": 0, "samples": 16000, "transcript": "First whole sentence."},
            {"filename": "second.wav", "start_sample": 16000, "samples": 32000, "transcript": "Second whole sentence."},
            {"filename": "third.wav", "start_sample": 48000, "samples": 48000, "transcript": "Third sentence."}]
        self.assertTrue(hasattr(self.controls, "select_verified_prefix"),
                        "Complete-utterance prefix selection is not implemented")
        selected, selection = self.controls.select_verified_prefix(audio, boundaries, 2)
        self.assertEqual(selection["end_sample"], 48000)
        self.assertEqual(selection["full_source_sample_count"], 96000)
        self.assertEqual(selection["source_boundaries"], boundaries[:2])
        for key in audio:
            self.assertTrue(np.array_equal(selected[key], audio[key][:48000]))

    def test_invalid_source_boundaries_fail_before_output_or_model_load(self):
        manifest, _, records = self.fixture()
        records["speakers"]["p232"]["source_boundaries"][1]["start_sample"] += 1
        manifest.write_text(json.dumps(records))
        calls = []
        output = self.folder / "trial"
        def forbidden_model(path):
            calls.append(path)
            self.fail("Invalid boundaries reached model initialization")
        with self.assertRaisesRegex(ValueError, "boundar"):
            self.controls.run_controls(manifest, output, self.folder / "model",
                                       expected_manifest_sha256=qa.sha(manifest),
                                       model_factory=forbidden_model)
        self.assertEqual(calls, [])
        self.assertFalse(output.exists())

    def test_default_factory_selects_bounded_backend_explicitly(self):
        manifest, digest, _ = self.fixture()
        backends = []
        class DefaultProcessor:
            last_trace = {"mode": "dereverb", "memory_indices": [1], "test_only": True}
            last_inference_seconds = 0
            def __init__(self, path, *, backend):
                backends.append(backend)
                self.info = {"equivalence_report_sha256": "a" * 64, "test_only": True}
            def cleanup(self, audio, rate): return audio.copy()
        with patch.object(self.controls.uses_trial, "UsesModel", DefaultProcessor):
            try:
                result = self.controls.run_controls(manifest, self.folder / "trial", self.folder / "model",
                                                    expected_manifest_sha256=digest)
            except TypeError as error:
                self.fail(f"Default processor did not receive the explicit bounded backend: {error}")
        self.assertEqual(backends, ["bounded"])
        self.assertEqual(result["model"]["equivalence_report_sha256"], "a" * 64)

    def test_default_factory_refuses_processing_without_equivalence_identity(self):
        manifest, digest, _ = self.fixture()
        calls = []
        class UnqualifiedProcessor:
            info = {}
            def __init__(self, path, backend="upstream"): pass
            def cleanup(self, audio, rate):
                calls.append(rate)
                raise AssertionError("Processing started before bounded-equivalence evidence was verified")
        with patch.object(self.controls.uses_trial, "UsesModel", UnqualifiedProcessor):
            with self.assertRaisesRegex(ValueError, "equivalence"):
                self.controls.run_controls(manifest, self.folder / "trial", self.folder / "model",
                                           expected_manifest_sha256=digest)
        self.assertEqual(calls, [])

    def test_changed_input_is_rejected_before_output_or_model_initialization(self):
        manifest, digest, records = self.fixture()
        clean_path = Path(records["speakers"]["p232"]["paths"]["clean"]["path"])
        clean_path.write_bytes(clean_path.read_bytes() + b"changed")
        calls = []
        output = self.folder / "trial"
        with self.assertRaisesRegex(ValueError, "hash"):
            self.controls.run_controls(manifest, output, self.folder / "model",
                                       expected_manifest_sha256=digest,
                                       model_factory=lambda path: calls.append(path))
        self.assertEqual(calls, [])
        self.assertFalse(output.exists())

    def test_changed_source_manifest_and_reference_outputs_are_rejected(self):
        manifest, digest, records = self.fixture()
        for path in (Path(records["source_manifest"]),
                     Path(records["speakers"]["p257"]["paths"]["synthetic_room_model"]["path"])):
            original = path.read_bytes()
            path.write_bytes(original + b"changed")
            with self.subTest(path=path), self.assertRaisesRegex(ValueError, "hash"):
                self.controls.load_verified_controls(manifest, digest)
            path.write_bytes(original)

    def test_existing_output_directory_is_never_reused(self):
        output = self.folder / "trial"
        output.mkdir()
        sentinel = output / "protected.txt"
        sentinel.write_text("historical evidence")
        with self.assertRaisesRegex(ValueError, "fresh"):
            self.controls.fresh_directory(output)
        self.assertEqual(sentinel.read_text(), "historical evidence")
        sentinel.unlink()
        with self.assertRaisesRegex(ValueError, "fresh"):
            self.controls.fresh_directory(output)

    def test_early_and_late_split_has_no_energy_before_its_declared_delay(self):
        clean = np.zeros(48000, dtype="float32")
        clean[100] = .25
        composite, _ = model_trial.synthetic_room(clean, 48000)
        inputs, gain = self.controls.prepare_inputs(clean, composite)
        self.assertEqual(gain, 1)
        self.assertTrue(np.array_equal(inputs["clean"], clean))
        self.assertTrue(np.array_equal(inputs["composite"], composite))
        self.assertEqual(np.count_nonzero(inputs["early"][:100]), 0)
        self.assertEqual(np.count_nonzero(inputs["early"]), 3)
        self.assertAlmostEqual(float(inputs["early"][772]), .0625)
        self.assertAlmostEqual(float(inputs["early"][1876]), .03, places=7)
        self.assertEqual(np.count_nonzero(inputs["late"][:2500] - clean[:2500]), 0)
        self.assertAlmostEqual(float(np.linalg.norm(inputs["late"] - clean)), .25 * .35, places=6)
        combined = inputs["early"] + (inputs["late"] - inputs["clean"])
        np.testing.assert_allclose(combined, composite, rtol=0, atol=2e-7)

    def test_all_conditions_share_one_downward_only_headroom_gain(self):
        clean = np.ones(48000, dtype="float32") * .75
        composite, _ = model_trial.synthetic_room(clean, 48000)
        inputs, gain = self.controls.prepare_inputs(clean, composite)
        self.assertLess(gain, 1)
        self.assertGreater(gain, 0)
        self.assertLessEqual(max(float(abs(x).max()) for x in inputs.values()), .800001)
        np.testing.assert_allclose(inputs["clean"], clean * gain, rtol=0, atol=0)
        np.testing.assert_allclose(inputs["composite"], composite * gain, rtol=0, atol=0)

    def test_clean_watchdog_ignores_global_gain_but_keeps_local_erasure(self):
        time = np.arange(48000 * 4) / 48000
        envelope = .04 + .12 * np.sin(2 * np.pi * .8 * time) ** 2
        clean = (envelope * np.sin(2 * np.pi * 180 * time)).astype("float32")
        good = self.controls.preservation_metrics(clean, clean * .15)
        self.assertEqual(good["watchdog"], [])
        self.assertGreater(good["analysis_gain_compensation_db"], 15)
        erased = clean.copy()
        erased[48000:96000] *= .001
        bad = self.controls.preservation_metrics(clean, erased)
        self.assertTrue(bad["watchdog"])

    def test_output_count_nonfinite_and_clipping_are_hard_failures(self):
        manifest, _, records = self.fixture()
        clean, _ = sf.read(records["speakers"]["p232"]["paths"]["clean"]["path"], dtype="float32")
        nonfinite = clean.copy()
        nonfinite[0] = np.nan
        clipped = clean.copy()
        clipped[0] = 1
        for output, issue in ((clean[:-1], "sample_count"), (nonfinite, "empty_or_nonfinite"),
                              (clipped, "clipping")):
            with self.subTest(issue=issue):
                self.assertIn(issue, self.controls._assess(clean, clean, output)["technical_issues"])

    def test_gate_requires_clean_and_meaningful_early_and_late_improvement(self):
        def condition(improvement=1.5, watchdog=None, issues=None):
            return {"technical_issues": issues or [], "paired": {"improvement_db": improvement},
                    "preservation": {"watchdog": watchdog or []}}
        speakers = {"p232": {"conditions": {key: condition() for key in self.controls.CONDITIONS}}}
        self.assertTrue(self.controls.prerequisite_gate(speakers)["passed"])
        speakers["p232"]["conditions"]["early"] = condition(.9)
        self.assertFalse(self.controls.prerequisite_gate(speakers)["passed"])
        speakers["p232"]["conditions"]["early"] = condition()
        speakers["p232"]["conditions"]["clean"] = condition(watchdog=[{"code": "hf_collapse"}])
        self.assertFalse(self.controls.prerequisite_gate(speakers)["passed"])
        speakers["p232"]["conditions"]["clean"] = condition(issues=["sample_count"])
        self.assertFalse(self.controls.prerequisite_gate(speakers)["passed"])

    def test_final_gate_requires_actual_same_input_treble_comparison(self):
        condition = {"technical_issues": [], "paired": {"improvement_db": 2, "output_si_sdr_db": 15},
                     "preservation": {"watchdog": []}}
        speakers = {"p232": {"conditions": {key: dict(condition) for key in self.controls.CONDITIONS}}}
        self.assertFalse(self.controls.qualification_gate(speakers)["passed"])
        speakers["p232"]["new_treble"] = {"conditions": {
            "early": {"technical_issues": [], "paired": {"output_si_sdr_db": 14}},
            "late": {"technical_issues": [], "paired": {"output_si_sdr_db": 16}}}}
        self.assertTrue(self.controls.qualification_gate(speakers)["passed"])
        speakers["p232"]["new_treble"]["conditions"]["early"]["paired"]["output_si_sdr_db"] = 14.1
        self.assertFalse(self.controls.qualification_gate(speakers)["passed"])

    def test_new_treble_runs_only_after_all_eight_uses_conditions_pass(self):
        manifest, digest, records = self.fixture()
        reference, _ = sf.read(records["speakers"]["p232"]["paths"]["clean"]["path"], dtype="float32")
        events = []
        class Processor:
            info = {"device": "cpu", "test_only": True}
            last_trace = {"mode": "dereverb", "memory_indices": [1], "test_only": True}
            last_inference_seconds = 0
            def __init__(self, label): self.label = label
            def cleanup(self, audio, rate):
                events.append(self.label)
                return reference.copy() if self.label == "uses" else audio.copy()
        def treble_factory(path):
            events.append("treble_initialized")
            return Processor("treble")
        result = self.controls.run_controls(manifest, self.folder / "trial", self.folder / "model",
                                            expected_manifest_sha256=digest,
                                            model_factory=lambda path: Processor("uses"),
                                            treble_model_dir=self.folder / "treble",
                                            treble_factory=treble_factory)
        self.assertEqual(events, ["uses"] * 8 + ["treble_initialized"] + ["treble"] * 4)
        self.assertTrue(result["qualification"]["passed"])
        self.assertEqual(result["inference_counts"], {"uses": 8, "new_treble": 4})

    def test_runner_preserves_partial_results_after_a_processing_error(self):
        manifest, digest, _ = self.fixture()
        class FailingProcessor:
            info = {"device": "cpu", "test_only": True}
            last_trace = {"mode": "dereverb", "memory_indices": [1], "test_only": True}
            last_inference_seconds = 0
            def __init__(self): self.calls = 0
            def cleanup(self, audio, rate):
                self.calls += 1
                if self.calls == 2: raise RuntimeError("intentional test interruption")
                return audio.copy()
        processor = FailingProcessor()
        output = self.folder / "trial"
        result = self.controls.run_controls(manifest, output, self.folder / "model",
                                            expected_manifest_sha256=digest, model_factory=lambda path: processor)
        partial = json.loads((output / "controls.json").read_text())
        self.assertEqual(partial["status"], "failed")
        self.assertFalse(partial["qualification"]["passed"])
        saved = partial["speakers"]["p232"]["conditions"]["clean"]["output"]
        self.assertEqual(qa.sha(saved["path"]), saved["sha256"])
        self.assertTrue((output / "p232_early_input.wav").is_file())
        self.assertEqual(processor.calls, 8)
        self.assertFalse(result["accepted_for_finnegan"])
        self.assertEqual(partial["speakers"]["p232"]["conditions"]["early"]["error"]["message"],
                         "intentional test interruption")

    def test_runner_does_not_claim_new_early_late_treble_scores(self):
        manifest, digest, records = self.fixture()
        class IdentityProcessor:
            info = {"device": "cpu", "test_only": True}
            last_trace = {"mode": "dereverb", "memory_indices": [1], "test_only": True}
            last_inference_seconds = 0
            def cleanup(self, audio, rate): return audio.copy()
        baseline_calls = []
        result = self.controls.run_controls(manifest, self.folder / "trial", self.folder / "model",
                                            expected_manifest_sha256=digest,
                                            model_factory=lambda path: IdentityProcessor(),
                                            treble_model_dir=self.folder / "treble",
                                            treble_factory=lambda path: baseline_calls.append(path))
        self.assertFalse(result["qualification"]["passed"])
        self.assertEqual(result["status"], "rejected")
        self.assertEqual(baseline_calls, [])
        self.assertEqual(result["inference_counts"], {"uses": 8, "new_treble": 0})
        for speaker, entry in result["speakers"].items():
            self.assertEqual(set(entry["old_treble"]["conditions"]), {"clean", "composite"})
            self.assertEqual(entry["old_treble"]["reported_paired_room"], records["speakers"][speaker]["paired_room"])
            for condition in entry["conditions"].values():
                self.assertEqual(qa.sha(condition["output"]["path"]), condition["output"]["sha256"])
                self.assertEqual(condition["processing"]["trace"]["memory_indices"], [1])


if __name__ == "__main__":
    unittest.main()
