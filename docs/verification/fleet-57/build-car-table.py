"""Create a one-row-per-edition table from retained measurements and full scenario results."""
import csv
import json
from pathlib import Path

out=Path('docs/verification/fleet-57')

def read(path):
    return json.loads(Path(path).read_text())


def case_metrics(cases):
    worst=max(cases,key=lambda r:r['peak_tilt_deg'])
    routes=[r for r in cases if 'maximum_cross_track_m' in r]
    return {'scenario_cases':len(cases),'passed_case_pairs':sum(r['scenario_passed'] for r in cases),
            'identical_repeat_pairs':sum(r['identical_trajectories'] for r in cases),
            'worst_tilt_case':worst['case'],'maximum_tilt_deg':worst['peak_tilt_deg'],
            'minimum_four_contact_pct':min(r['four_contact_pct'] for r in cases),
            'minimum_four_loaded_pct':min(r['four_loaded_pct'] for r in cases),
            'maximum_route_error_m':max(r['maximum_cross_track_m'] for r in routes)}


def build():
    roster=read('assets/cars/roster.json')
    profiles={r['id']:r for r in read('assets/cars/physics.json')['cars']}
    measured={r['car']:r for r in read(out/'reference-comparison.json')['measurements']}
    benchmark=read(out/'benchmark-evaluation.json')
    circles={(r['car'],r['diameter_ft']):r for r in benchmark['skidpad_measurements']}
    results=read(out/'results.json')['aggregate']
    geometry={r['car']:r for r in read(out/'coverage-and-geometry.json')['rows']}
    rows=[]
    for index,car in enumerate(roster):
        key=car['id'];profile=profiles[key];params=profile['parameters']
        cases=[r for r in results if r['car']==key]
        row={'car':key,'name':car['name'],'source_game':car['game'],
             'corpus_reference':profile['corpus_reference'],'preserved_prior_edition':index<40,
             'drive_layout':'FWD' if params['front_drive']==1 else 'RWD' if params['front_drive']==0 else 'AWD',
             'mass_kg':params['mass_kg'],'power_kw':params['power_kw'],
             'longitudinal_mu_estimate':params['tire_mu'],'lateral_mu_estimate':params['lateral_mu'],
             'estimated_parameter_fields':geometry[key]['estimated_parameter_count'],
             **{k:v for k,v in measured[key].items() if k!='car'},
             'censored_100_mph_after_20s':measured[key]['zero_100_mph_s'] is None,
             'skidpad_200ft_g':circles[key,200]['peak_sustained_g'],
             'skidpad_300ft_g':circles[key,300]['peak_sustained_g'],
             **case_metrics(cases),
             'wheelbase_m':geometry[key]['wheelbase_m'],
             'reference_wheelbase_error_pct':geometry[key].get('wheelbase_error_pct')}
        rows.append(row)
    assert len(rows)==57 and all(r['scenario_cases']==24 for r in rows)
    with (out/'all-57-cars.csv').open('w') as sink:
        writer=csv.DictWriter(sink,list(rows[0]));writer.writeheader();writer.writerows(rows)
    return rows


if __name__=='__main__':
    print('Built per-car rows:',len(build()))
