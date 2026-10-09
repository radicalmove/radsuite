"""Source-preserving revision 2: WPE room cleanup and bounded speech leveling."""
import time
import importlib.metadata
import numpy as np
from scipy import signal
import analysis as qa

WPE_CONFIG=dict(chunk_seconds=12,overlap_seconds=2,fft_size=1024,hop_size=256,
                taps=10,delay=4,iterations=2,psd_context=1,strength=.65)

def dereverb(x,sr,strength=.65,delay=4):
    if type(delay) is not int or not 2<=delay<=8:raise ValueError('WPE prediction delay must be an integer from 2 to 8 frames')
    if not 0<=strength<=1:raise ValueError('WPE blend strength must be between zero and one')
    if sr!=48000:raise ValueError('Studio WPE requires 48 kHz working PCM')
    if importlib.metadata.version('nara-wpe')!='0.0.11':raise ValueError('Studio r2 requires nara-wpe==0.0.11')
    from nara_wpe.utils import stft,istft
    from nara_wpe.wpe import wpe_v8
    t0=time.perf_counter();n=sr*12;overlap=sr*2;step=n-overlap
    out=np.zeros(len(x),dtype='float64');weights=np.zeros(len(x),dtype='float64')
    for start in range(0,len(x),step):
        stop=min(len(x),start+n);part=x[start:stop]
        if len(part)<1024:out[start:stop]+=part;weights[start:stop]+=1;continue
        spec=stft(part,size=1024,shift=256).T[:,None,:]
        # Keep air/transient details untouched; WPE acts on the voiced/room bands.
        estimate=spec.copy();freq=np.fft.rfftfreq(1024,1/sr)
        active=(freq>=80)&(freq<8000)
        estimate[active]=wpe_v8(spec[active],taps=10,delay=delay,iterations=2,psd_context=1)
        mixed=spec*(1-strength)+estimate*strength
        y=istft(mixed[:,0,:].T,size=1024,shift=256)[:len(part)]
        if len(y)!=len(part) or not np.isfinite(y).all():raise ValueError('WPE output is not finite or changes duration')
        fade=np.ones(len(part));width=min(overlap,len(part))
        if start>0:fade[:width]=np.linspace(0,1,width,endpoint=False)
        if stop<len(x):fade[-width:]=np.minimum(fade[-width:],np.linspace(1,0,width,endpoint=False))
        out[start:stop]+=y*fade;weights[start:stop]+=fade
        if stop==len(x):break
    usable=weights>1e-8;out[usable]/=weights[usable];out[~usable]=x[~usable]
    return out.astype('float32'),dict(WPE_CONFIG,strength=strength,delay=delay,prediction_delay_ms=delay*256/sr*1000,seconds=time.perf_counter()-t0)

def level_speech(x,source,sr,speech,trace=None):
    size=round(sr*.02);r=qa.frames(x,sr);dry=qa.frames(source,sr);n=min(len(r),len(speech))
    eligible=speech[:n]&(r[:n]>np.maximum(dry[:n]*.3,1e-5))
    # Speech-weighted centered 2 s energy, avoiding downward level estimates
    # from pauses. Gain acts on speech support only, with smoothed transitions.
    kernel=np.ones(100)
    count=signal.convolve(eligible.astype(float),kernel,mode='same')
    power=signal.convolve(r[:n]**2*eligible,kernel,mode='same')
    level=np.sqrt(power/np.maximum(count,1))
    valid=(count>5)&(level>1e-5)
    target=float(np.sqrt(np.percentile(level[valid],20)*np.percentile(level[valid],80))) if np.any(valid) else 1e-3
    gain=np.clip(20*np.log10(target/np.maximum(level,1e-8)),-8,8)
    gain=signal.convolve(np.pad(gain,(25,25),mode='edge'),np.ones(51)/51,mode='valid')
    support=(signal.convolve(eligible.astype(int),np.ones(13),mode='same')>0)&speech[:n]
    activation=signal.convolve(support.astype(float),np.ones(7)/7,mode='same')
    gain*=activation
    curve=np.interp(np.arange(len(x)),np.arange(n)*size+size/2,gain,left=gain[0] if n else 0,right=gain[-1] if n else 0)
    y=x*10**(curve/20)
    if trace is not None:trace['frame_gain_db']=gain.copy()
    return y.astype('float32'),dict(target_speech_rms_dbfs=qa.db(target),maximum_gain_db=float(gain.max()) if n else 0,
                                    minimum_gain_db=float(gain.min()) if n else 0,
                                    gain_span_db=float(np.ptp(gain)) if n else 0,
                                    eligible_frames=int(eligible.sum()),lookahead_seconds=1,window_seconds=2)

