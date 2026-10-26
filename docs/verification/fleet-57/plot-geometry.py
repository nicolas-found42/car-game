"""Source-backed geometry comparison; no runtime parameters are changed."""
import json
from pathlib import Path
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
import numpy as np
out=Path('docs/verification/fleet-57')
g=json.load(open(out/'coverage-and-geometry.json'))
t=json.load(open(out/'track-geometry.json'))
r=json.load(open(out/'tire-geometry.json'))
p=json.load(open(out/'geometry-inspection.json'))
car=next(x for x in g['rows'] if x['car']=='ref-a08')
tracks={x['axle']:x for x in t['rows'] if x['car']=='ref-a08'}
tires={x['axle']:x for x in r['rows'] if x['car']=='ref-a08'}
fig,axes=plt.subplots(1,2,figsize=(12,7),gridspec_kw={'width_ratios':[1,1.2]})
for observed,label,color,style in [(True,'Prepared source model','#426b95','-'),(False,'Real reference geometry','#d6841c','--')]:
    base=car['wheelbase_m'] if observed else car['reference_wheelbase_m']
    points=[]
    for axle,z in [('front',-base/2),('rear',base/2)]:
        track=tracks[axle]['observed_m' if observed else 'reference_m']
        x=np.array([-track/2,track/2]);axes[0].plot(x,[z,z],style,color=color,linewidth=2)
        points.extend([(v,z) for v in x])
    points=np.array(points)
    axes[0].plot(points[[0,2],0],points[[0,2],1],style,color=color,alpha=.45)
    axes[0].plot(points[[1,3],0],points[[1,3],1],style,color=color,alpha=.45)
    axes[0].scatter(points[:,0],points[:,1],s=70,label=label,color=color,zorder=3)
axes[0].set(aspect='equal',xlabel='Lateral X (m)',ylabel='Longitudinal Z (m)',title='Wheel-center layout (plan view)',xlim=(-1.1,1.1),ylim=(1.7,-1.7))
axes[0].legend(fontsize=9,loc='center');axes[0].grid(alpha=.18)
x=np.arange(2);values=[[tires[a][k] for a in ['front','rear']] for k in ['prepared_mesh_radius_m','nominal_unloaded_radius_m']]
for offset,v,label,color in [(-.18,values[0],'Mesh radius','#426b95'),(.18,values[1],'Nominal unloaded tire radius','#d6841c')]:
    bars=axes[1].bar(x+offset,v,width=.34,label=label,color=color)
    axes[1].bar_label(bars,fmt='%.3fm',padding=4,fontsize=10)
axes[1].set(xticks=x,xticklabels=['Front','Rear'],ylabel='Radius (m)',ylim=(0,.43),title='Source tires are larger than the reference sizes')
axes[1].legend(fontsize=9,loc='upper left');axes[1].grid(axis='y',alpha=.18);axes[1].set_axisbelow(True)
fig.suptitle('C5 Corvette Z06: geometry is an actionable physics input',fontsize=17,x=.03,ha='left')
fig.text(.03,.075,'Wheelbase: +5.65%. Track: −7.44% front / −7.74% rear. Radius: +8.88% front / +10.90% rear.\nThe current model divides wheel torque by mean mesh radius: the source sizes imply 9.0% less wheel force at equal torque and gear ratio.\nNominal tire sizes are not measured loaded rolling radii. Stock-versus-modified source identity needs verification before correction.\nPivot centers match selected wheel mesh bounds within millimetres; these differences are present in the authored geometry.',fontsize=10)
fig.tight_layout(rect=[0,.20,1,.94]);fig.savefig(out/'c5-geometry.png',dpi=170);plt.close(fig)
