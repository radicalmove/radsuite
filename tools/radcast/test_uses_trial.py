import importlib
import importlib.util
import inspect
import ast
import hashlib
import json
import os
from pathlib import Path
import shutil
import tempfile
import unittest
import numpy as np

ROOT=Path(__file__).resolve().parents[2]
QUALIFICATION=ROOT/'docs/radcast/studio-experiment/uses-qualification'
ARTIFACTS=QUALIFICATION/'artifacts'


def builder(test):
    path=QUALIFICATION/'build_vendor.py'
    if not path.is_file():test.fail('Audited vendor builder is not implemented')
    spec=importlib.util.spec_from_file_location('radcast_uses_vendor_builder',path)
    module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
    return module


class UsesVendorTests(unittest.TestCase):
    def test_build_preserves_every_selected_upstream_class_and_function_ast(self):
        build=builder(self)
        with tempfile.TemporaryDirectory() as temporary:
            out=Path(temporary)/'vendor'
            audit=build.build(ARTIFACTS,out)
            self.assertEqual(audit['code_revision'],'bfc13cecfd0a07ed8e21d733b0ce130a1c69211a')
            self.assertEqual(audit['model_family'],'legacy USES (ASRU 2023)')
            self.assertFalse(audit['is_uses2_checkpoint'])
            self.assertEqual(len(audit['symbols']),11)
            for record in audit['symbols']:
                original=ast.parse((ARTIFACTS/record['source_file']).read_text())
                generated=ast.parse((out/'radcast_uses_vendor'/record['vendor_file']).read_text())
                before=next(n for n in original.body if getattr(n,'name',None)==record['name'])
                after=next(n for n in generated.body if getattr(n,'name',None)==record['name'])
                self.assertEqual(ast.dump(before,include_attributes=False),ast.dump(after,include_attributes=False))
                expected=hashlib.sha256(ast.dump(before,include_attributes=False).encode()).hexdigest()
                self.assertEqual(record['ast_sha256'],expected)
            self.assertTrue((out/'radcast_uses_vendor'/'LICENSE').is_file())
            self.assertIn('Apache',(out/'radcast_uses_vendor'/'README.md').read_text())
            self.assertIn('Ported from https://github.com/ujscjj/DPTNet',
                          (out/'radcast_uses_vendor'/'dptnet.py').read_text())

    def test_build_rejects_modified_pinned_source_before_writing_package(self):
        build=builder(self)
        with tempfile.TemporaryDirectory() as temporary:
            folder=Path(temporary);source=folder/'artifacts';shutil.copytree(ARTIFACTS,source)
            path=source/'uses.py';path.write_text(path.read_text()+'\n# changed\n')
            with self.assertRaisesRegex(ValueError,'uses.py.*hash'):
                build.build(source,folder/'vendor')
            self.assertFalse((folder/'vendor').exists())


