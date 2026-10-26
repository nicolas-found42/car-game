"""Compare unchanged 57-edition inputs across the geometry/gearing correction."""
import csv
import hashlib
import json
from collections import Counter
from pathlib import Path
import sys
sys.path.insert(0, "tools")
from fleet40_protocol import ROUTES
from input_equivalence import verify_pair

OUT = Path('docs/verification/fleet57-geometry-drivetrain')
ART = Path('artifacts/vehicle-reference/fleet57-geometry-drivetrain')
BASE = Path('docs/verification/fleet-57')
BASE_ART = Path('artifacts/vehicle-reference/fleet-57')
STUDY = Path('artifacts/fleet57-geometry-drivetrain-20261025')


def read(path):
    return json.loads(Path(path).read_text())


def keyed(rows, *keys):
    result = {tuple(r[k] for k in keys): r for r in rows}
    assert len(result) == len(rows)
    return result


def changes(before, after):
    return {k: {'before': before[k], 'after': v, 'delta': v-before[k]}
            for k, v in after.items()
            if isinstance(v, (float, int)) and not isinstance(v, bool)
            and isinstance(before.get(k), (float, int))}


def scenario_comparison():
    before = read(BASE/'results.json'); after = read(OUT/'results.json')
    old = keyed(before['runs'], 'car', 'case', 'repeat')
    new = keyed(after['runs'], 'car', 'case', 'repeat')
    assert new.keys() == old.keys() and len(new) == 2736
    assert after['scenario_successes'] == 2736 and after['unexpected_resets'] == 0
    assert all(r['identical_trajectories'] for r in after['aggregate'])
    same = Counter(); route_input_matches = 0; timed_proof = []
    for key, row in new.items():
        prior = old[key]
        assert row['scenario_sha256'] == prior['scenario_sha256'], key
        if row['case'] not in ROUTES:
            timed_proof.append(verify_pair(prior, row))
        else:
            route_input_matches += row['input_sha256'] == prior['input_sha256']
        same[row['car']] += row['trajectory_sha256'] == prior['trajectory_sha256']
    (OUT/'input-comparison.json').write_text(json.dumps({'timed_executions': len(timed_proof),
        'all_ready_relative_controls_equal': True,
        'raw_file_difference_count': sum(not v['whole_raw_input_hash_equal'] for v in timed_proof),
        'method': 'Compare controller and all effective controls per relative tick after each run readiness; retain warmup/readiness and raw file hashes.',
        'rows': timed_proof}, indent=2)+'\n')
    old_agg = keyed(before['aggregate'], 'car', 'case')
    cases = [{'car': r['car'], 'case': r['case'],
              'metrics': changes(old_agg[r['car'], r['case']], r)} for r in after['aggregate']]
    summary = {'executions': len(new), 'paired_repeat_groups': len(after['aggregate']),
               'scenario_successes': after['scenario_successes'], 'unexpected_resets': after['unexpected_resets'],
               'assertions': sum(r['assertions_executed'] for r in new.values()),
               'routes': sum('route_passed' in r for r in new.values()),
               'requested_resets': sum(r['requested_resets'] for r in new.values()),
               'ticks': sum(r['ticks'] for r in new.values()),
               'all_repeat_trajectories_identical': True, 'all_scenario_hashes_unchanged': True,
               'all_timed_ready_relative_controls_equal': True,
               'closed_loop_route_input_hash_matches': route_input_matches,
               'closed_loop_route_input_note': 'Same courses and controller scenario; feedback commands may change with vehicle response.',
               'unchanged_baseline_trajectories_by_car': dict(same),
               'minimum_four_contact_pct': min(r['four_contact_pct'] for r in new.values()),
               'minimum_four_loaded_pct': min(r['four_loaded_pct'] for r in new.values()),
               'maximum_route_cross_track_m': max(r.get('maximum_cross_track_m', 0) for r in new.values())}
    return cases, summary


def fixture_comparison(kind, count, *keys):
    a = read(BASE_ART/f'{kind}-r1.json'); b = read(ART/f'{kind}-r1.json')
    assert (ART/f'{kind}-r1.json').read_bytes() == (ART/f'{kind}-r2.json').read_bytes()
    assert len(a) == len(b) == count
    old = keyed(a, *keys); new = keyed(b, *keys)
    assert old.keys() == new.keys()
    if kind == 'benchmark':
        for rows in [old, new]:
            for r in rows.values():
                r['sixty_to_hundred_mph_s'] = None if r['zero_100_mph_s'] is None else r['zero_100_mph_s']-r['zero_60_mph_s']
    return [{**dict(zip(keys, key)), 'metrics': changes(old[key], r), 'before': old[key], 'after': r}
            for key, r in new.items()]


