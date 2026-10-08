"""Explicit post-result correction of the overlap reference, not model output."""
import numpy as np


def coherent_frame_loss(predictions,weights):
    total=sum(weights);merged=sum(x*w for x,w in zip(predictions,weights))/total
    coherent=sum(abs(x)*w for x,w in zip(predictions,weights))/total
    actual=float(np.sqrt(np.mean(merged**2)));bound=float(np.sqrt(np.mean(coherent**2)))
    return float(20*np.log10(max(actual,1e-30)/max(bound,1e-30)))


def assess(speakers,baseline=False):
    reasons=[]
    if set(speakers)!={'p232','p257'}:reasons.append('Both speakers required')
    for s,r in speakers.items():
        for name in ('clean','early','late','composite'):
            c=r.get('conditions',{}).get(name,{})
            if (not isinstance(c.get('technical_issues'),list) or 'paired' not in c
                    or not isinstance(c.get('preservation',{}).get('watchdog'),list)):
                reasons.append(f'{s}/{name}: QA incomplete');continue
            values=c['paired']
            if any(k not in values or not np.isfinite(values[k]) for k in ('output_si_sdr_db','improvement_db')):
                reasons.append(f'{s}/{name}: score incomplete');continue
            if c['technical_issues']:reasons.append(f'{s}/{name}: technical issue')
            join=c.get('join_assessment',{})
            bound=join.get('lower_bound_db')
            if bound is None or not np.isfinite(bound) or bound < -1:reasons.append(f'{s}/{name}: coherent join failed/incomplete')
            if name=='clean' and (c['preservation']['watchdog'] or values['output_si_sdr_db']<15):reasons.append(f'{s}/clean: preservation')
            if name=='early' and values['improvement_db'] < -1:reasons.append(f'{s}/early: degradation')
            if name in ('late','composite') and values['improvement_db']<1:reasons.append(f'{s}/{name}: benefit below1dB')
        if baseline:
            for name in ('late','composite'):
                old=r.get('new_treble',{}).get('conditions',{}).get(name,{})
                new=r.get('conditions',{}).get(name,{})
                if 'paired' not in old or 'paired' not in new or old.get('technical_issues'):
                    reasons.append(f'{s}/{name}: baseline incomplete');continue
                if new['paired']['output_si_sdr_db']-old['paired']['output_si_sdr_db']<1:
                    reasons.append(f'{s}/{name}: benefit over Treble below1dB')
    return dict(passed=not reasons,reasons=reasons,metric_correction=True,
        scope='Corrected coherent-sign reference; original arithmetic-energy rejection retained. No model/window/weights change.')