class UsesConstructionTests(unittest.TestCase):
    def setUp(self):
        self.module=importlib.import_module('uses_trial')
        if not hasattr(self.module,'UsesModel'):self.fail('Real USES checkpoint adapter is not implemented')

    def test_real_checkpoint_builds_strictly_without_model_forward(self):
        model=self.module.UsesModel(ARTIFACTS)
        self.assertEqual(model.info['tensor_count'],261)
        self.assertEqual(model.info['element_count'],3052492)
        self.assertFalse(model.model.training)
        self.assertEqual(model.separator.uses.memory_types,2)
        self.assertEqual(model.separator.uses.segment_size,64)
        self.assertEqual(model.info['fft'],1536)
        self.assertEqual(model.info['frequency_bins'],769)
        self.assertEqual(model.info['threads'],4)
        self.assertTrue(all(k.startswith('separator.') for k in model.model.state_dict()))
        self.assertTrue(all(p.device.type=='cpu' for p in model.model.parameters()))

    def test_config_or_weights_hash_mismatch_is_rejected_before_loading(self):
        with tempfile.TemporaryDirectory() as temporary:
            directory=Path(temporary)
            for name in ('config.yaml','20epoch.pth'):shutil.copy2(ARTIFACTS/name,directory/name)
            (directory/'config.yaml').write_text('separator: uses2\n')
            with self.assertRaisesRegex(ValueError,'config.yaml.*hash'):
                self.module.UsesModel(directory)
            shutil.copy2(ARTIFACTS/'config.yaml',directory/'config.yaml')
            (directory/'20epoch.pth').write_bytes(b'wrong checkpoint')
            with self.assertRaisesRegex(ValueError,'20epoch.pth.*hash'):
                self.module.UsesModel(directory)

    def test_explicit_dereverb_is_the_only_accepted_mode_before_any_forward(self):
        model=self.module.UsesModel(ARTIFACTS)
        x=np.ones(48000,dtype='float32')*.001
        for mode in ('no_dereverb','both','denoise',None):
            with self.subTest(mode=mode),self.assertRaisesRegex(ValueError,'dereverb'):
                model.cleanup(x,48000,mode=mode)
        self.assertEqual(model.last_trace,None)

    def test_vendor_tampering_is_rejected_before_import(self):
        with tempfile.TemporaryDirectory() as temporary:
            target=Path(temporary)/'vendor';shutil.copytree(QUALIFICATION/'vendor',target)
            path=target/'radcast_uses_vendor'/'uses.py';path.write_text(path.read_text()+'\n# changed\n')
            with self.assertRaisesRegex(ValueError,'vendor.*hash'):
                self.module.UsesModel(ARTIFACTS,vendor_dir=target)

    def test_bounded_backend_has_explicit_provenance_without_model_forward(self):
        if 'backend' not in inspect.signature(self.module.UsesModel).parameters:
            self.fail('Bounded backend selection is not implemented')
        # This case exercises absence of a proof. The real experiment may
        # already contain the successfully completed trained comparison.
        with tempfile.TemporaryDirectory() as temporary:
            isolated=Path(temporary)/'artifacts';isolated.mkdir()
            for name in ('config.yaml','20epoch.pth'):shutil.copy2(ARTIFACTS/name,isolated/name)
            model=self.module.UsesModel(isolated,vendor_dir=QUALIFICATION/'vendor',backend='bounded')
        self.assertEqual(model.info['execution_backend'],'bounded_features')
        self.assertFalse(model.info['trained_backend_equivalence_verified'])
        self.assertEqual(model.info['backend_sha256'],
                         hashlib.sha256((ROOT/'tools/radcast/uses_bounded.py').read_bytes()).hexdigest())
        self.assertEqual(model.last_trace,None)
        self.assertEqual(self.module.UsesModel(ARTIFACTS).info['execution_backend'],'upstream')

    def test_unknown_backend_is_rejected_before_artifact_access(self):
        if 'backend' not in inspect.signature(self.module.UsesModel).parameters:
            self.fail('Bounded backend selection is not implemented')
        with self.assertRaisesRegex(ValueError,'backend'):
            self.module.UsesModel(Path('/missing-artifact-folder'),backend='audio_chunks')

    def test_silence_trace_never_claims_upstream_forward_calls(self):
        for backend in ('upstream','bounded'):
            model=self.module.UsesModel(ARTIFACTS,backend=backend)
            result=model.cleanup(np.zeros(48000,dtype='float32'),48000)
            self.assertEqual(np.count_nonzero(result),0)
            self.assertFalse(model.last_trace['upstream_separator_forward_called'])
            self.assertFalse(model.last_trace['upstream_uses_forward_called'])
            self.assertEqual(model.last_trace['memory_indices'],[])

    def test_bounded_equivalence_metadata_binds_every_provenance_field(self):
        # Temporary metadata fixtures test binding, never create a qualification stamp.
        with tempfile.TemporaryDirectory() as temporary:
            directory=Path(temporary);artifacts=directory/'artifacts';artifacts.mkdir()
            for name in ('config.yaml','20epoch.pth'):shutil.copy2(ARTIFACTS/name,artifacts/name)
            args=dict(vendor_dir=QUALIFICATION/'vendor',backend='bounded')
            model=self.module.UsesModel(artifacts,**args)
            if 'adapter_sha256' not in model.info:self.fail('Execution evidence binding is not implemented')
            self.assertEqual(model.info['equivalence_report_sha256'],None)
            keys=('checkpoint_sha256','config_sha256','vendor_audit_sha256','adapter_sha256','uses_bounded_sha256','execution_config')
            record=dict(passed=True,**{key:model.info[key] for key in keys})
            report=directory/'bounded-equivalence.json';report.write_text(json.dumps(record))
            bound=self.module.UsesModel(artifacts,**args)
            self.assertTrue(bound.info['trained_backend_equivalence_verified'])
            self.assertEqual(bound.info['equivalence_report_sha256'],hashlib.sha256(report.read_bytes()).hexdigest())
            for key in keys:
                changed=dict(record);changed[key]='different'
                report.write_text(json.dumps(changed))
                with self.subTest(key=key),self.assertRaisesRegex(ValueError,'equivalence.*'+key):
                    self.module.UsesModel(artifacts,**args)
            report.write_text(json.dumps(dict(record,passed=False)))
            with self.assertRaisesRegex(ValueError,'equivalence.*passed'):
                self.module.UsesModel(artifacts,**args)


