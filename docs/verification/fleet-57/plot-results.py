import json
from pathlib import Path
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
import numpy as np
out=Path('docs/verification/fleet-57');art=Path('artifacts/vehicle-reference/fleet-57')
def read(p):return json.loads(Path(p).read_text())
roster=read('assets/cars/roster.json');data=read(out/'reference-comparison.json');m={r['car']:r for r in data['measurements']};b=read(out/'benchmark-evaluation.json');s={(r['car'],r['diameter_ft']):r for r in b['skidpad_measurements']}
fig,axes=plt.subplots(1,3,figsize=(20,20),sharey=True)
colors=['#426b95']*40+['#27856e']*17
for ax,field,title in zip(axes,['zero_60_mph_s','brake_70_mph_0_m','g'],['0–60 mph (seconds)','70–0 mph (metres)','200-ft circle (sustained g)']):
 values=[s[c['id'],200]['peak_sustained_g'] if field=='g' else m[c['id']][field] for c in roster]
 ax.barh(range(57),values,color=colors,height=.7)
 for i,c in enumerate(roster):
  refs=[r for r in (b['handling_comparisons'] if field=='g' else data['rows']) if r['car']==c['id'] and (r.get('metric')==field or (field=='g' and r['diameter_ft']==200))]
  for r in refs:ax.scatter(r.get('reference',r.get('reference_g')),i,c='#d6841c',s=23,marker='<' if r.get('upper_bound') else 'D',zorder=3)
 ax.axhline(39.5,color='#999',linewidth=.8);ax.set_title(title);ax.grid(axis='x',alpha=.2);ax.set_axisbelow(True)
axes[0].set_yticks(range(57),[c['name'] for c in roster],fontsize=8.5);axes[0].invert_yaxis()
fig.suptitle('57 car editions • 50 real reference cars • production physics',x=.02,ha='left',fontsize=18)
fig.text(.02,.016,'Blue: preserved 40 editions. Green: 17 additions. Orange: qualified reference targets (left triangle = upper bound).\n120 Hz; two exact fixture repeats; no rollout. Related trims and prototype tests differ. Most new tire and chassis settings are estimates.\nAll acceleration runs retained four wheel-ray contacts. These measurements do not establish wet, loose-surface, impact, or complete real handling fidelity.',fontsize=10)
fig.tight_layout(rect=[0,.06,1,.97]);fig.savefig(out/'benchmarks-57.png',dpi=150);plt.close(fig)
if (out/'results.json').exists():
 data=read(out/'results.json');cases=data['study']['cases'];r={(x['car'],x['case']):x for x in data['aggregate']}
 values=np.array([[r[c['id'],case]['peak_tilt_deg'] for case in cases] for c in roster]);fig,ax=plt.subplots(figsize=(19,20))
 im=ax.imshow(values,aspect='auto',cmap='YlOrRd',vmin=0)
 ax.set_yticks(range(57),[c['name'] for c in roster],fontsize=8);ax.set_xticks(range(24),cases,rotation=65,ha='right',fontsize=9);ax.axhline(39.5,color='#23765e',linewidth=1)
 for i,c in enumerate(roster):
  for j,case in enumerate(cases):
   row=r[c['id'],case]
   if not row['scenario_passed']:ax.text(j,i,'×',ha='center',va='center',color='blue')
   elif row['four_contact_pct']<100:ax.text(j,i,'○',ha='center',va='center',color='cyan')
 fig.colorbar(im,ax=ax,label='Peak chassis tilt (degrees)',shrink=.6);ax.set_title('57 cars × 24 scenarios × two exact repeats — peak tilt')
 fig.text(.02,.012,'Tilt combines pitch and roll and includes map impacts; smaller does not necessarily mean more realistic.\nBlue ×: failed assertion. Cyan circle: wheel-ray contact loss. Reset tails excluded. The line separates preserved editions from additions.',fontsize=10)
 fig.tight_layout(rect=[0,.05,1,.98]);fig.savefig(out/'scenario-matrix.png',dpi=150);plt.close(fig)
print('Figures rendered')