def presence_gain(x,sr):
    bands=qa.signal_metrics(x,sr)['bands_db_relative']
    # Heuristic corrective control, not an Adobe EQ estimate. Already-clear
    # speech is left alone; mid-heavy speech gets at most 2 dB of presence.
    ratio=bands['presence']-bands['mid']
    return float(np.clip(-11-ratio,0,2))

def presence(x,sr,gain_db=None):
    # Bounded peaking EQ, Q=.8 at 2.4 kHz. No bass lift or blanket air shelf.
    gain_db=presence_gain(x,sr) if gain_db is None else gain_db
    if gain_db==0:return x.astype('float32',copy=True)
    a=10**(gain_db/40);w=2*np.pi*2400/sr;alpha=np.sin(w)/(2*.8)
    b=np.array([1+alpha*a,-2*np.cos(w),1-alpha*a]);den=np.array([1+alpha/a,-2*np.cos(w),1-alpha/a])
    return signal.lfilter(b/den[0],den/den[0],x).astype('float32')

def bounded_room_tail(x,sr,maximum_attenuation_db=2.):
    """Bounded falling-energy control; not an isolated reverb estimator.

    Steady speech settles to unity and strong transients reset to unity.
    Reduce low/mid energy below its delayed history, with bounded release
    smoothing through modest rises. Leave >=4 kHz untouched; retain phase.
    """
    if sr!=48000:raise ValueError('Room-tail control requires 48 kHz working PCM')
    if not 0<=maximum_attenuation_db<=2:raise ValueError('Room-tail attenuation must be between zero and two dB')
    t0=time.perf_counter();f,_,spec=signal.stft(x,fs=sr,nperseg=1024,noverlap=768,boundary='zeros')
    power=abs(spec)**2;delayed=np.pad(power,((0,0),(2,0)))[:,:power.shape[1]]
    alpha=np.exp(-256/(sr*.120))
    late=signal.lfilter([1-alpha],[1,-alpha],delayed,axis=1)
    decay=np.clip((late-power)/np.maximum(late,1e-16),0,1)
    # Fade the control out before the sibilant/air region; this is not low-pass.
    band=np.clip((4000-f)/500,0,1);band[f<80]=0
    target=-maximum_attenuation_db*decay*band[:,None]
    smooth=signal.lfilter([.4],[1,-.6],target,axis=1)
    transients=power>1.35*late
    smooth=np.where(transients,0,smooth)
    smooth[f>=4000]=0
    gain=10**(smooth/20)
    _,y=signal.istft(spec*gain,fs=sr,nperseg=1024,noverlap=768)
    y=y[:len(x)].astype('float32')
    if len(y)!=len(x) or not np.isfinite(y).all():raise ValueError('Room-tail output changes duration or contains nonfinite samples')
    return y,dict(method='bounded_falling_energy_control',maximum_attenuation_db=maximum_attenuation_db,
                  minimum_gain_db=float(smooth.min()),upper_passthrough_hz=4000,
                  fft_size=1024,hop_size=256,history_delay_frames=2,decay_estimate_ms=120,
                  transient_ratio=1.35,release_smoothing_alpha=.6,attenuated_tf_fraction=float(np.mean(smooth<-.1)),
                  seconds=time.perf_counter()-t0)
