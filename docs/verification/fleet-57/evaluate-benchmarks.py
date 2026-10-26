import json,csv,hashlib,sys
from pathlib import Path
sys.path.insert(0,'tools')
import compare_fleet40_references as reference
art=Path('artifacts/vehicle-reference/fleet-57');out=Path('docs/verification/fleet-57')
def read(p):return json.loads(Path(p).read_text())
def write(p,v):Path(p).write_text(json.dumps(v,indent=2)+'\n')
assert read(art/'benchmark-r1.json')==read(art/'benchmark-r2.json')
assert read(art/'skidpad-r1.json')==read(art/'skidpad-r2.json')
m=read(art/'benchmark-r1.json');s=read(art/'skidpad-r1.json')
assert len(m)==57 and len(s)==114 and all(x['accepted_windows']>0 for x in s)
assert m[:40]==read('artifacts/vehicle-reference/gt500-midrange/after-benchmark-r1.json')
assert s[:80]==read('artifacts/vehicle-reference/gt500-midrange/after-skidpad-r1.json')
reference.FIELDS.update({'acceleration_0_60_mph_upper_bound':'zero_60_mph_s','reference_2006_acceleration_0_100_kmh_manual':'zero_100_kmh_s'})
r=reference.compare(m)
handling=[];by={(r['car'],r['diameter_ft']):r for r in s}
for target in read('docs/verification/fleet-40-calibration/handling-targets.json'):
 actual=by[target['car'],target['diameter_ft']]['peak_sustained_g'];handling.append({**target,'simulation_g':actual,'delta_pct':100*(actual/target['reference_g']-1)})
write(out/'reference-comparison.json',{'schema':1,'measurements':m,'rows':r,'benchmarks_identical':True,'cars_with_reference_metrics':len({x['car'] for x in r}),'limitations':'Source-qualified claims and tests; no rollout; related trims and prototype references remain qualified. Null speed endpoints are not reached within 20s.'})
with (out/'reference-comparison.csv').open('w') as f:
 w=csv.DictWriter(f,sorted({k for x in r for k in x}));w.writeheader();w.writerows(r)
write(out/'benchmark-evaluation.json',{'scenario_matrix_status':'independent; consult completion-summary.json when available','identical_benchmark_rows':57,'identical_circle_rows':114,'unchanged_prior_benchmark_rows':40,'unchanged_prior_circle_rows':80,'minimum_acceleration_four_contact_fraction':min(x['supported_fraction'] for x in m),'censored_endpoints':[{'car':x['car'],'metric':k,'horizon_s':20} for x in m for k,v in x.items() if v is None],'performance_reference_rows':len(r),'editions_with_performance_targets':len({x['car'] for x in r}),'largest_qualified_performance_gaps':sorted([x for x in r if x['delta_pct'] is not None],key=lambda x:abs(x['delta_pct']),reverse=True),'handling_comparisons':handling,'skidpad_measurements':s,'new_model_maximum_skidpad_tilt_deg':max(x['peak_tilt_deg'] for x in s[80:]),'minimum_circle_accepted_windows':min(x['accepted_windows'] for x in s)})
print('Benchmarks evaluated:',len(r),'reference comparisons for',len({x['car'] for x in r}),'editions')
