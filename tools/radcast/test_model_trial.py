"""Contracts for the isolated, pinned room-trained model experiment."""
import importlib
import importlib.util
import os
from pathlib import Path
import tempfile
import unittest
import numpy as np


def api(test):
    test.assertIsNotNone(importlib.util.find_spec('model_trial'),
                         'The isolated model-trial adapter is not implemented')
    return importlib.import_module('model_trial')


class LoadAuditTests(unittest.TestCase):
    def test_complete_checkpoint_matches_every_expected_tensor(self):
        trial=api(self)
        expected={'layer.weight':np.ones((2,3)), 'layer.bias':np.ones(2)}
        self.assertEqual(trial.audit_state_dict(expected,expected)['tensor_count'],2)

    def test_missing_unexpected_and_mismatched_tensors_fail_closed(self):
        trial=api(self)
        expected={'layer.weight':np.ones((2,3))}
        for checkpoint in ({}, {'unrelated.weight':np.ones((2,3))},
                           {'layer.weight':np.ones((2,4))}):
            with self.assertRaisesRegex(ValueError,'checkpoint'):
                trial.audit_state_dict(expected,checkpoint)

    def test_nonfinite_weights_are_rejected(self):
        trial=api(self)
        with self.assertRaisesRegex(ValueError,'finite'):
            trial.audit_state_dict({'w':np.zeros(2)},{'w':np.array([0,np.nan])})

    def test_corrupt_or_missing_pinned_artifacts_are_rejected(self):
        trial=api(self)
        with tempfile.TemporaryDirectory() as folder:
            root=Path(folder)
            with self.assertRaises(FileNotFoundError):trial.verify_artifacts(root)
            (root/'checkpoints').mkdir()
            (root/'config.ini').write_text('unexpected config')
            (root/'checkpoints/model_119.ckpt.best').write_bytes(b'unexpected checkpoint')
            with self.assertRaisesRegex(ValueError,'hash'):trial.verify_artifacts(root)


class AudioContractTests(unittest.TestCase):
    def test_wrong_rate_empty_nonfinite_or_multichannel_input_is_rejected(self):
        trial=api(self)
        for x,sr in ((np.zeros(480),16000),(np.array([]),48000),
                     (np.array([np.nan]),48000),(np.zeros((2,480)),48000)):
            with self.assertRaises(ValueError):trial.validate_audio(x,sr)

    def test_output_cannot_drop_or_append_samples(self):
        trial=api(self)
        source=np.zeros(481,dtype='float32')
        self.assertEqual(len(trial.validate_output(source,source.copy())),481)
        for output in (source[:-1],np.r_[source,0],np.full_like(source,np.inf)):
            with self.assertRaises(ValueError):trial.validate_output(source,output)

    def test_trial_cannot_overwrite_historical_evidence(self):
        trial=api(self)
        self.assertTrue(hasattr(trial,'fresh_evidence_directory'),
                        'Evidence-directory protection is not implemented')
        with tempfile.TemporaryDirectory() as folder:
            root=Path(folder);(root/'historic.wav').write_bytes(b'keep')
            with self.assertRaises(ValueError):trial.fresh_evidence_directory(root)
            self.assertEqual((root/'historic.wav').read_bytes(),b'keep')

    def test_declared_room_control_keeps_direct_arrival_and_sample_count(self):
        trial=api(self)
        self.assertTrue(hasattr(trial,'synthetic_room'),
                        'Paired synthetic-room control is not implemented')
        x=np.zeros(48001,dtype='float32');x[1000]=1
        room,info=trial.synthetic_room(x,48000)
        self.assertEqual(room.shape,x.shape)
        self.assertEqual(room[1000],1)
        self.assertEqual(np.count_nonzero(room[:1000]),0)
        np.testing.assert_array_equal(room,trial.synthetic_room(x,48000)[0])
        self.assertTrue(info['synthetic_not_measured_room'])

    def test_experimental_blend_is_explicit_and_bounded(self):
        trial=api(self)
        self.assertTrue(hasattr(trial,'validate_blend'),'Bounded trial blend is not implemented')
        self.assertEqual(trial.validate_blend(0),0)
        self.assertEqual(trial.validate_blend(.15),.15)
        for value in (-.1,.31,float('nan'),float('inf'),True):
            with self.assertRaises(ValueError):trial.validate_blend(value)


@unittest.skipUnless(os.environ.get('RADSUITE_TEST_MODEL_TRIAL')=='1',
                     'Set RADSUITE_TEST_MODEL_TRIAL=1 for real pinned-model inference')
class RealModelTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.trial=importlib.import_module('model_trial')
        cls.model=cls.trial.TrebleModel(Path(os.environ['RADSUITE_TRIAL_MODEL']))

    def fixture(self,seconds):
        sr=48000;t=np.arange(round(sr*seconds)+13)/sr
        return ((.04*np.sin(2*np.pi*(180*t+2*t*t))+
                 .01*np.sin(2*np.pi*3500*t))*(.6+.4*np.sin(2*np.pi*2*t)**2)).astype('float32')

    def test_real_checkpoint_loaded_completely_on_cpu(self):
        self.assertGreater(self.model.info['load_audit']['tensor_count'],50)
        self.assertEqual(self.model.info['device'],'cpu')
        self.assertEqual(self.model.info['sample_rate'],48000)
        self.assertEqual(self.model.info['checkpoint_epoch'],119)

    def test_repeatable_full_context_with_nonaligned_final_frame(self):
        x=self.fixture(20.2)
        a=self.model.cleanup(x,48000)
        b=self.model.cleanup(x,48000)
        self.assertEqual(a.shape,x.shape)
        self.assertTrue(np.isfinite(a).all())
        np.testing.assert_array_equal(a,b)
        import analysis as qa
        timing=qa.alignment(x,a,48000)
        self.assertLessEqual(abs(timing['lag_seconds']),.015)
        self.assertLessEqual(abs(timing['block_lag_spread_seconds']),.025)


if __name__=='__main__':unittest.main()
