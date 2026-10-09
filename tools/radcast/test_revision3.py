import unittest
import numpy as np
from scipy import signal
import analysis as qa
import speech_cleanup as sc

class DelayTests(unittest.TestCase):
    def fixture(self,seconds=3):
        sr=48000;t=np.arange(round(seconds*sr))/sr
        phase=2*np.pi*(180*t+3*t*t)
        clean=(.08*np.sin(phase)+.02*np.sin(phase*3))*(np.sin(2*np.pi*2*t)**2)
        rir=np.zeros(round(sr*.08));rir[0]=1;rir[round(.014*sr)]=.25;rir[round(.037*sr)]=.12
        return signal.fftconvolve(clean,rir)[:len(clean)].astype('float32'),sr

    def test_experimental_delay_report_and_timing(self):
        x,sr=self.fixture(13)
        y,info=sc.dereverb(x,sr,delay=2)
        self.assertEqual(info['delay'],2)
        self.assertEqual(len(y),len(x));self.assertTrue(np.isfinite(y).all())
        self.assertAlmostEqual(qa.alignment(x,y,sr)['lag_seconds'],0,places=2)
        self.assertGreater(np.sqrt(np.mean(y*y))/np.sqrt(np.mean(x*x)),.3)

    def test_default_delay_remains_four(self):
        x,sr=self.fixture()
        _,info=sc.dereverb(x,sr)
        self.assertEqual(info['delay'],4)

    def test_invalid_delay_is_rejected(self):
        x,sr=self.fixture()
        for value in (0,1,9,2.5,True):
            with self.assertRaises(ValueError):sc.dereverb(x,sr,delay=value)

class TraceTests(unittest.TestCase):
    def test_level_gain_trace_replays_exact_processing(self):
        x,sr=DelayTests().fixture();trace={}
        y,_=sc.level_speech(x,x,sr,qa.masks(x,sr)[0],trace=trace)
        gain=trace['frame_gain_db'];size=round(sr*.02)
        self.assertEqual(len(gain),len(qa.frames(x,sr)))
        curve=np.interp(np.arange(len(x)),np.arange(len(gain))*size+size/2,gain,left=gain[0],right=gain[-1])
        replay=(x*10**(curve/20)).astype('float32')
        np.testing.assert_array_equal(y,replay)



class DiagnosticSafetyTests(unittest.TestCase):
    def test_diagnostic_directory_cannot_reuse_old_evidence(self):
        import tempfile
        from pathlib import Path
        import soundfile as sf
        import studio
        with tempfile.TemporaryDirectory() as d:
            root=Path(d);folder=root/'diagnostics';folder.mkdir();(folder/'old.wav').write_bytes(b'evidence')
            x,sr=DelayTests().fixture(.5);sf.write(root/'source.wav',x,sr,subtype='FLOAT')
            with self.assertRaises(ValueError):studio.render(root/'source.wav',root/'out.wav',diagnostic_dir=folder)
            self.assertEqual((folder/'old.wav').read_bytes(),b'evidence')

    def test_fallback_diagnostics_identify_delivered_chain(self):
        import tempfile,json
        from pathlib import Path
        import soundfile as sf
        import studio
        with tempfile.TemporaryDirectory() as d:
            root=Path(d);x,sr=DelayTests().fixture(.5);sf.write(root/'source.wav',x,sr,subtype='FLOAT')
            result=studio.render(root/'source.wav',root/'out.wav',model_dir=root/'missing',diagnostic_dir=root/'diag')
            self.assertTrue(result['fallback'])
            manifest=json.loads((root/'diag/diagnostic-manifest.json').read_text())
            self.assertEqual(manifest['delivered_chain'],'source_cleanup_fallback')
            self.assertEqual(manifest['files']['delivered_master.wav']['role'],'delivered_master')
            self.assertEqual(manifest['files']['delivered_master.wav']['sha256'],result['output_sha256'])



class BoundedTailTests(unittest.TestCase):
    def test_tail_gain_is_bounded_and_air_is_not_filtered(self):
        x,sr=DelayTests().fixture();t=np.arange(len(x))/sr
        x=x+.01*np.sin(2*np.pi*9000*t)
        y,info=sc.bounded_room_tail(x.astype('float32'),sr)
        self.assertEqual(len(x),len(y));self.assertTrue(np.isfinite(y).all())
        self.assertGreaterEqual(info['minimum_gain_db'],-2.001)
        # Direct high-frequency projection stays essentially unchanged.
        basis=np.exp(-2j*np.pi*9000*t)
        ratio=abs(np.dot(y,basis))/abs(np.dot(x,basis))
        self.assertAlmostEqual(ratio,1,delta=.01)
        self.assertAlmostEqual(qa.alignment(x,y,sr)['lag_seconds'],0,places=2)

    def test_steady_signal_is_not_globally_dulled(self):
        sr=48000;t=np.arange(sr*3)/sr;x=(.05*np.sin(2*np.pi*1000*t)).astype('float32')
        y,info=sc.bounded_room_tail(x,sr)
        self.assertGreater(np.sqrt(np.mean(y[sr:2*sr]**2))/np.sqrt(np.mean(x[sr:2*sr]**2)),.99)

if __name__=='__main__':unittest.main()