def geometry_comparison():
    old = keyed(read(ART/'roster.json'), 'id'); new = keyed(read('assets/cars/roster.json'), 'id')
    assert old.keys() == new.keys() and len(new) == 57
    assert all(old[k] == r for k, r in new.items() if k != ('ref-a08',))
    assert (ART/'physics.json').read_bytes() == Path('assets/cars/physics.json').read_bytes()
    def measures(car):
        return {'wheelbase_m': car['wheels'][2][2]-car['wheels'][0][2],
                'front_track_m': car['wheels'][1][0]-car['wheels'][0][0],
                'rear_track_m': car['wheels'][3][0]-car['wheels'][2][0],
                'front_radius_m': car['radii'][0], 'rear_radius_m': car['radii'][2],
                'collider_half_width_m': car['half_width'], 'collider_half_length_m': car['half_length']}
    return {'same_editions': 57, 'unchanged_geometry_editions': 56, 'all_profiles_byte_identical': True,
            'c5': changes(measures(old['ref-a08',]), measures(new['ref-a08',])),
            'receipt': 'assets/cars/ref-a08.json'}


def metric_columns(metrics, prefix=''):
    return {f'{prefix}{key}_{state}': v
            for key, value in metrics.items() for state, v in value.items()}


def table(bench, skid, cases):
    names = {r['id']: r['name'] for r in read('assets/cars/roster.json')}
    radii = {r['car']: r for r in read(OUT/'radius-coupling.json')['rows']}
    rows = []
    for b in bench:
        car = b['car']; row = {'car': car, 'name': names[car], **radii[car]}
        row.update(metric_columns(b['metrics']))
        for s in (s for s in skid if s['car'] == car):
            row.update(metric_columns(s['metrics'], f'skidpad_{int(s["diameter_ft"])}ft_'))
        matching = [c for c in cases if c['car'] == car]
        for state in ['before', 'after']:
            row[f'maximum_scenario_tilt_deg_{state}'] = max(c['metrics']['peak_tilt_deg'][state] for c in matching)
            row[f'minimum_scenario_four_contact_pct_{state}'] = min(c['metrics']['four_contact_pct'][state] for c in matching)
        rows.append(row)
    with (OUT/'all-57-cars.csv').open('w') as sink:
        writer = csv.DictWriter(sink, sorted({k for r in rows for k in r})); writer.writeheader(); writer.writerows(rows)


def main():
    cases, matrix = scenario_comparison()
    bench = fixture_comparison('benchmark', 57, 'car')
    skid = fixture_comparison('skidpad', 114, 'car', 'diameter_ft')
    gt = next(r for r in bench if r['car'] == 'redline-gt500')['after']
    assert 4.0 <= gt['zero_60_mph_s'] <= 4.7
    assert 5.1 <= gt['sixty_to_hundred_mph_s'] <= 5.8
    assert 34 <= gt['brake_60_mph_0_m'] <= 38
    assert all(r['after']['supported_fraction'] == 1 for r in bench)
    assert all(r['after']['accepted_windows'] > 0 for r in skid)
    table(bench, skid, cases)
    result = {'matrix': matrix, 'geometry': geometry_comparison(), 'benchmarks': bench,
              'skidpads': skid, 'cases': cases, 'gt500_existing_guards_pass': True,
              'limits': ['Same seed17/dry map and exact same scenario inputs; not wet/rough-surface validation.',
                         'No rollout in acceleration fixtures; null high-speed endpoints are censored at20s.',
                         'AWD nominal coupling and unloaded nominal C5 tire radii remain approximations.',
                         'Corrected C5 shell envelope retains approximate source wheel arches.',
                         'Immutable evidence supports runtime material and rebuild completeness, not full portable runtime replay.']}
    (OUT/'comparison.json').write_text(json.dumps(result, indent=2)+'\n')
    print(json.dumps({'matrix': matrix, 'c5': next(r for r in bench if r['car']=='ref-a08'),
                      'gt500': next(r for r in bench if r['car']=='redline-gt500')}, indent=2))


if __name__ == '__main__':
    main()
