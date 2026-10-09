"""Source-loss recovery and cancellation safety, not perceptual-quality tests."""
import importlib
import importlib.util
import unittest
import numpy as np


def api(test):
    test.assertIsNotNone(importlib.util.find_spec('adaptive_preservation'),
                         'The adaptive preservation probe is not implemented')
    return importlib.import_module('adaptive_preservation')


class AdaptivePreservationTests(unittest.TestCase):
    def fixture(self, gain=.02):
        sr=48000;t=np.arange(sr*10+137)/sr
        x=(.05*np.sin(2*np.pi*180*t)*(.2+.8*np.sin(2*np.pi*2*t)**2)).astype('float32')
        y=x*.65;y[5*sr:7*sr]=x[5*sr:7*sr]*gain
        return x,y

    def test_invalid_rate_nonfinite_empty_or_mismatched_stems_are_rejected(self):
        p=api(self);x=np.zeros(960,dtype='float32')
        for source,dry,model,sr in ((x,x,x,16000),(x,x,x[:-1],48000),
            (x,x,np.full_like(x,np.nan),48000),(x.reshape(2,-1),x,x,48000),
            (x[:0],x[:0],x[:0],48000)):
            with self.assertRaises(ValueError):p.apply(source,dry,model,sr)

    def test_silence_and_short_finite_recordings_keep_every_sample(self):
        p=api(self)
        for length in (137,48013):
            x=np.zeros(length,dtype='float32');y,w,r=p.apply(x,x,x,48000)
            np.testing.assert_array_equal(y,x);self.assertFalse(np.any(w))

    def test_unchanged_and_uniformly_attenuated_speech_bypass_exactly(self):
        p=api(self);x,_=self.fixture()
        for model in (x,x*.65):
            y,w,r=p.apply(x,x,model,48000)
            np.testing.assert_array_equal(y,model);self.assertFalse(np.any(w))

    def test_severe_loss_is_recovered_without_touching_normal_speech(self):
        p=api(self);x,m=self.fixture();y,w,r=p.apply(x,x,m,48000)
        np.testing.assert_array_equal(y[:3*48000],m[:3*48000])
        np.testing.assert_array_equal(y[9*48000:],m[9*48000:])
        core=slice(6*48000,round(6.5*48000))
        gain=np.linalg.norm(y[core])/np.linalg.norm(x[core])
        self.assertGreater(gain,.20);self.assertLessEqual(float(w.max()),.2+1e-7)
        self.assertGreater(len(r['triggered_windows']),0)
        self.assertEqual(len(y),len(x));self.assertEqual(y.dtype,np.float32)

    def test_bounded_smooth_weights_and_deterministic_nonaligned_output(self):
        p=api(self);x,m=self.fixture();a,w,r=p.apply(x,x,m,48000);b,v,s=p.apply(x,x,m,48000)
        np.testing.assert_array_equal(a,b);np.testing.assert_array_equal(w,v)
        self.assertTrue(np.isfinite(a).all());self.assertGreaterEqual(float(w.min()),0)
        self.assertLessEqual(float(w.max()),.2+1e-7)
        curve=p.sample_curve(w,len(x),48000)
        self.assertLess(float(np.max(abs(np.diff(curve)))),.001)

    def test_onset_ramp_and_cap_are_frozen(self):
        p=api(self)
        np.testing.assert_allclose(p.activation(np.array([0,-9,-10.5,-12,-30])),[0,0,.1,.2,.2])

    def test_phase_cancellation_bypasses_whole_recovery_support(self):
        p=api(self);x,m=self.fixture(gain=-.15);y,w,r=p.apply(x,x,m,48000)
        self.assertGreater(len(r['energy_guard_bypassed_intervals']),0)
        np.testing.assert_array_equal(y,m)
        self.assertFalse(np.any(w));self.assertEqual(r['remaining_energy_guard_conflicts'],0)
        # Bypass is conservative. It does not assert this suppressed source is
        # recovered or waive the final original-source quiet-loss watchdog.

    def test_severely_attenuated_opposite_phase_can_recover_when_energy_safe(self):
        p=api(self);x,m=self.fixture(gain=-.02);y,w,r=p.apply(x,x,m,48000)
        core=slice(6*48000,round(6.5*48000))
        self.assertGreater(np.linalg.norm(y[core])/np.linalg.norm(x[core]),.17)
        self.assertEqual(r['remaining_energy_guard_conflicts'],0)


if __name__=='__main__':unittest.main()
