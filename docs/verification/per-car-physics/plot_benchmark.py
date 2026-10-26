import json,pathlib
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
root=pathlib.Path('docs/verification/per-car-physics');d=json.load(open(root/'real-reference-comparison.json'))
rows=d['measurements'];ids=[r['car'] for r in rows]
labels={'350z':'Nissan 350Z','fr2-cobra':'SVT Cobra (proxy)','fr2-fr500':'Mustang FR500 concept','gt-rx7':'RX-7 A spec (game proxy)','gt-viper':'Viper GTS','gt4-m3':'BMW M3 CSL','gt4-nsx':'Acura NSX (1991)','mc3-skyline':'Skyline GT-R (MC3 proxy)','mc3-supra':'Supra Turbo (proxy)','mcla-evo':'Lancer Evolution IX','mcla-skyline':'Skyline V-spec (1999)','redline-911':'Porsche 911 SC','redline-gt500':'Shelby GT500'}
plt.rcParams.update({'font.family':'DejaVu Sans','font.size':10,'axes.spines.top':False,'axes.spines.right':False})
fig,axes=plt.subplots(1,2,figsize=(13,7.7),sharey=True)
for ax,metric,title,xlabel in [(axes[0],'zero_60_mph_s','Acceleration: 0–60 mph','Seconds'),(axes[1],'brake_70_mph_0_m','Braking: 70–0 mph','Metres')]:
 ax.barh(range(len(rows)),[r[metric] for r in rows],color='#456c9b',height=.65,label='Simulation')
 refs={r['car']:r for r in d['rows'] if r['metric']==metric}
 for i,id in enumerate(ids):
  if id in refs:ax.scatter(refs[id]['reference'],i,s=65,marker='D',color='#d88728',zorder=3)
 ax.set(title=title,xlabel=xlabel);ax.grid(axis='x',alpha=.18);ax.set_axisbelow(True);ax.invert_yaxis()
axes[0].set_yticks(range(len(rows)),[labels[id] for id in ids]);axes[0].scatter([],[],s=65,marker='D',color='#d88728',label='Published claim/test (scope qualified)');axes[0].legend(loc='lower left',bbox_to_anchor=(0,-.16),frameon=False,ncol=2)
fig.suptitle('Distinct profiles: measured behavior versus available real references',fontsize=15,x=.03,ha='left',y=.98)
fig.text(.03,.025,'Two identical 120 Hz production-physics fixture repeats. No rollout. Source loading/trim/test methods differ; these are comparisons, not acceptance limits.\nM3/911 acceleration references use 0–100 km/h; GT500 braking reference uses 60–0 mph. Those remain in the full comparison table.',fontsize=9,color='#555')
fig.tight_layout(rect=[.01,.09,1,.94]);fig.savefig(root/'benchmark-reference.png',dpi=160);plt.close(fig)
