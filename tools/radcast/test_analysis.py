import unittest
import numpy as np
import analysis as a

class AnalysisTests(unittest.TestCase):
    def test_timing_lag(self):
        rng = np.random.default_rng(1)
        x = rng.normal(size=48000)
        self.assertAlmostEqual(a.alignment(x, np.r_[np.zeros(480), x], 48000)['lag_seconds'], .01, places=3)

    def test_watchdog_rejects_clipping_drift_and_noise(self):
        base = dict(duration_seconds=2, sample_peak_dbfs=-3, true_peak_dbfs=-3,
                    pause_rms_dbfs=-50, speech_rms_dbfs=-20, envelope_correlation=1,
                    lag_seconds=0, block_lag_spread_seconds=0, clipped_samples=0,
                    bands_db_relative={'body':-5,'presence':-5,'sibilants':-12,'air':-20,'sub':-30})
        out = dict(base, duration_seconds=2.2, true_peak_dbfs=0, pause_rms_dbfs=-40,
                   clipped_samples=10, envelope_correlation=.5)
        self.assertGreaterEqual(len(a.watchdog(base, out)), 5)

    def test_same_signal_has_no_warnings(self):
        sr = 48000
        t = np.arange(sr * 2) / sr
        x = .1 * np.sin(2*np.pi*1000*t)
        m = a.signal_metrics(x, sr)
        m.update(true_peak_dbfs=-20, lag_seconds=0, block_lag_spread_seconds=0, envelope_correlation=1)
        self.assertEqual(a.watchdog(m, m), [])

    def test_spectral_collapse_is_detected(self):
        base = dict(duration_seconds=1, clipped_samples=0, true_peak_dbfs=-3,
                    pause_rms_dbfs=-60, speech_rms_dbfs=-20,
                    bands_db_relative={'sub':-30,'body':-6,'presence':-6,'sibilants':-12,'air':-18})
        out = dict(base, bands_db_relative=dict(base['bands_db_relative'], air=-38, sibilants=-25))
        codes = [w['code'] for w in a.watchdog(base, out)]
        self.assertIn('hf_collapse', codes)

    def test_nonfinite_is_destructive(self):
        with self.assertRaises(ValueError):
            a.signal_metrics(np.array([0., np.nan]), 48000)



class FloorTests(unittest.TestCase):
    def test_inaudible_band_changes_do_not_fail_export(self):
        source=dict(duration_seconds=2,clipped_samples=0,true_peak_dbfs=-3,pause_rms_dbfs=-60,speech_rms_dbfs=-20,
                    bands_db_relative={'sub':-80,'body':-1,'presence':-70,'sibilants':-90,'air':-100})
        out=dict(source,bands_db_relative=dict(source['bands_db_relative'],sibilants=-72))
        self.assertEqual(a.watchdog(source,out),[])

if __name__=='__main__':unittest.main()
