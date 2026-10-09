import importlib
import unittest


class QualificationTests(unittest.TestCase):
    def setUp(self):
        try:self.m=importlib.import_module('tfgrid_controls')
        except ModuleNotFoundError:self.fail('TFGrid qualification gate missing')

    def fixture(self):
        return {s:dict(conditions={
            'clean':dict(technical_issues=[],preservation={'watchdog':[]},paired={'output_si_sdr_db':22.,'improvement_db':0.},processing={'trace':{'join_energy_protected_worst_db':-.1}}),
            **{k:dict(technical_issues=[],preservation={'watchdog':[]},paired={'output_si_sdr_db':20.,'improvement_db':(0. if k=='early' else 4.)},processing={'trace':{'join_energy_protected_worst_db':-.1}}) for k in ['early','late','composite']}},
            new_treble={'conditions':{k:{'technical_issues':[],'paired':{'output_si_sdr_db':18.}} for k in ['late','composite']}}) for s in ['p232','p257']}

    def test_retained_early_response_does_not_require_anechoic_improvement(self):
        x=self.fixture();self.assertTrue(self.m.gate(x,baseline=False)['passed'])

    def test_damaged_clean_voice_blocks_finnegan(self):
        x=self.fixture();x['p232']['conditions']['clean']['paired']['output_si_sdr_db']=10.
        self.assertFalse(self.m.gate(x,baseline=False)['passed'])

    def test_late_benefit_required_for_both_speakers(self):
        x=self.fixture();x['p257']['conditions']['late']['paired']['improvement_db']=.9
        self.assertFalse(self.m.gate(x,baseline=False)['passed'])

    def test_missing_or_weaker_baseline_comparison_cannot_qualify(self):
        x=self.fixture();del x['p257']['new_treble'];self.assertFalse(self.m.gate(x,baseline=True)['passed'])
        x=self.fixture();x['p257']['new_treble']['conditions']['late']['paired']['output_si_sdr_db']=19.5
        self.assertFalse(self.m.gate(x,baseline=True)['passed'])

    def test_destructive_join_blocks_finnegan(self):
        x=self.fixture();x['p232']['conditions']['early']['processing']['trace']['join_energy_protected_worst_db']=-2
        self.assertFalse(self.m.gate(x,baseline=False)['passed'])

    def test_missing_qa_fields_fail_closed(self):
        x=self.fixture();del x['p232']['conditions']['clean']['preservation']['watchdog']
        self.assertFalse(self.m.gate(x,baseline=False)['passed'])

    def test_late_exception_clears_previous_approval(self):
        self.assertTrue(hasattr(self.m,'mark_failed'))
        r={'accepted_for_finnegan':True,'qualification':{'passed':True}}
        self.m.mark_failed(r,ValueError('input changed'))
        self.assertFalse(r['accepted_for_finnegan']);self.assertFalse(r['qualification']['passed'])


if __name__=='__main__':unittest.main()
