#!/usr/bin/env python3
"""Audit matched inputs and evidence before computing fleet handling comparisons."""
import argparse
from collections import Counter
import csv
import hashlib
import json
from pathlib import Path
import statistics

from handling_metrics import clearance, measure, planar
from handling_protocol import CASES, scenario
from recorder_health import inspect_health
from run_handling_study import ROOT, sha


def load(path):
    return json.loads(path.read_text())


def lines(path):
    return [json.loads(line) for line in path.read_text().splitlines() if line.strip()]


def check_inputs(inputs, task, ready):
    definition = scenario(task['car'], task['case'])
    end = CASES[task['case']]['end_tick']
    by_tick = {row['tick'] - ready: row for row in inputs if ready < row['tick'] <= ready + end}
    assert len(by_tick) == end, 'missing input tick'
    assert len(by_tick) == sum(ready < r['tick'] <= ready + end for r in inputs), 'duplicate tick'
    held = {'throttle': 0.0, 'brake': 0.0, 'steer': 0.0, 'handbrake': False}
    timeline = {item['tick']: item['command'] for item in definition['commands']}
    canonical = []
    for tick in range(1, end + 1):
        command = timeline.get(tick, {})
        if command.get('action') == 'drive':
            held.update({k: command[k] for k in ['throttle', 'brake', 'steer']})
        expected = {**held, 'reset': command.get('action') == 'reset'}
        row = by_tick[tick]
        assert row['controller'] == 'scenario', (tick, row['controller'])
        assert all(row[key] == value for key, value in expected.items()), (tick, row, expected)
        canonical.append([tick, expected])
    return hashlib.sha256(json.dumps(canonical, sort_keys=True).encode()).hexdigest()


def evidence(task, study):
    run = ROOT / task['run']
    manifest = load(run / 'manifest.json')
    summary = load(run / 'scenario-summary.json')
    health = inspect_health(run, manifest)
    assert task['exit_code'] == 0 and manifest['exit']['code'] == 0, task
    assert summary['passed'] and summary['ticks'] == CASES[task['case']]['end_tick'], summary
    assert health['ok'], health
    assert manifest['binary_sha256'] == study['binary_sha256'], 'binary mismatch'
    capture = load(run / 'provenance/capture.json')
    assert capture['catalog_matches_embedded'], 'compiled catalog mismatch'
    ready = summary['readiness']['tick']
    events = lines(run / 'events.jsonl')
    states = {e['tick'] - ready: e['fields'] for e in events
              if e['kind'] == 'simulation.completed'}
    raw_physics = lines(run / 'physics.jsonl')
    physics = {row['tick'] - ready: row for row in raw_physics}
    assert len(physics) == len(raw_physics), 'duplicate physics tick'
    assert all(p['car'] == task['car'] for p in raw_physics), 'wrong physics car'
    expected_gaps = {1680} if task['case'] == 'brake' else set()
    assert set(range(1, summary['ticks'] + 1)) - physics.keys() == expected_gaps
    assert all(t in states for t in range(summary['ticks'] + 1)), 'completed state gap'
    inputs_sha = check_inputs(lines(run / 'input.jsonl'), task, ready)
    commands = [r for r in lines(run / 'commands.jsonl') if r['source'] == 'scenario']
    assert [r['command'] for r in commands] == [{'action': 'assign_controller', 'controller': 'scenario'}] + [
        {**i['command'], **({'handbrake': False} if i['command']['action'] == 'drive' else {})}
        for i in scenario(task['car'], task['case'])['commands']], 'scenario command mismatch'
    return run, ready, events, states, physics, inputs_sha, capture


def reset_metrics(events, states, case):
    resets = [e for e in events if e['kind'] == 'vehicle.reset']
    reasons = Counter(e['fields']['reason'] for e in resets)
    requested = reasons.get('requested-reset', 0)
    unexpected = sum(reasons.values()) - requested
    result = {'requested_resets': requested, 'unexpected_resets': unexpected}
    assert requested == (1 if case == 'brake' else 0), reasons
    result['reset_final_count'] = states[max(states)]['reset_count']
    if case == 'brake':
        result['recovery_1s_mps'] = planar(states[1800]['linear_velocity'])
        result['recovery_displacement_m'] = sum((states[1800]['position'][i]
                                               - states[1680]['position'][i]) ** 2
                                              for i in [0, 2]) ** .5
    return result


