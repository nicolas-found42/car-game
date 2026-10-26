import json,hashlib
from pathlib import Path
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
from matplotlib.patches import Rectangle
out=Path('docs/verification/fleet-40-calibration');art=Path('artifacts/vehicle-reference/fleet-40-calibration')
read=lambda p:json.loads(Path(p).read_text())
before=read('docs/verification/fleet-40/results.json')['runs']
after=[json.loads(s) for s in (art/'incremental-analysis.jsonl').read_text().splitlines()]
controls=read(art/'trail-surface-counterfactual.json');proof=[]
fig,axes=plt.subplots(1,2,figsize=(13,6.5))
for ax,case in zip(axes,['trail-brake-left','trail-brake-right']):
 samples={}
 for label,rows,color in [('Before',before,'#8b9ba3'),('After',after,'#bc6a3d')]:
  row=next(r for r in rows if r['car']=='redline-gt500' and r['case']==case and r['repeat']==1)
  trace=[json.loads(s) for s in (Path(row['run'])/'physics.jsonl').read_text().splitlines()]
  ax.plot([p['position'][0] for p in trace],[p['position'][2] for p in trace],label=label,color=color,lw=2)
  samples[label]=row
  if label=='After':
   bad=[p for p in trace if p['grounded']<4 and p['tick']>240]
   ax.scatter([p['position'][0] for p in bad],[p['position'][2] for p in bad],s=9,color='#b32634',label='Fewer than four ray contacts',zorder=4)
 flat=next(r for r in controls if r['case']==case and not r['urban_surfaces'])
 replica=next(r for r in controls if r['case']==case and r['urban_surfaces'])
 assert abs(replica['four_contact_pct']-samples['After']['four_contact_pct'])<1e-9
 assert flat['four_contact_pct']==100
 ax.plot([p['position'][0] for p in flat['samples']],[p['position'][2] for p in flat['samples']],ls='--',color='#34788a',label='Same after controls on flat ground',alpha=.8)
 ax.add_patch(Rectangle((-324,348),88,88,facecolor='#b7b8ab',alpha=.45,label='0.18 m sidewalk'))
 ax.axhspan(479.5,480.5,color='#444b52',alpha=.8,label='Perimeter wall')
 ax.set(xlim=(-305,-220),ylim=(428,484),aspect='equal',xlabel='World X (m)',ylabel='World Z (m)',title=case.replace('-',' ').title())
 ax.grid(alpha=.2)
 proof.append({'case':case,'before_four_contact_pct':samples['Before']['four_contact_pct'],'after_four_contact_pct':samples['After']['four_contact_pct'],'flat_four_contact_pct':flat['four_contact_pct'],'replica_four_contact_pct':replica['four_contact_pct'],'flat_peak_tilt_deg':flat['peak_tilt_deg'],'replica_peak_tilt_deg':replica['peak_tilt_deg'],'actual_peak_tilt_deg':samples['After']['peak_tilt_deg'],'actual_run':samples['After']['run'],'same_scenario_sha256':samples['Before']['scenario_sha256']==samples['After']['scenario_sha256']})
handles,labels=axes[0].get_legend_handles_labels();fig.legend(handles,labels,loc='lower center',ncol=2,fontsize=9)
fig.suptitle('GT500 fixed-input trail braking: map contact versus flat-ground stability')
fig.text(.02,.012,'Paths show the chassis origin; tires and bumpers can contact an obstacle before that origin crosses its edge.',fontsize=9)
fig.tight_layout(rect=[0,.18,1,.94]);fig.savefig(out/'trail-contacts.png',dpi=160)
(out/'trail-surface-proof.json').write_text(json.dumps({'car':'redline-gt500','production_profile_sha256':hashlib.sha256(Path('assets/cars/physics.json').read_bytes()).hexdigest(),'city_source_sha256':hashlib.sha256(Path('src/city.rs').read_bytes()).hexdigest(),'colliders':{'sidewalk_center':[-280,.09,392],'sidewalk_size':[88,.18,88],'wall_center':[0,1,480],'wall_size':[960,2,1]},'method':'Same scenario Drive controls through frozen production VehiclePlugin; change only the two nearby obstacle colliders in the flat fixture. No production source/scene/scenario changes.','cases':proof},indent=2)+'\n')
