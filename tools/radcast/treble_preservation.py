"""Reviewed adaptive faint-speech protection controller."""
import numpy as np
from scipy.ndimage import maximum_filter1d
import analysis as qa
from treble_model import validate_audio
CONFIG=dict(onset_relative_db=-9.,full_relative_db=-12.,maximum_original=.20,
            frame_seconds=.02,expansion_frames_each_side=8,smoothing_frames=5,
            energy_guard_max_loss_db=.1,offline_whole_recording_statistics=True)

def activation(relative_gain):
    return .2*np.clip((-np.asarray(relative_gain)-9)/3,0,1)


def sample_curve(weights,count,sr):
    if not len(weights):return np.zeros(count)
    size=round(sr*.02)
    return np.interp(np.arange(count),np.arange(len(weights))*size+size/2,
                     weights,left=weights[0],right=weights[-1])


def intervals(mask):
    d=np.diff(np.r_[False,mask,False].astype(int))
    return list(zip(np.flatnonzero(d==1),np.flatnonzero(d==-1)))


def apply(source,highpassed,model,sr,protection_reference=None):
    source=validate_audio(source,sr);highpassed=validate_audio(highpassed,sr);model=validate_audio(model,sr)
    if source.shape!=model.shape or highpassed.shape!=model.shape:
        raise ValueError('Original-derived stems must have identical sample counts')
    references={'original':source}
    if protection_reference is not None:
        reference=validate_audio(protection_reference,sr)
        if reference.shape!=source.shape:raise ValueError('Protection reference sample count changed')
        references['preserved_cleanup']=reference
    n=len(source)//960;weights=np.zeros(n);records=[];medians={}
    for label,reference in references.items():
        detector=qa.speech_window_gain(reference,model,sr) if n>=50 else dict(speech_window_gain_median_db=None)
        median=detector['speech_window_gain_median_db'];medians[label]=median
        if median is None:continue
        for start,gain in zip(detector['speech_window_starts_seconds'],detector['speech_window_gain_db']):
            relative=gain-median;weight=float(activation(relative))
            if weight<=0:continue
            first=round(start*50);last=min(n,first+50)
            weights[first:last]=np.maximum(weights[first:last],weight)
            records.append(dict(reference=label,start_seconds=start,end_seconds=start+1,
                                gain_db=gain,relative_gain_db=relative,original_weight=weight))
    if np.any(weights):
        weights=maximum_filter1d(weights,size=17,mode='constant',cval=0)
        weights=np.convolve(weights,np.ones(5)/5,mode='same')
    weights=np.clip(weights,0,.2)
    def mix(w):
        curve=sample_curve(w,len(source),sr)
        return ((1-curve)*model+curve*highpassed).astype('float32')
    y=mix(weights);speech,_=qa.masks(source,sr)
    if protection_reference is not None:speech|=qa.masks(reference,sr)[0]
    before=qa.frames(model,sr);original=qa.frames(source,sr)
    eligible=speech&(original>1e-5)&(before>1e-8)
    def conflicts(output):
        after=qa.frames(output,sr)
        delta=20*np.log10(np.maximum(after,1e-30)/np.maximum(before,1e-30))
        return eligible&(delta<-.1),delta
    bad,delta=conflicts(y);initial_bad=int(bad.sum());bypassed=[]
    for start,end in intervals(weights>0):
        if np.any(bad[max(0,start-1):min(n,end+1)]):
            weights[start:end]=0
            bypassed.append(dict(start_frame=int(start),end_frame_exclusive=int(end),
                                 start_seconds=start*.02,end_seconds=end*.02))
    if bypassed:y=mix(weights)
    remaining,delta=conflicts(y)
    if np.any(remaining):raise ValueError('Post-bypass blend still reduces protected model-frame energy')
    if y.shape!=source.shape or not np.isfinite(y).all():raise ValueError('Invalid preservation output')
    return y,weights,dict(config=CONFIG.copy(),median_eligible_model_gain_db=medians['original'],
        detector_reference_median_gains_db=medians,uses_preserved_cleanup_reference=protection_reference is not None,
        triggered_windows=records,initial_energy_guard_conflict_frames=initial_bad,
        energy_guard_bypassed_intervals=bypassed,remaining_energy_guard_conflicts=int(remaining.sum()),
        minimum_protected_candidate_vs_model_frame_gain_db=float(delta[eligible].min()) if np.any(eligible) else None,
        active_frame_count=int(np.count_nonzero(weights)),frame_count=n,
        active_intervals=[dict(start_seconds=int(a)*.02,end_seconds=int(b)*.02) for a,b in intervals(weights>0)],
        sample_count=len(y),note='Source-periodicity/modulation and frame energy are protection proxies, not semantic speech or reverb estimates.')
