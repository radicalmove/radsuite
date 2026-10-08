import importlib
import ast
import hashlib
import shutil
import tempfile
import json
import os
from pathlib import Path
import unittest
import numpy as np


class TFGridTests(unittest.TestCase):
    def setUp(self):
        try:self.m=importlib.import_module('tfgrid_trial')
        except ModuleNotFoundError:self.fail('TFGrid adapter missing')

    def test_invalid_audio_rejected(self):
        for x,sr in [(np.zeros(10),16000),(np.zeros((10,2)),48000),(np.array([]),48000),(np.array([np.nan]),48000)]:
            with self.subTest(shape=x.shape),self.assertRaises(ValueError):self.m.validate_audio(x,sr)

    def test_overlap_identity_reconstructs_short_exact_and_nonmultiple_lengths(self):
        for n in [17,48000,144000,144001,398282,475203]:
            x=np.random.default_rng(n).normal(0,.03,n).astype('float32')
            y,trace=self.m.overlap_apply(x,48000,lambda z:z.copy())
            self.assertEqual(x.shape,y.shape);self.assertEqual(y.dtype,np.float32)
            self.assertLess(float(np.max(abs(x-y))),1e-8)
            self.assertEqual(trace['windows'][-1]['end_sample'],n)
            self.assertEqual(trace['window_samples'],144000)
            self.assertEqual(trace['hop_samples'],96000)

    def test_overlap_applies_processor_once_each_without_level_matching(self):
        x=np.linspace(-.03,.03,475203,dtype='float32');seen=[]
        def process(z):seen.append(len(z));return z*.5
        y,t=self.m.overlap_apply(x,48000,process)
        self.assertLess(float(np.max(abs(y-x*.5))),1e-8)
        self.assertEqual(len(seen),len(t['windows']))
        self.assertTrue(all(n<=144000 for n in seen))

    def test_bad_window_output_cannot_be_stitched(self):
        x=np.ones(200000,dtype='float32')*.02
        for f in [lambda z:z[:-1],lambda z:np.full(len(z),np.nan,dtype='float32')]:
            with self.assertRaises(ValueError):self.m.overlap_apply(x,48000,f)

    def test_overlap_exposes_destructive_cancellation(self):
        x=np.random.default_rng(5).normal(0,.02,200000).astype('float32');counter=[0]
        def opposite(z):counter[0]+=1;return z if counter[0]==1 else -z
        y,t=self.m.overlap_apply(x,48000,opposite)
        self.assertIn('join_energy_worst_db',t)
        self.assertLess(t['join_energy_worst_db'],-20)

    def test_native48k_spectrum_roundtrip_preserves_nonmultiple_length(self):
        import torch
        x=torch.randn((1,48123))*.02;z=self.m.encode(x,48000)
        self.assertEqual(z.shape[-1],769)
        self.assertLess(float(torch.max(abs(self.m.decode(z,48000,48123)-x))),1e-7)

    def test_publisher_classes_preserved_and_strict_checkpoint_complete(self):
        root=Path(__file__).resolve().parents[2]/'docs/radcast/studio-experiment/tfgridnet-qualification'
        model=self.m.TFGridModel(root/'artifacts')
        self.assertEqual(model.info['tensor_count'],270);self.assertEqual(model.info['element_count'],8524352)
        orig=ast.parse((root/'artifacts/publisher-pinned-tfgridnetv3_separator.py').read_text())
        vendor=ast.parse((root/'vendor/radcast_tfgrid_vendor/model.py').read_text())
        for node in orig.body:
            if isinstance(node,ast.ClassDef):
                copy=next(n for n in vendor.body if getattr(n,'name',None)==node.name)
                self.assertEqual(ast.dump(node,include_attributes=False),ast.dump(copy,include_attributes=False))
        self.assertIsNone(model.last_trace)

    def test_manifest_rewrite_cannot_replace_pinned_configuration(self):
        root=Path(__file__).resolve().parents[2]/'docs/radcast/studio-experiment/tfgridnet-qualification'
        with tempfile.TemporaryDirectory() as folder:
            q=Path(folder);a=q/'artifacts';a.mkdir()
            shutil.copy2(root/'artifacts/config.yaml',a/'config.yaml');(a/'model.pth').symlink_to(root/'artifacts/model.pth')
            shutil.copytree(root/'vendor',q/'vendor')
            config=a/'config.yaml';config.write_text(config.read_text()+'\n# substituted\n')
            manifest=json.loads((root/'artifact-manifest.json').read_text());manifest['files']['config.yaml']['sha256']=hashlib.sha256(config.read_bytes()).hexdigest()
            (q/'artifact-manifest.json').write_text(json.dumps(manifest))
            with self.assertRaisesRegex(ValueError,'[Cc]onfig'):self.m.TFGridModel(a)

    def test_unsupported_device_rejected_before_artifact_access(self):
        with self.assertRaisesRegex(ValueError,'device'):self.m.TFGridModel(Path('/missing-artifacts'),device='cuda')

    @unittest.skipUnless(os.environ.get('RADSUITE_TEST_TFGRID')=='1','Enabled after local runtime smoke')
    def test_real_strict_model_repeatable_native_window(self):
        model=self.m.TFGridModel(Path(os.environ['RADSUITE_TFGRID_ARTIFACTS']))
        x=np.random.default_rng(83).normal(0,.01,48000).astype('float32')
        y=model.cleanup(x,48000);z=model.cleanup(x,48000)
        self.assertTrue(np.array_equal(y,z));self.assertEqual(len(y),len(x))
        self.assertTrue(np.isfinite(y).all());self.assertEqual(model.last_trace['frequency_bins'],769)
        self.assertFalse(model.info['output_peak_normalization'])


if __name__=='__main__':unittest.main()
