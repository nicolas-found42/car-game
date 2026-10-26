"""Show the selected calibration envelopes with matched measures and circle sizes."""
import json
from pathlib import Path
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
import numpy as np

out=Path('docs/verification/fleet57-reference-calibration')
r=json.load(open(out/'comparison.json')); targets=json.load(open(out/'reference-evaluation.json'))
b={v['car']:v for v in r['benchmarks']}; s={(v['car'],v['diameter_ft']):v for v in r['skidpads']}
refs={(v['car'],v['metric']):v['reference'] for v in targets['primary_rows'] if v['use_for_calibration']}
cars=['ref-a14','ref-e12']; x=np.arange(2); colors=['#9ba8b8','#167b88']
fig,axes=plt.subplots(2,2,figsize=(12,8),constrained_layout=True)
for i,ax in enumerate(axes.flat):
    reference=[]; ceiling=0
    for j,state in enumerate(['before','after']):
        if i==0:
            values=[b[c]['metrics']['zero_100_mph_s'][state] for c in cars]
            reference=[refs[c,'zero_100_mph'] for c in cars]
        elif i==1:
            values=[b[c]['metrics']['sixty_to_hundred_mph_s'][state] for c in cars]
            reference=[refs[c,'zero_100_mph']-refs[c,'zero_60_mph'] for c in cars]
        elif i==2:
            metrics=['brake_60_mph_0_m','brake_70_mph_0_m']
            values=[b[c]['metrics'][m][state] for c,m in zip(cars,metrics)]
            reference=[refs['ref-a14','brake_60_mph_0'],refs['ref-e12','brake_70_mph_0']]
        else:
            values=[s[c,d]['metrics']['peak_sustained_g'][state] for c,d in zip(cars,[200,300])]
            reference=[.88,.91]
        ceiling=max(ceiling,*values,*reference)
        bars=ax.bar(x+(j-.5)*.32,values,.32,label=state.title(),color=colors[j])
        ax.bar_label(bars,fmt='%.3f',padding=3,fontsize=9)
    ax.scatter(x,reference,color='#c16a16',marker='D',s=36,label='Qualified period target',zorder=4)
    labels=[['CTS-V','RS4'],['CTS-V','RS4'],['CTS-V: 60–0 mph','RS4: 70–0 mph'],['CTS-V: 200 ft circle','RS4: 300 ft circle']][i]
    ax.set_xticks(x,labels); ax.set_ylabel(['seconds','seconds','metres','g'][i])
    ax.set_title(['0–100 mph','60–100 mph interval','Matched standardized braking','Matched skidpad lateral grip'][i])
    ax.set_ylim(0,ceiling*1.22)
    ax.grid(axis='y',alpha=.2);ax.set_axisbelow(True)
axes[0,0].legend(loc='lower right',frameon=False,fontsize=9)
fig.suptitle('RS4 and CTS-V: qualified reference calibration',fontsize=17)
fig.savefig(out/'comparison.png',dpi=170)
plt.close(fig)
