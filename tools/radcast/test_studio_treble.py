import ast
import json
import importlib
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import numpy as np
import soundfile as sf


class ProductionTrebleTests(unittest.TestCase):
    def setUp(self):
        try:self.m=importlib.import_module('studio_treble')
        except ModuleNotFoundError:self.fail('Production Treble helper missing')

    def test_core_controller_matches_reviewed_experiment(self):
        root=Path(__file__).parent
        orig=ast.parse((root/'adaptive_preservation.py').read_text());new=ast.parse((root/'treble_preservation.py').read_text())
        for name in ['activation','sample_curve','intervals','apply']:
            a=next(n for n in orig.body if getattr(n,'name',None)==name);b=next(n for n in new.body if getattr(n,'name',None)==name)
            self.assertEqual(ast.dump(a,include_attributes=False),ast.dump(b,include_attributes=False))

    def test_model_adapter_matches_reviewed_strict_loader(self):
        root=Path(__file__).parent
        orig=ast.parse((root/'model_trial.py').read_text());new=ast.parse((root/'treble_model.py').read_text())
        for name in ['verify_artifacts','as_array','audit_state_dict','validate_audio','validate_output','TrebleModel']:
            a=next(n for n in orig.body if getattr(n,'name',None)==name);b=next(n for n in new.body if getattr(n,'name',None)==name)
            self.assertEqual(ast.dump(a,include_attributes=False),ast.dump(b,include_attributes=False))

    def test_protection_reference_is_source_derived_and_decision_only(self):
        raw=np.ones(48000,dtype='float32')*.03;hp=np.ones(48000,dtype='float32')*.1
        r=self.m.protection_reference(raw,hp)
        self.assertTrue(np.array_equal(r,(.8*raw+.2*hp).astype('float32')))

    def test_saved_gain_replay_does_not_recalculate_leveling(self):
        x=np.linspace(-.1,.1,48000,dtype='float32');g=np.ones(50)*6
        y=self.m.with_frame_gains(x,g)
        self.assertLess(float(np.max(abs(y-x*10**(.3)))),1e-7)

    def test_missing_model_falls_back_and_keeps_original_file(self):
        with tempfile.TemporaryDirectory() as folder:
            p=Path(folder);source=p/'input.wav';out=p/'output.wav'
            t=np.arange(96000)/48000;x=(.1*np.sin(2*np.pi*220*t)*np.sin(2*np.pi*2*t)**2).astype('float32')
            sf.write(source,x,48000,subtype='FLOAT');before=source.read_bytes()
            r=self.m.render(source,out,model_dir=p/'missing-model')
            self.assertTrue(r['fallback']);self.assertTrue(r['warnings'])
            self.assertEqual(r['config']['preset_id'],'studio_treble')
            self.assertEqual(before,source.read_bytes());self.assertTrue(out.with_suffix('.qa.json').exists())
            y,sr=sf.read(out,dtype='float32');self.assertEqual(sr,48000);self.assertEqual(len(y),len(x))
            self.assertFalse(r['watchdog'])

    def test_successful_per_input_calibration_and_guard_fallback(self):
        class Model:
            info={'checkpoint_epoch':119};last_inference_seconds=0
            def __init__(self,*args):pass
            def cleanup(self,x,sr):return .9*x
        with tempfile.TemporaryDirectory() as folder:
            p=Path(folder);source=p/'input.wav';out=p/'output.wav'
            t=np.arange(96000)/48000;x=(.1*np.sin(2*np.pi*220*t)*np.sin(2*np.pi*2*t)**2).astype('float32')
            sf.write(source,x,48000,subtype='FLOAT')
            with patch.object(self.m,'TrebleModel',Model):
                r=self.m.render(source,out)
                self.assertFalse(r['fallback']);self.assertTrue(r['baseline_metrics'])
                self.assertEqual(r['calibrated_stem_identities']['raw_model']['count'],len(x))
                self.assertEqual(r['calibrated_stem_identities']['calibrated_frame_gain_db']['count'],100)
                final=self.m.studio.verify_export(source,out,out.with_suffix('.qa.json'),origin=source)
                self.assertEqual(final['calibrated_stem_identities'],r['calibrated_stem_identities'])
                audio,rate=sf.read(out,dtype='float32');trimmed=p/'trimmed.wav'
                self.m.studio.write_float(trimmed,audio[:72000],rate)
                edited_report=p/'trimmed.qa.json';edited_report.write_text(json.dumps(r))
                edited=self.m.studio.verify_export(source,trimmed,edited_report,edited=True,removed_seconds=.5,origin=source)
                self.assertTrue(edited['final_export_edited']);self.assertEqual(edited['metrics']['expected_duration_seconds'],1.5)
                defect={'code':'speech','detail':'Injected preservation rejection'}
                for name,guards in [('source',[[],[defect],[],[]]),('baseline',[[],[],[defect],[]])]:
                    with patch.object(self.m.qa,'watchdog',side_effect=guards):
                        rejected=self.m.render(source,p/(name+'-rejected.wav'))
                    self.assertTrue(rejected['fallback']);self.assertFalse(rejected['watchdog'])
                    self.assertIn(defect,rejected['rejected_path_watchdog']+rejected['watchdog_against_baseline'])

    def test_calibration_has_no_experiment_saved_file_dependencies(self):
        code=Path(self.m.__file__).read_text()
        self.assertNotIn('finnegan-repair-dry20',code);self.assertNotIn('GAIN_SHA',code)
        self.assertNotIn('from model_trial',code)


if __name__=='__main__':unittest.main()
