"""Plot delivery changes and the explicitly qualified period grip envelope."""
import json
from pathlib import Path
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
OUT=Path('docs/verification/fleet57-next-calibration')
ART=Path('artifacts/vehicle-reference/fleet57-next-calibration')
BASE=Path('artifacts/vehicle-reference/fleet57-reference-calibration')
e=json.load(open(OUT/'reference-evaluation.json'))
fig,axes=plt.subplots(1,2,figsize=(12,5),constrained_layout=True)
x=[0,1,2];rows=e['primary_rows']
for shift,state,color in [(-.17,'before','#9ba8b8'),(.17,'after','#167b88')]:
    bars=axes[0].bar([v+shift for v in x],[r[state] for r in rows],.32,label=state.title(),color=color)
    axes[0].bar_label(bars,fmt='%.3f',padding=3)
axes[0].scatter(x,[r['reference'] for r in rows],color='#c16a16',marker='D',label='Related Track 6MT test',zorder=4)
axes[0].set_xticks(x,['0–60 mph','0–100 mph','60–100 mph interval'])
axes[0].set_ylabel('seconds');axes[0].set_ylim(0,17);axes[0].set_title('350Z: effective delivery correction')
old={(r['car'],r['diameter_ft']):r for r in json.load(open(BASE/'skidpad-r1.json'))}
new={(r['car'],r['diameter_ft']):r for r in json.load(open(ART/'skidpad-r1.json'))}
for shift,mapping,label,color in [(-.17,old,'Before','#9ba8b8'),(.17,new,'After','#167b88')]:
    bars=axes[1].bar([v+shift for v in [0,1]],[mapping['gt4-nsx',d]['peak_sustained_g'] for d in [200,300]],.32,label=label,color=color)
    axes[1].bar_label(bars,fmt='%.3f',padding=3)
axes[1].axhspan(.84,.95,color='#c16a16',alpha=.15,label='Qualified model envelope')
axes[1].set_xticks([0,1],['200 ft circle','300 ft circle']);axes[1].set_ylim(0,1.15);axes[1].set_ylabel('sustained lateral g')
axes[1].set_title('Early NSX: conservative period-tire estimate')
for ax in axes:
    ax.grid(axis='y',alpha=.2);ax.set_axisbelow(True);ax.legend(fontsize=8,loc='lower right')
fig.suptitle('Supported calibration; test equipment and protocol uncertainty retained',fontsize=14)
fig.savefig(OUT/'comparison.png',dpi=160);plt.close(fig)
