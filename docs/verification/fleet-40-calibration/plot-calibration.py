import json
from pathlib import Path
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
out=Path('docs/verification/fleet-40-calibration');data=json.loads((out/'reference-deltas.json').read_text())
selected=[('ref-e04','zero_100_kmh_s','Murciélago 0–100 km/h'),('redline-gt500','zero_60_mph_s','GT500 0–60 mph'),('gt-viper','brake_70_mph_0_m','Viper 70–0 mph'),('350z','brake_70_mph_0_m','350Z 70–0 mph'),('ref-a14','brake_60_mph_0_m','CTS-V 60–0 mph'),('gt4-m3','top_speed_kmh','M3 CSL top speed'),('ref-e12','top_speed_kmh','RS4 top speed'),('redline-gt500','top_speed_kmh','GT500 top speed'),('ref-a16','top_speed_kmh','GNX top speed')]
rows=[next(r for r in data['performance']+data['envelopes'] if (r['car'],r['metric'])==(car,metric)) for car,metric,_ in selected]
labels=[r[2] for r in selected]+[{'350z':'350Z','gt-viper':'Viper','redline-gt500':'GT500','ref-a01':'Mustang GT'}[r['car']]+' steady circle' for r in data['handling']]
first=[r['before_delta_pct'] for r in rows]+[r['before_delta_pct'] for r in data['handling']]
last=[r['after_delta_pct'] for r in rows]+[r['after_delta_pct'] for r in data['handling']]
fig,ax=plt.subplots(figsize=(12,9));ys=list(range(len(labels)))
ax.barh([y-.17 for y in ys],first,height=.32,label='Before',color='#b77855');ax.barh([y+.17 for y in ys],last,height=.32,label='After',color='#3b7b8f')
ax.set_yticks(ys,labels);ax.invert_yaxis();ax.axvline(0,color='#555',lw=.8);ax.set_xlabel('Difference from source-qualified reference (%)');ax.grid(axis='x',alpha=.2);ax.set_axisbelow(True)
ax.set_title('Targeted calibration: performance and handling gaps');ax.legend(loc='lower right')
fig.text(.02,.015,'Negative time/distance = quicker/shorter; positive grip/top speed = stronger/faster. These are fitted calibration comparisons, not independent validation.\nRelated year, trim, transmission, and skidpad-method qualifications remain in reference-deltas.json.',fontsize=9)
fig.tight_layout(rect=[0,.07,1,1]);fig.savefig(out/'calibration-deltas.png',dpi=160)
