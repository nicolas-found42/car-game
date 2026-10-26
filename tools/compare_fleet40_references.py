#!/usr/bin/env python3
"""Compare standardized measurements to qualified reference facts, preserving variant gaps."""
import argparse
import csv
import json
from pathlib import Path

from run_handling_study import ROOT, sha

FIELDS={'acceleration_0_60_mph':'zero_60_mph_s','tested_0_60_mph':'zero_60_mph_s',
        'acceleration_0_100_kmh':'zero_100_kmh_s','acceleration_0_100_kmh_upper_bound':'zero_100_kmh_s',
        'acceleration_0_100_mph':'zero_100_mph_s','tested_0_100_mph':'zero_100_mph_s',
        'reference_2004US_acceleration_0_60_mph':'zero_60_mph_s',
        'reference_2006EU_acceleration_0_100_kmh':'zero_100_kmh_s',
        'tested_60_0_mph':'brake_60_mph_0_m','tested_70_0_mph':'brake_70_mph_0_m'}


def read(path):
    return json.loads(path.read_text())


def qualified_fact(profile, car, fact, observed):
    metric=FIELDS[fact['field']];actual=observed[profile['id']][metric]
    if actual is None:return None
    reference=fact['value']*(.3048 if fact['unit']=='ft' else 1.)
    source=next(s for s in car['sources'] if s['id']==fact['source_id'])
    note=fact.get('note','');bound=fact['field'].endswith('upper_bound')
    status='instrumented_test' if fact['field'].startswith('tested_') else 'manufacturer_claim'
    if fact['field'].startswith('reference_'):status='related_reference'
    if 'automatic' in note.lower():status='related_transmission_test'
    if 'prototype' in note.lower():status='related_prototype_test'
    return {'car':profile['id'],'metric':metric,'simulation':actual,'reference':reference,
            'unit':'m' if metric.startswith('brake') else 's','upper_bound':bound,
            'delta_pct':None if bound else 100*(actual/reference-1),
            'within_bound':actual<=reference if bound else None,'reference_status':status,
            'note':note,'identity_note':profile['applicability'],'source_id':source['id'],'source_url':source['url']}


def compare(measurements):
    observed={r['car']:r for r in measurements}
    original=read(ROOT/'docs/verification/per-car-physics/real-reference-comparison.json')['rows']
    rows=[]
    for reference in original:
        row=reference.copy();actual=observed[row['car']][row['metric']];row['simulation']=actual
        row['delta']=actual-row['reference']
        row['delta_pct']=100*(actual/row['reference']-1) if not row['upper_bound'] else None
        row['within_bound']=actual<=row['reference'] if row['upper_bound'] else None
        rows.append(row)
    corpus={c['id']:c for c in read(ROOT/'docs/vehicle-reference/real-profiles.json')['cars']}
    expansions=[read(ROOT/f'docs/verification/{name}/profile-expansion.json')['cars'] for name in ['fleet-40','fleet-57']]
    for profile in sum(expansions,[]):
        car=corpus[profile['corpus_reference']]
        for fact in car['facts']:
            if fact['field'] in FIELDS:
                row=qualified_fact(profile,car,fact,observed)
                if row:rows.append(row)
    return rows


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--first',type=Path,required=True);parser.add_argument('--repeat',type=Path,required=True)
    parser.add_argument('--output',type=Path,required=True);args=parser.parse_args()
    measurements=read(args.first);assert measurements==read(args.repeat),'Benchmark repeats differ'
    assert len(measurements)==len(read(ROOT/'assets/cars/roster.json'))
    rows=compare(measurements);args.output.mkdir(parents=True,exist_ok=True)
    result={'schema':1,'measurements':measurements,'rows':rows,'benchmarks_identical':True,
            'measurement_sha256':sha(args.first),'profile_sha256':sha(ROOT/'assets/cars/physics.json'),
            'cars_with_reference_metrics':len({r['car'] for r in rows}),
            'limitations':'No rollout; nominal selected masses; estimated tires/launch; related years, markets and transmissions stay qualified.'}
    (args.output/'reference-comparison.json').write_text(json.dumps(result,indent=2)+'\n')
    fields=sorted({k for r in rows for k in r})
    with (args.output/'reference-comparison.csv').open('w') as stream:
        writer=csv.DictWriter(stream,fields);writer.writeheader();writer.writerows(rows)
    print(f'{len(rows)} qualified comparisons across {result["cars_with_reference_metrics"]}/{len(measurements)} editions')


if __name__=='__main__':main()
