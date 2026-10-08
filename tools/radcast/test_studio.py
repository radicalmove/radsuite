import tempfile
import unittest
from pathlib import Path
import numpy as np
import soundfile as sf
import studio

class StudioTests(unittest.TestCase):
    def test_upper_band_preservation(self):
        sr=48000;t=np.arange(sr)/sr
        x=(.1*np.sin(2*np.pi*9000*t)).astype('float32')
        y=studio.preservation_blend(x,np.zeros_like(x),sr)
        self.assertGreater(np.sqrt(np.mean(y*y))/np.sqrt(np.mean(x*x)),.65)
        self.assertEqual(len(x),len(y))

    def test_cleanup_does_not_boost_bass(self):
        sr=48000;t=np.arange(sr)/sr
        x=(.1*np.sin(2*np.pi*140*t)).astype('float32')
        y=studio.transparent_cleanup(x,sr)
        self.assertLessEqual(np.sqrt(np.mean(y*y)), np.sqrt(np.mean(x*x))*1.01)

    def test_stages_are_lossless_and_mastering_last(self):
        self.assertEqual(studio.STAGES[-1],'final_linear_loudness_true_peak')
        self.assertNotIn('reconstruction',studio.STAGES)
        self.assertEqual(studio.WORKING_RATE,48000)

    def test_missing_model_falls_back_with_report(self):
        with tempfile.TemporaryDirectory() as d:
            root=Path(d);sr=48000;t=np.arange(sr*2)/sr
            x=(.1*np.sin(2*np.pi*220*t)*(np.sin(2*np.pi*2*t)**2)).astype('float32')
            sf.write(root/'in.wav',x,sr,subtype='FLOAT')
            result=studio.render(root/'in.wav',root/'out.wav',model_dir=root/'missing')
            self.assertTrue(result['fallback'])
            self.assertIn('model_unavailable',str(result['warnings']))
            y,rate=sf.read(root/'out.wav')
            self.assertEqual(rate,sr);self.assertEqual(len(y),len(x))
            self.assertEqual(sf.info(root/'out.wav').subtype,'FLOAT')



class ExportTests(unittest.TestCase):
    def test_final_export_clipping_is_rejected(self):
        with tempfile.TemporaryDirectory() as d:
            root=Path(d);sr=48000;t=np.arange(sr*2)/sr
            x=.1*np.sin(2*np.pi*220*t)*(np.sin(2*np.pi*2*t)**2)
            sf.write(root/'source.wav',x,sr,subtype='FLOAT')
            sf.write(root/'bad.wav',x*20,sr,subtype='FLOAT')
            (root/'bad.qa.json').write_text('{}')
            with self.assertRaises(ValueError):studio.verify_export(root/'source.wav',root/'bad.wav',root/'bad.qa.json')

class ModelReproducibilityTests(unittest.TestCase):
    def test_wrong_checkpoint_is_rejected_before_model_import(self):
        with tempfile.TemporaryDirectory() as d:
            root=Path(d);(root/'checkpoints').mkdir();(root/'config.ini').write_text('bad config')
            (root/'checkpoints/model_120.ckpt.best').write_bytes(b'wrong model')
            with self.assertRaises(ValueError):studio.model_cleanup(np.zeros(48000,dtype='float32'),root)



class WorkspaceTests(unittest.TestCase):
    def test_only_verified_checkpoint_is_visible_to_loader(self):
        from unittest.mock import patch
        import analysis
        with tempfile.TemporaryDirectory() as d:
            root=Path(d);(root/'checkpoints').mkdir()
            (root/'config.ini').write_bytes(b'config')
            (root/'checkpoints/model_120.ckpt.best').write_bytes(b'pinned')
            (root/'checkpoints/model_121.ckpt.best').write_bytes(b'unverified')
            hashes={p:analysis.sha(root/p) for p in ('config.ini','checkpoints/model_120.ckpt.best')}
            with patch.dict(studio.EXPECTED_MODEL_HASHES,hashes,clear=True):
                with studio.pinned_model_workspace(root) as path:
                    self.assertEqual([p.name for p in (path/'checkpoints').iterdir()],['model_120.ckpt.best'])

class EditedExportTests(unittest.TestCase):
    def test_unexpected_edited_truncation_is_rejected(self):
        with tempfile.TemporaryDirectory() as d:
            root=Path(d);sr=48000;t=np.arange(sr*2)/sr;x=.1*np.sin(2*np.pi*220*t)
            sf.write(root/'source.wav',x,sr,subtype='FLOAT')
            sf.write(root/'truncated.wav',x[:sr],sr,subtype='FLOAT')
            (root/'report.json').write_text('{}')
            with self.assertRaises(ValueError):studio.verify_export(root/'source.wav',root/'truncated.wav',root/'report.json',edited=True)



class WavMetadataTests(unittest.TestCase):
    def test_float_writer_removes_volatile_peak_timestamp(self):
        import struct
        with tempfile.TemporaryDirectory() as d:
            p=Path(d)/'audio.wav'
            studio.write_float(p,np.zeros(48000,dtype='float32'),48000)
            raw=p.read_bytes();at=raw.find(b'PEAK')
            self.assertEqual(struct.unpack('<I',raw[at+12:at+16])[0],0)
            self.assertEqual(sf.info(p).subtype,'FLOAT')

if __name__=='__main__':unittest.main()
