import importlib
import unittest
import numpy as np


class RevisedJoinTests(unittest.TestCase):
    def setUp(self):
        try:self.m=importlib.import_module('tfgrid_qualification')
        except ModuleNotFoundError:self.fail('Documented coherent-reference assessment missing')

    def test_aligned_level_difference_is_not_cancellation(self):
        x=np.linspace(-.1,.1,960);a=x*.01;b=x
        self.assertAlmostEqual(self.m.coherent_frame_loss([a,b],[np.ones(960)*.5,np.ones(960)*.5]),0,places=10)

    def test_opposing_predictions_are_cancellation(self):
        x=np.linspace(-.1,.1,960)
        self.assertLess(self.m.coherent_frame_loss([x,-x],[np.ones(960)*.5,np.ones(960)*.5]),-20)

    def test_missing_qa_never_passes(self):
        self.assertFalse(self.m.assess({'p232':{'conditions':{}},'p257':{'conditions':{}}},baseline=False)['passed'])


if __name__=='__main__':unittest.main()
