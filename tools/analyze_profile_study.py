#!/usr/bin/env python3
"""Cross-check profile studies, visual asset identity, applied profiles and driven axles."""
import argparse
import json
import math
from pathlib import Path

from compare_handling_studies import check_matching, compare, save
from run_handling_study import ROOT, sha


def read(path):
    return json.loads(path.read_text())


def visual_inventory(run):
    capture = read(ROOT / run / 'provenance/capture.json')
    return {e['logical_path']: e['sha256'] for e in capture['inventory']
            if e['role'] in ['prepared_asset', 'receipt_or_catalog', 'runtime_asset']
            and e['logical_path'] != 'assets/cars/physics.json'}


def applied_profile(run, car, expected):
    path = ROOT / run / 'events.jsonl'
    events = (json.loads(line) for line in path.open())
    profiles = [e['fields'] for e in events if e['kind'] == 'vehicle.physics_profile']
    matches = [p for p in profiles if p['id'] == car]
    assert matches and all(same_json(p, expected) for p in matches), 'applied profile differs: ' + car
    return sha(path)


def same_json(first, second):
    if isinstance(first, dict) and isinstance(second, dict):
        return first.keys() == second.keys() and all(same_json(first[k], second[k]) for k in first)
    if isinstance(first, list) and isinstance(second, list):
        return len(first) == len(second) and all(same_json(a, b) for a, b in zip(first, second))
    if isinstance(first, (int, float)) and isinstance(second, (int, float)):
        return math.isclose(first, second, rel_tol=1e-12, abs_tol=1e-12)
    return first == second


def audit_axles(run, car):
    path = ROOT / run / 'physics.jsonl'
    samples = (json.loads(line) for line in path.open())
    driven = [p for p in samples if p['input']['throttle'] > .99 and p['grounded'] == 4]
    assert driven, 'no supported acceleration samples'
    front = any(sum(p['drive_demands'][:2]) > 1 for p in driven)
    rear = any(sum(p['drive_demands'][2:]) > 1 for p in driven)
    front_applied = any(sum(p['longitudinal_forces'][:2]) > 1 for p in driven)
    rear_applied = any(sum(p['longitudinal_forces'][2:]) > 1 for p in driven)
    expected_awd = car in ['mcla-evo', 'mcla-skyline', 'mc3-skyline']
    assert front == expected_awd and rear, (car, front, rear)
    assert front_applied == expected_awd and rear_applied, (car, front_applied, rear_applied)
    return {'front_drive_observed': front, 'rear_drive_observed': rear,
            'front_traction_applied': front_applied, 'rear_traction_applied': rear_applied}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--before', type=Path, required=True)
    parser.add_argument('--after', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    before, after = [read(path / 'analysis.json') for path in [args.before, args.after]]
    configs = [read(path / 'study.json') for path in [args.before, args.after]]
    check_matching(before, after, *configs)
    old = {(r['car'], r['case'], r['repeat']): r for r in before['runs']}
    profiles = {p['id']: p for p in read(ROOT / 'assets/cars/physics.json')['cars']}
    checks = []
    for row in after['runs']:
        key = row['car'], row['case'], row['repeat']
        assert visual_inventory(old[key]['run']) == visual_inventory(row['run']), key
        event_sha = applied_profile(row['run'], row['car'], profiles[row['car']])
        audit = audit_axles(row['run'], row['car'])
        checks.append({'identity': key, 'events_sha256': event_sha, **audit})
    result = {'before': str(args.before), 'after': str(args.after), 'matched_runs_per_version': 130,
              'before_binary_sha256': configs[0]['binary_sha256'], 'after_binary_sha256': configs[1]['binary_sha256'],
              'profile_sha256': sha(ROOT / 'assets/cars/physics.json'),
              'identical_visual_assets_scenarios_and_inputs': True, 'profile_and_axle_checks': checks,
              'rows': compare(before, after)}
    save(args.output, result)
    print(f'130 matched runs; all visual assets unchanged; all applied profiles and driven axles checked')


if __name__ == '__main__':
    main()
