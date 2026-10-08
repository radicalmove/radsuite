import importlib
import json
from pathlib import Path
import tempfile
import unittest


class UsesRenderGateTests(unittest.TestCase):
    def setUp(self):
        try:self.module=importlib.import_module('uses_render')
        except ModuleNotFoundError:self.fail('USES full-candidate gate is not implemented')

    def test_rejected_controls_cannot_create_candidate_directory(self):
        with tempfile.TemporaryDirectory() as folder:
            root=Path(folder);controls=root/'controls.json'
            controls.write_text(json.dumps(dict(accepted_for_finnegan=False,qualification=dict(passed=False,reasons=['clean preservation failed']))))
            target=root/'candidate'
            with self.assertRaises(ValueError):
                self.module.render(controls,root/'missing-baseline.json',target,root/'missing-model')
            self.assertFalse(target.exists())

    def test_incomplete_controls_cannot_create_candidate_directory(self):
        with tempfile.TemporaryDirectory() as folder:
            root=Path(folder);controls=root/'controls.json'
            controls.write_text(json.dumps(dict(accepted_for_finnegan=True,qualification=dict(passed=True),counts=dict(uses=7,new_treble=0))))
            target=root/'candidate'
            with self.assertRaises(ValueError):
                self.module.render(controls,root/'missing-baseline.json',target,root/'missing-model')
            self.assertFalse(target.exists())

    def test_completed_controls_use_actual_runner_count_schema(self):
        self.assertTrue(hasattr(self.module,'qualified_controls'),'Positive qualification validator missing')
        report=dict(accepted_for_finnegan=True,qualification=dict(passed=True),inference_counts=dict(uses=8,new_treble=4))
        self.assertEqual(self.module.qualified_controls(report),report)

    def test_one_study_attempt_is_atomic_across_output_directories(self):
        self.assertTrue(hasattr(self.module,'claim_attempt'),'Task-level budget ledger missing')
        with tempfile.TemporaryDirectory() as folder:
            root=Path(folder);a=self.module.claim_attempt(root,dict(outdir='first'))
            self.assertTrue(a.exists())
            with self.assertRaises(FileExistsError):self.module.claim_attempt(root,dict(outdir='second'))
            self.assertEqual(json.loads(a.read_text())['outdir'],'first')

    def test_another_accepted_baseline_cannot_replace_reviewed_dry20(self):
        self.assertTrue(hasattr(self.module,'validate_baseline_identity'),'Pinned baseline identity validator missing')
        base=dict(accepted_for_listening=True,fallback=False,watchdog=[],explicit_dry_blend=.2)
        with self.assertRaises(ValueError):self.module.validate_baseline_identity(base,'a'*64)
        with self.assertRaises(ValueError):self.module.validate_baseline_identity(dict(base,explicit_dry_blend=0),self.module.BASELINE_REPORT_SHA)

    def test_backend_label_cannot_replace_passed_equivalence_evidence(self):
        self.assertTrue(hasattr(self.module,'require_equivalence'),'Passed numerical evidence validator missing')
        with self.assertRaises(ValueError):self.module.require_equivalence(dict(execution_backend='bounded_equivalent'))

    def test_verified_bounded_features_backend_can_pass_its_actual_schema(self):
        self.module.require_equivalence(dict(execution_backend='bounded_features',trained_backend_equivalence_verified=True,equivalence_report_sha256='a'*64))


if __name__=='__main__':unittest.main()
