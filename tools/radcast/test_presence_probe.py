"""Signal contracts for one frozen-upstream tonal probe."""
import importlib
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
import numpy as np


def api(test):
    test.assertIsNotNone(importlib.util.find_spec('presence_probe'),
                         'The bounded presence probe is not implemented')
    return importlib.import_module('presence_probe')


class PresenceProbeTests(unittest.TestCase):
    def test_wrong_rate_nonfinite_empty_or_multichannel_is_rejected(self):
        p=api(self)
        for x,sr in ((np.zeros(480),16000),(np.zeros((2,480)),48000),
                     (np.array([]),48000),(np.array([np.nan]),48000)):
            with self.assertRaises(ValueError):p.apply(x,sr)

    def test_exact_count_finite_repeatable_non_window_aligned_output(self):
        p=api(self);sr=48000;t=np.arange(sr+137)/sr
        x=(.07*np.sin(2*np.pi*180*t)+.02*np.sin(2*np.pi*3300*t)).astype('float32')
        a=p.apply(x,sr);b=p.apply(x,sr)
        self.assertEqual(a.shape,x.shape);self.assertEqual(a.dtype,np.float32)
        self.assertTrue(np.isfinite(a).all());np.testing.assert_array_equal(a,b)

    def test_silence_remains_exactly_silent(self):
        p=api(self);x=np.zeros(48013,dtype='float32')
        np.testing.assert_array_equal(p.apply(x,48000),x)

    def test_combined_response_is_bounded_without_sub_inflation_or_air_cliff(self):
        p=api(self);f=np.geomspace(20,23999,12000);g=p.response_db(f,48000)
        self.assertLessEqual(float(g.max()),3.5)
        self.assertLessEqual(float(g[f<80].max()),.5)
        self.assertGreaterEqual(float(g.min()),-.01)
        self.assertGreaterEqual(float(g[f>=8000].min()),-.01)

    def test_body_and_word_detail_are_modest_measured_tone_changes(self):
        p=api(self);sr=48000;t=np.arange(sr*2)/sr
        for frequency,minimum,maximum in ((180,.9,1.3),(3000,1.9,3.5),(5200,1.4,3.5)):
            x=(.05*np.sin(2*np.pi*frequency*t)).astype('float32')
            y=p.apply(x,sr)
            gain=20*np.log10(np.sqrt(np.mean(y[sr:]**2))/np.sqrt(np.mean(x[sr:]**2)))
            self.assertGreaterEqual(gain,minimum);self.assertLessEqual(gain,maximum)

    def test_impulse_has_no_prepulse_or_appended_samples_and_keeps_arrival(self):
        p=api(self);x=np.zeros(48013,dtype='float32');x[24000]=.1
        y=p.apply(x,48000)
        self.assertEqual(len(y),len(x));self.assertEqual(np.count_nonzero(y[:24000]),0)
        self.assertLessEqual(abs(int(np.argmax(np.abs(y)))-24000),48)

    def test_rejected_baseline_is_refused_before_artifact_access(self):
        p=api(self)
        with tempfile.TemporaryDirectory() as folder:
            path=Path(folder)/'trial.qa.json'
            path.write_text(json.dumps({'accepted_for_listening':False,'fallback':False,'watchdog':[{'code':'quiet_speech_loss'}]}))
            with self.assertRaisesRegex(ValueError,'accepted'):p.verified_baseline(path)

    def test_changed_baseline_master_is_refused(self):
        p=api(self)
        import analysis as qa
        with tempfile.TemporaryDirectory() as folder:
            root=Path(folder);master=root/'master.wav';master.write_bytes(b'initial')
            report=dict(accepted_for_listening=True,fallback=False,watchdog=[],output=str(master),output_sha256=qa.sha(master))
            path=root/'trial.qa.json';path.write_text(json.dumps(report));master.write_bytes(b'changed')
            with self.assertRaisesRegex(ValueError,'hash'):p.verified_baseline(path)


if __name__=='__main__':unittest.main()
