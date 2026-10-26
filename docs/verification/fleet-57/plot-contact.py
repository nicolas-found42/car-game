"""Visualize the controlled boundary-wall contact-loss result."""
import json
from pathlib import Path
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
import numpy as np
from matplotlib.patches import Rectangle
out=Path('docs/verification/fleet-57');art=Path('artifacts/vehicle-reference/fleet-57')
proof=json.load(open(out/'contact-investigation.json'));run=Path(proof['source_run'])
ready=next(e['fields']['tick'] for e in map(json.loads,(run/'events.jsonl').read_text().splitlines()) if e['kind']=='scenario.ready')
actual=[json.loads(s) for s in (run/'physics.jsonl').read_text().splitlines()]
loss=[r for r in actual if r['tick']>=ready+240 and r['grounded']<4]
probe=json.load(open(art/'contact-probe-r1.json'))
fig,axes=plt.subplots(1,2,figsize=(13,5.8),gridspec_kw={'width_ratios':[1.15,1]})
for surface,color,label in [('flat','#d6841c','Wall removed'),('wall','#426b95','Wall present')]:
    r=next(x for x in probe if x['case']=='trail-brake-right' and x['surface']==surface)
    p=np.array([x['position'] for x in r['samples'] if x['tick']>=240])
    axes[0].plot(p[:,0],p[:,2],color=color,label=label,linewidth=2)
    axes[1].step([x['tick']/120 for x in r['samples']],[x['grounded'] for x in r['samples']],where='post',color=color,label=label,linewidth=2)
axes[0].axhspan(479.5,480.5,color='#666',alpha=.5,label='Boundary wall')
axes[0].scatter([r['position'][0] for r in loss],[r['position'][2] for r in loss],s=22,c='#bd3d37',zorder=5,label='Recorded loss samples')
car=next(r for r in json.load(open('assets/cars/roster.json')) if r['id']=='ref-j13')
r=loss[0];q=np.array(r['rotation'][:3]);w=r['rotation'][3];points=[]
for x in [-car['half_width'],car['half_width']]:
    for y in [-.27,.27]:
        for z in [car['body_center_z']-car['half_length'],car['body_center_z']+car['half_length']]:
            v=np.array([x,y,z]);points.append(v+2*np.cross(q,np.cross(q,v)+w*v)+r['position'])
points=np.array(points);low=points.min(axis=0);high=points.max(axis=0)
axes[0].add_patch(Rectangle((low[0],low[2]),high[0]-low[0],high[2]-low[2],fill=False,edgecolor='#bd3d37',linestyle='--',label='Projected chassis bounds'))
axes[0].set(xlim=(-240,-222),ylim=(470,481),xlabel='World X (m)',ylabel='World Z (m)',title='Turn endpoint near the boundary wall',aspect='equal');axes[0].legend(fontsize=8,loc='lower left');axes[0].grid(alpha=.2)
axes[1].set(xlim=(7.5,7.95),ylim=(2.7,4.25),yticks=[3,4],xlabel='Time after readiness (s)',ylabel='Wheel ray contacts',title='Eight lost-contact samples = 66.7ms')
axes[1].legend(fontsize=9,loc='lower left');axes[1].grid(alpha=.2)
fig.suptitle('Subaru 22B: boundary-wall contact loss under brake-held reverse',x=.025,ha='left',fontsize=16)
fig.text(.025,.035,'Identical profile, initial state and controls; 120Hz. Wall-only and wall+curb reproduce the loss; flat and curb-only retain four contacts.\nThe recorded car is reversing at 0.56–0.66m/s under held braking. The rear-right ray misses; the chassis never becomes fully airborne.\nFixture poses are sampled after integration, while production poses are logged before it; the declared one-tick pose alignment is checked separately.',fontsize=9)
fig.tight_layout(rect=[0,.18,1,.93]);fig.savefig(out/'22b-wall-contact.png',dpi=170);plt.close(fig)
