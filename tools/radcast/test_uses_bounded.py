"""Real, tiny untrained USES math comparisons; no pretrained audio inference."""
import importlib
from pathlib import Path
import sys
import unittest

import torch

VENDOR=Path(__file__).resolve().parents[2]/'docs/radcast/studio-experiment/uses-qualification/vendor'
sys.path.insert(0,str(VENDOR))
from radcast_uses_vendor import USESSeparator
from radcast_uses_vendor.tcn import GlobalLayerNorm


class UsesBoundedTests(unittest.TestCase):
    def setUp(self):
        try:self.backend=importlib.import_module('uses_bounded')
        except ModuleNotFoundError:self.fail('Bounded feature execution is not implemented')
        if torch.get_num_threads()!=4:torch.set_num_threads(4)
        torch.manual_seed(20261009)

    def separator(self,segment_size=4,num_spk=1):
        return USESSeparator(input_dim=9,num_spk=num_spk,enc_channels=4,bottleneck_size=4,
            num_blocks=2,num_spatial_blocks=1,segment_size=segment_size,memory_size=2,
            memory_types=2,rnn_type='lstm',bidirectional=True,hidden_size=4,att_heads=1,
            dropout=0.0,norm_type='cLN',activation='relu',ch_mode='tac',ch_att_dim=4,
            eps=1e-5,ref_channel=0).eval()

    def compare(self,frames,segment_size=4):
        separator=self.separator(segment_size)
        spectrum=torch.complex(torch.randn(1,frames,9)*.05,torch.randn(1,frames,9)*.03)
        with torch.inference_mode():
            expected,_,_=separator(spectrum,torch.tensor([frames]),additional={'mode':'dereverb'})
            actual,trace=self.backend.bounded_separator(separator,spectrum,mode='dereverb')
        difference=actual-expected[0]
        maximum=float(difference.abs().max());rms=float(torch.sqrt(torch.mean(difference.abs()**2)))
        self.assertEqual(actual.shape,spectrum.shape)
        self.assertTrue(torch.isfinite(actual).all())
        self.assertLessEqual(maximum,3e-6)
        self.assertLessEqual(rms,3e-7)
        self.assertEqual(trace['memory_indices'],[1])
        self.assertEqual(trace['memory_initialization_count'],1)
        self.assertEqual(trace['segment_count'],(frames+segment_size-1)//segment_size)
        self.assertEqual(trace['atf_block_calls'],trace['segment_count']*2)
        self.assertFalse(trace['upstream_uses_forward_called'])
        self.assertFalse(trace['upstream_separator_forward_called'])
        self.assertLessEqual(trace['maximum_post_encoder_frames'],min(frames,segment_size+2))
        self.assertEqual(trace['external_audio_chunks'],1)
        return maximum,rms

    def test_matches_original_on_multiple_segments_and_nondivisible_last_segment(self):
        for frames in (1,3,4,5,8,9,17):
            with self.subTest(frames=frames):self.compare(frames)

    def test_retains_upstream_64_frame_memory_schedule(self):
        for frames in (63,64,65,129):
            with self.subTest(frames=frames):self.compare(frames,segment_size=64)

    def test_temporal_convolution_halos_preserve_segment_border_impulses(self):
        separator=self.separator();spectrum=torch.zeros((1,9,9),dtype=torch.complex64)
        spectrum[0,0,0]=.8+.3j;spectrum[0,3,4]=-.4+.2j
        spectrum[0,4,8]=.9-.5j;spectrum[0,7,0]=.3+.7j;spectrum[0,8,8]=-.6+.1j
        with torch.inference_mode():
            expected,_,_=separator(spectrum,torch.tensor([9]),additional={'mode':'dereverb'})
            actual,trace=self.backend.bounded_separator(separator,spectrum)
        self.assertLessEqual(float((actual-expected[0]).abs().max()),3e-6)
        self.assertTrue(trace['post_encoder_time_halo']==trace['pre_decoder_time_halo']==1)
        self.assertTrue(trace['final_padding_after_bottleneck'])

    def test_dereverb_uses_group_one_and_preserves_history_across_segments(self):
        separator=self.separator();spectrum=torch.complex(torch.randn(1,9,9),torch.randn(1,9,9))
        with torch.no_grad():
            pattern=torch.arange(8,dtype=torch.float32).reshape(1,1,4,1,2)
            separator.uses.memory_tokens[0].copy_(pattern*.2)
            separator.uses.memory_tokens[1].copy_(pattern*-.3)
        with torch.inference_mode():
            dereverb,_,_=separator(spectrum,torch.tensor([9]),additional={'mode':'dereverb'})
            denoised,_,_=separator(spectrum,torch.tensor([9]),additional={'mode':'no_dereverb'})
            actual,trace=self.backend.bounded_separator(separator,spectrum)
        self.assertGreater(float((dereverb[0]-denoised[0]).abs().max()),1e-5)
        self.assertLessEqual(float((actual-dereverb[0]).abs().max()),3e-6)
        self.assertEqual(trace['memory_initialization_count'],1)
        self.assertEqual(trace['memory_reuse_count'],2)

    def test_rejects_other_modes_nonmono_and_global_normalization(self):
        separator=self.separator();spectrum=torch.zeros((1,5,9),dtype=torch.complex64)
        for mode in ('no_dereverb','both',None):
            with self.subTest(mode=mode),self.assertRaisesRegex(ValueError,'dereverb'):
                self.backend.bounded_separator(separator,spectrum,mode=mode)
        for invalid in (spectrum.real,spectrum.unsqueeze(2),spectrum.expand(2,-1,-1),spectrum[:,:0]):
            with self.subTest(shape=invalid.shape),self.assertRaises(ValueError):
                self.backend.bounded_separator(separator,invalid)
        separator.uses.layer_norm=GlobalLayerNorm(4)
        with self.assertRaisesRegex(ValueError,'channel'):
            self.backend.bounded_separator(separator,spectrum)
        with self.assertRaisesRegex(ValueError,'speaker'):
            self.backend.bounded_separator(self.separator(num_spk=2),spectrum)


if __name__=='__main__':unittest.main()
