import unittest
import numpy as np
import analysis as qa
import speech_cleanup as sc

class Revision2Tests(unittest.TestCase):
    def fixture(self):
        sr=48000;t=np.arange(sr*14)/sr
        envelope=np.where(t<7,.005,.15)*(np.sin(2*np.pi*2*t)**2)
        envelope[(t>5)&(t<7)]=0
        return (envelope*np.sin(2*np.pi*220*t)).astype('float32'),sr

    def test_quiet_speech_not_pause_when_later_passage_is_loud(self):
        x,sr=self.fixture();speech,pause=qa.masks(x,sr)
        # Centers of quiet voiced syllables, over 20 dB below later speech.
        for center in (.12,.62,1.12,2.12,3.12):
            self.assertTrue(speech[round(center/.02)])
            self.assertFalse(pause[round(center/.02)])

    def test_leveling_reduces_passage_difference_without_raising_silent_gap(self):
        x,sr=self.fixture();x[:7*sr]*=3;speech,_=qa.masks(x,sr)
        y,info=sc.level_speech(x,x,sr,speech)
        a=np.sqrt(np.mean(y[sr:4*sr]**2));b=np.sqrt(np.mean(y[9*sr:12*sr]**2))
        self.assertLess(qa.db(b/a),10)
        self.assertLessEqual(max(abs(y[int(5.4*sr):int(6.6*sr)])),1e-8)
        self.assertLessEqual(info['maximum_gain_db'],8.01)
        self.assertGreaterEqual(info['minimum_gain_db'],-8.01)

    def test_wpe_preserves_sample_count_and_finite_values(self):
        x,sr=self.fixture();y,info=sc.dereverb(x[:sr*3],sr)
        self.assertEqual(len(x[:sr*3]),len(y));self.assertTrue(np.isfinite(y).all())
        self.assertAlmostEqual(qa.alignment(x[:sr*3],y,sr)['lag_seconds'],0,places=2)
        self.assertEqual(info['taps'],10)

    def test_presence_correction_is_bounded(self):
        sr=48000;t=np.arange(sr)/sr;x=.1*np.sin(2*np.pi*2400*t)
        y=sc.presence(x,sr)
        self.assertLess(qa.db(np.sqrt(np.mean(y*y))/np.sqrt(np.mean(x*x))),2.1)
        self.assertEqual(len(y),len(x))



class EnvelopeTests(unittest.TestCase):
    def test_slow_level_change_is_separated_from_articulation_shape(self):
        sr=48000;t=np.arange(sr*10)/sr
        x=(.05*np.sin(2*np.pi*220*t)*(np.sin(2*np.pi*3*t)**2)).astype('float32')
        gain=np.exp(np.linspace(-1,1,len(x)))
        result=qa.alignment(x,x*gain,sr)
        self.assertGreater(result['speech_envelope_shape_correlation'],.95)



class QuietLossTests(unittest.TestCase):
    def test_watchdog_catches_local_quiet_voice_erasure(self):
        sr=48000;t=np.arange(sr*12)/sr
        x=(.1*np.sin(2*np.pi*300*t)*(np.sin(2*np.pi*3*t)**2)).astype('float32')
        x[4*sr:8*sr]*=.04;y=x.copy();y[4*sr:8*sr]*=.1
        source=qa.signal_metrics(x,sr);out=qa.signal_metrics(y,sr,qa.masks(x,sr));out.update(qa.alignment(x,y,sr));out['true_peak_dbfs']=-5
        self.assertIn('quiet_speech_loss',[w['code'] for w in qa.watchdog(source,out)])

    def test_pause_attenuation_matches_recorded_config(self):
        import studio
        sr=48000;t=np.arange(sr*8)/sr;source=.1*np.sin(2*np.pi*300*t)
        source[4*sr:]=0;x=np.ones_like(source)*.001
        y=studio.pause_control(x,source,sr,max_db=studio.CONFIG['pause_max_attenuation_db'])
        self.assertAlmostEqual(qa.db(np.mean(y[6*sr:7*sr])/.001),-studio.CONFIG['pause_max_attenuation_db'],places=2)



class AdaptivePresenceTests(unittest.TestCase):
    def test_already_present_voice_receives_no_presence_boost(self):
        sr=48000;t=np.arange(sr)/sr
        x=(.03*np.sin(2*np.pi*500*t)+.05*np.sin(2*np.pi*2400*t)).astype('float32')
        self.assertAlmostEqual(sc.presence_gain(x,sr),0,places=6)

if __name__=='__main__':unittest.main()
