#!/usr/bin/env python3
"""Compare actual production-physics benchmark measurements with qualified source facts."""
import argparse
import csv
import json
from pathlib import Path

from run_handling_study import ROOT, sha

FIELDS = {
    'zero_to_60_mph': 'zero_60_mph_s', 'zero_to_100_kmh': 'zero_100_kmh_s',
    'zero_to_100_mph': 'zero_100_mph_s',
    'braking_70_to_0_mph': 'brake_70_mph_0_m', 'braking_60_to_0_mph': 'brake_60_mph_0_m',
    'tested_acceleration_0_60_mph': 'zero_60_mph_s',
    'tested_acceleration_0_100_mph': 'zero_100_mph_s',
    'tested_braking_70_0_mph': 'brake_70_mph_0_m',
    'acceleration_0_100_kmh': 'zero_100_kmh_s',
    'related_mr_test_acceleration_0_60_mph': 'zero_60_mph_s',
    'related_mr_test_braking_70_0_mph': 'brake_70_mph_0_m',
    'braking_100_0_distance_upper_bound': 'brake_100_0_m',
}


def read(path):
    return json.loads(path.read_text())


def reference_rows(car, measurement, sources):
    rows = []
    for fact in car['performance_references'] + car['real_facts']:
        metric = FIELDS.get(fact['field'])
        if metric is None:
            continue
        value = fact['value'] * (0.3048 if fact['unit'] == 'ft' else 1.)
        actual = measurement[metric]
        source = sources[fact['source_id']]
        bound = fact['field'].endswith('upper_bound')
        status = fact.get('status') or qualified_status(car['fleet_id'], fact['field'], bound)
        rows.append({'car': car['fleet_id'], 'metric': metric, 'simulation': actual,
                     'reference': value, 'unit': 'm' if 'brake' in metric else 's',
                     'delta': actual - value if actual is not None else None,
                     'delta_pct': 100 * (actual/value-1) if actual is not None and not bound else None,
                     'upper_bound': bound, 'within_bound': actual <= value if bound and actual is not None else None,
                     'reference_status': status, 'note': fact.get('note', ''),
                     'identity_note': car['identity_notes'], 'source_id': fact['source_id'],
                     'source_url': source['url']})
    return rows


def qualified_status(car, field, bound):
    if field.startswith('related_mr_test_'):
        return 'measured_related_mr_trim'
    if car == '350z':
        return 'measured_related_equipment_grade'
    if car == 'gt4-m3':
        return 'manufacturer_upper_bound' if bound else 'manufacturer_claim'
    return 'source_qualified_corpus_fact'


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--first', type=Path, required=True)
    parser.add_argument('--repeat', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    first, repeat = read(args.first), read(args.repeat)
    assert first == repeat, 'benchmark trajectories differ across repeats'
    supplement = ROOT / 'docs/verification/per-car-physics/fleet-reference-supplement.json'
    references = read(supplement)
    observed = {r['car']: r for r in first}
    assert set(observed) == {c['fleet_id'] for c in references['cars']} and len(observed) == 13
    rows = [row for car in references['cars'] for row in reference_rows(car, observed[car['fleet_id']], references['sources'])]
    result = {'schema': 1, 'two_benchmarks_identical': True, 'rows': rows, 'measurements': first,
              'reference_sha256': sha(supplement), 'benchmark_sha256': sha(args.first),
              'profile_sha256': sha(ROOT / 'assets/cars/physics.json'),
              'benchmark_source_sha256': sha(ROOT / 'examples/fleet_benchmark.rs'),
              'not_directly_comparable': 'No rollout, nominal selected reference mass, estimated tire/launch models; related trim/year and claims remain labeled. Turning/tilt lack matched real reference tests.'}
    args.output.mkdir(exist_ok=True, parents=True)
    (args.output / 'real-reference-comparison.json').write_text(json.dumps(result, indent=2) + '\n')
    with (args.output / 'real-reference-comparison.csv').open('w') as stream:
        writer = csv.DictWriter(stream, list(rows[0])); writer.writeheader(); writer.writerows(rows)
    print(f'{len(rows)} source-qualified comparisons; {len({r["car"] for r in rows})}/13 cars have comparable metric definitions')


if __name__ == '__main__':
    main()