def trajectory_digest(physics, end):
    fields = ['position', 'rotation', 'linear_velocity', 'angular_velocity',
              'grounded', 'wheel_lengths', 'wheel_loads', 'speed', 'slip', 'reset_count']
    rows = [[tick, [physics[tick][f] for f in fields]] for tick in range(1, end + 1)
            if tick in physics]
    return hashlib.sha256(json.dumps(rows, sort_keys=True).encode()).hexdigest()


def analyze_one(task, study, car):
    run, ready, events, states, physics, inputs_sha, capture = evidence(task, study)
    spec = CASES[task['case']]
    metrics = measure(states, physics, task['case'], spec)
    metrics.update(reset_metrics(events, states, task['case']))
    course_end = 1680 if task['case'] == 'brake' else spec['end_tick'] + 1
    course = [physics[t] for t in range(240, course_end)]
    metrics['course_clearance_m'] = clearance(course, car)
    assert metrics['course_clearance_m'] > 0, 'body left obstacle-free perimeter strip'
    assert all(-436 < r['position'][0] < 436 for r in course), 'outside matched road'
    paths = ['manifest.json', 'scenario.json', 'scenario-summary.json',
             'recorder-health.json', 'events.jsonl', 'commands.jsonl',
             'input.jsonl', 'physics.jsonl', 'evidence-manifest.json']
    files = {name: sha(run / name) for name in paths}
    assets = {e['logical_path']: e['sha256'] for e in capture['inventory']
              if e['role'] in ['runtime_asset', 'embedded_catalog']}
    return {**task, 'ready_tick': ready, **metrics, 'matched_input_sha256': inputs_sha,
            'trajectory_sha256': trajectory_digest(physics, spec['end_tick']),
            'file_sha256': files, 'asset_sha256': assets}


def aggregate(rows):
    groups = {}
    metric_names = ['approach_mps', 'near_stop_s', 'near_stop_m', 'brake_1s_mps',
                    'brake_2s_mps', 'turn_deg', 'turn_exit_mps', 'turn_speed_pct',
                    'turn_lateral_m', 'four_contact_pct', 'min_contacts',
                    'airborne_ticks', 'four_loaded_pct', 'peak_tilt_deg', 'peak_slip_deg',
                    'requested_resets', 'unexpected_resets', 'reset_final_count',
                    'recovery_1s_mps', 'recovery_displacement_m', 'course_clearance_m']
    for row in rows:
        groups.setdefault((row['car'], row['case']), []).append(row)
    result = []
    for (car, case), items in sorted(groups.items()):
        item = {'car': car, 'case': case, 'n': len(items),
                'identical_trajectories': len({r['trajectory_sha256'] for r in items}) == 1}
        for name in metric_names:
            values = [r[name] for r in items if r.get(name) is not None]
            if values:
                item[name] = statistics.mean(values)
                item[name + '_range'] = [min(values), max(values)]
        result.append(item)
    return result


def save_csv(path, rows):
    fields = sorted({key for row in rows for key in row if not isinstance(row[key], (dict, list))})
    with path.open('w') as output:
        writer = csv.DictWriter(output, fields)
        writer.writeheader()
        writer.writerows({key: row.get(key) for key in fields} for row in rows)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('study', type=Path)
    args = parser.parse_args()
    study = load(args.study / 'study.json')
    tasks = lines(args.study / 'runs.jsonl')
    assert len(tasks) == len(study['tasks']), 'incomplete batch'
    keys = [(r['car'], r['case'], r['repeat']) for r in tasks]
    assert len(keys) == len(set(keys)), 'duplicate run assignment'
    assert set(keys) == {(r['car'], r['case'], r['repeat']) for r in study['tasks']}
    cars = {c['id']: c for c in load(ROOT / 'assets/cars/roster.json')}
    rows = [analyze_one(task, study, cars[task['car']]) for task in tasks]
    for case in CASES:
        assert len({r['matched_input_sha256'] for r in rows if r['case'] == case}) == 1
    assert all(r['asset_sha256'] == rows[0]['asset_sha256'] for r in rows)
    summary = {'schema': 1, 'study': str(args.study), 'runs': rows,
               'aggregate': aggregate(rows), 'complete_runs': len(rows),
               'identical_repeat_groups': sum(r['identical_trajectories'] for r in aggregate(rows))}
    (args.study / 'analysis.json').write_text(json.dumps(summary, indent=2) + '\n')
    save_csv(args.study / 'per-run.csv', rows)
    save_csv(args.study / 'comparison.csv', summary['aggregate'])
    print(json.dumps({k: summary[k] for k in ['complete_runs', 'identical_repeat_groups']}))


if __name__ == '__main__':
    main()