class UsesTrialTests(unittest.TestCase):
    def setUp(self):
        try:self.module=importlib.import_module('uses_trial')
        except ModuleNotFoundError:self.fail('USES adapter is not implemented')

    def test_rejects_wrong_rate_stereo_empty_and_nonfinite(self):
        for x,sr in [(np.zeros(48000),16000),(np.zeros((48000,2)),48000),(np.array([]),48000),(np.array([np.nan]),48000)]:
            with self.subTest(shape=x.shape,sr=sr),self.assertRaises(ValueError):self.module.validate_audio(x,sr)

    def test_native_rate_stft_covers_whole_band_and_roundtrips_exact_length(self):
        import torch
        x=torch.from_numpy(np.random.default_rng(8).normal(0,.02,48123).astype('float32')).unsqueeze(0)
        spectrum=self.module.encode(x,48000)
        self.assertEqual(spectrum.shape[-1],769)
        y=self.module.decode(spectrum,48000,x.shape[-1])
        self.assertEqual(y.shape,x.shape)
        self.assertLess(float(torch.max(abs(x-y))),1e-7)

    def test_variance_is_restored_once_without_mean_or_peak_normalization(self):
        import torch
        x=torch.tensor([[.03,.2,-.1,.01]],dtype=torch.float32)
        y,scale=self.module.normalize(x)
        self.assertTrue(torch.equal(scale,torch.std(x,dim=1,keepdim=True)))
        self.assertTrue(torch.allclose(y*scale,x))

    def test_silence_bypasses_variance_division(self):
        import torch
        y,scale=self.module.normalize(torch.zeros((1,48000)))
        self.assertTrue(torch.isfinite(y).all())
        self.assertEqual(float(y.abs().max()),0)

    def test_strict_loading_rejects_missing_extra_and_wrong_shapes(self):
        import torch
        model=torch.nn.Linear(3,2);state=model.state_dict()
        for bad in [{k:v for k,v in state.items() if k!='bias'},dict(state,extra=torch.ones(1)),dict(state,weight=torch.zeros((2,4)))]:
            with self.subTest(keys=list(bad)),self.assertRaises(ValueError):self.module.strict_load(model,bad)
        info=self.module.strict_load(model,state);self.assertEqual(info['tensor_count'],2)

    def test_early_and_late_controls_preserve_direct_arrival(self):
        x=np.zeros(48000,dtype='float32');x[0]=.1
        early,late=self.module.room_conditions(x,48000)
        self.assertEqual(len(early),len(x));self.assertEqual(len(late),len(x))
        self.assertEqual(float(early[0]),float(x[0]));self.assertEqual(float(late[0]),float(x[0]))
        self.assertAlmostEqual(float(early[672]),.025,places=7)
        self.assertAlmostEqual(float(early[1776]),.012,places=7)
        self.assertEqual(np.count_nonzero(early),3)
        self.assertEqual(np.count_nonzero(late[1:2400]),0)

    def test_late_impulse_control_matches_existing_float32_rir_exactly(self):
        from model_trial import synthetic_room
        x=np.zeros(48000,dtype='float32');x[0]=.1
        expected,_=synthetic_room(x,48000)
        expected[672]=0;expected[1776]=0
        _,late=self.module.room_conditions(x,48000)
        self.assertTrue(np.array_equal(late,expected))

    @unittest.skipUnless(os.environ.get('RADSUITE_TEST_USES')=='1','real USES test explicitly enabled after runtime smoke')
    def test_real_model_strict_mode_repeatability_and_shape(self):
        model=self.module.UsesModel(Path(os.environ['RADSUITE_USES_ARTIFACTS']))
        self.assertEqual(model.info['tensor_count'],261)
        self.assertEqual(model.info['element_count'],3052492)
        x=np.random.default_rng(24).normal(0,.001,48000).astype('float32')
        y=model.cleanup(x,48000);z=model.cleanup(x,48000)
        self.assertEqual(y.dtype,np.float32);self.assertEqual(len(y),len(x))
        self.assertTrue(np.isfinite(y).all());self.assertTrue(np.array_equal(y,z))
        self.assertEqual(model.last_trace['mode'],'dereverb')
        self.assertEqual(model.last_trace['memory_indices'],[1])


if __name__=='__main__':unittest.main()
