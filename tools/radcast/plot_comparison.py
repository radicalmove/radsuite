"""Scientific diagnostic figure from the stable-offset 10–40 s excerpt."""
import argparse
import json
from pathlib import Path
import numpy as np
from scipy import signal
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
import analysis as qa

def main():
    p=argparse.ArgumentParser();p.add_argument('--report',required=True);p.add_argument('--out',required=True);args=p.parse_args()
    files=json.loads(Path(args.report).read_text())['files']
    source,sr,_=qa.decode(files['original']['path']);source=source[10*sr:40*sr]
    speech,_=qa.masks(source,sr);size=round(sr*.02)
    fig,(ax,bx)=plt.subplots(2,1,figsize=(11,8),layout='constrained')
    names={'original':'Original','optimized':'Optimized','natural_double_plus':'Natural++','reference':'Preferred reference','studio_v1':'Studio v1','studio_v1_tail':'Studio with mild tail control'}
    colors=['#666666','#b73535','#df9a25','#39834d','#326bcb','#7554a1']
    for (key,r),color in zip(files.items(),colors):
        x,rate,_=qa.decode(r['path'])
        start=10 if key=='original' else r['matched_excerpt']['output_start_seconds']
        x=x[round(start*rate):round((start+30)*rate)]
        n=min(len(x)//size,len(speech));blocks=x[:n*size].reshape(n,size)[speech[:n]]
        power=np.mean(abs(np.fft.rfft(blocks*np.hanning(size),n=4096))**2,axis=0)
        power=signal.convolve(power,np.ones(9)/9,mode='same');power/=max(power.sum(),1e-24)
        f=np.fft.rfftfreq(4096,1/rate)
        ax.semilogx(f,10*np.log10(np.maximum(power,1e-20)),label=names[key],color=color,alpha=.9)
        if key!='original':
            e=r['matched_excerpt'];bands=e['output']['bands_db_relative'];base=e['source']['bands_db_relative']
            bx.plot(list(bands),[bands[b]-base[b] for b in bands],'.-',label=names[key],color=color)
    ax.set(xlim=(80,16000),ylim=(-95,-5),ylabel='Speech spectrum energy share (dB)',title='RADcast: matched 30 s excerpt, loudness-independent spectral comparison')
    ax.axvline(8000,color='#aaa',ls='--');ax.grid(True,alpha=.2);ax.legend(ncol=3,fontsize=9)
    bx.axhline(0,color='#555',lw=1);bx.set(ylabel='Band share change from original (dB)',title='Changes in broad spectral balance; no implication of perceptual quality')
    bx.grid(True,alpha=.2)
    fig.savefig(args.out,dpi=160)

if __name__=='__main__':main()
