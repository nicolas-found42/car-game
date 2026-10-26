"""Plot the matched production replay; requires Matplotlib (version in receipt)."""
import hashlib
import json
from pathlib import Path
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt

ROOT=Path(__file__).resolve().parents[3]
ART=ROOT/'artifacts/nsx-gear-hunting-fix'
OUT=Path(__file__).resolve().parent


def sha(path):
    with path.open('rb') as stream:return hashlib.file_digest(stream,'sha256').hexdigest()


def replay(control):
    path=ROOT/control['run']/'physics.jsonl'
    rows=[json.loads(line) for line in path.read_text().splitlines()]
    start=min(r['tick'] for r in rows if r['input']['throttle']>.5)
    active=[r for r in rows if r['tick']>=start]
    return active,[(r['tick']-start)/120 for r in active],path


def main():
    controls=json.loads((ART/'ten-second-comparison.json').read_text())['controls']
    fig,axes=plt.subplots(2,2,figsize=(12,8))
    fig.subplots_adjust(top=.86,bottom=.12,hspace=.42,wspace=.25)
    sources={}
    for name,color,label in [('before','#B6383E','Before'),('after','#126A9E','After')]:
        rows,time,path=replay(controls[name]);sources[str(path.relative_to(ROOT))]=sha(path)
        gear=[r['gear'] for r in rows]
        axes[0,0].step(time,gear,where='post',color=color,label=label,lw=1.3)
        axes[0,1].step(time,gear,where='post',color=color,lw=1.3)
        axes[1,0].plot(time,[r['decisions']['engine_demand_n']/1000 for r in rows],color=color,lw=1.1)
        axes[1,1].plot(time,[r['speed']*3.6 for r in rows],color=color,lw=1.5)
    titles=['Gear selection: full replay','First reversal: zoom','Engine demand before traction limits','Forward speed']
    labels=['Forward gear','Forward gear','Engine demand (kN)','Speed (km/h)']
    for axis,title,label in zip(axes.flat,titles,labels,strict=True):
        axis.set(title=title,xlabel='Seconds after throttle command',ylabel=label,xlim=(0,10))
        axis.grid(alpha=.22);axis.spines[['top','right']].set_visible(False)
    axes[0,0].set(yticks=[1,2,3,4],ylim=(.8,4.2))
    axes[0,1].set(xlim=(3.25,3.9),yticks=[1,2],ylim=(.8,2.2))
    axes[0,1].axvline(3.3,color='#555555',ls=':',lw=.8)
    axes[1,0].set_ylim(bottom=0);axes[1,1].set_ylim(bottom=0)
    fig.suptitle('NSX-R LM GT2: 252 → 3 forward gear changes',fontsize=16,y=.98)
    fig.legend(*axes[0,0].get_legend_handles_labels(),loc='upper center',bbox_to_anchor=(.5,.95),ncol=2,frameon=False)
    fig.text(.08,.035,'Same seed, initial state and controls. Force decisions precede integration. Diagnostic replay, not real-world calibration.',fontsize=9,color='#444444')
    image=OUT/'shift-diagnosis.png';fig.savefig(image,dpi=150,facecolor='white');plt.close(fig)
    proof={'matplotlib_version':matplotlib.__version__,'input_hashes':sources,'image_sha256':sha(image)}
    (ART/'plot-proof.json').write_text(json.dumps(proof,indent=2)+'\n')
    print(image)


if __name__=='__main__':main()
