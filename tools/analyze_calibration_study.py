#!/usr/bin/env python3
"""Validate a complete declared cohort, then write descriptive JSON/CSV metrics."""
import argparse
import csv
import itertools
from pathlib import Path
from calibration_io import digest, identity, load, local, require, sha, write
from calibration_evidence import audit
from handling_metrics import measure, planar, support, yaw_change
from handling_protocol import CASES


def check_matrix(study, plan, tasks):
    expected = set(itertools.product(plan['cars'], plan['cases'], range(plan['repeats'])))
    require(len(tasks) == len(expected) and {identity(t) for t in tasks} == expected,
            'missing or duplicate cohort task')
    require(plan['cohort_sha256'] == digest([plan['cars'], plan['cases'], plan['repeats']]),
            'cohort declaration changed')
    planned = {identity(t): t for t in plan['tasks']}
    require(len(planned) == len(expected) and set(planned) == expected, 'invalid planned matrix')
    for task in tasks:
        require(all(task[k] == v for k, v in planned[identity(task)].items()), 'task differs from plan')
        require(sha(local(study, task['scenario'])) == task['scenario_sha256'], 'scenario input changed')
    for item in plan['bindings'].values():
        require(sha(local(study, item['path'])) == item['sha256'], 'frozen study input changed')


def metrics(samples):
    window = samples[239:]
    require(bool(window), 'empty measurement window')
    distance = sum(((b['position'][0] - a['position'][0]) ** 2 +
                    (b['position'][2] - a['position'][2]) ** 2) ** .5
                   for a, b in zip(window, window[1:]) if a['reset_count'] == b['reset_count'])
    return {**support(window), 'path_m': distance, 'peak_mps': max(planar(s['linear_velocity']) for s in window),
            'final_mps': planar(window[-1]['linear_velocity']), 'yaw_change_deg': yaw_change(window)}


def measure_case(samples, case):
    observed = metrics(samples)
    if case in CASES:
        relative = dict(enumerate(samples, 1))
        observed.update({'action_' + key: value for key, value in
                         measure(relative, relative, case, CASES[case]).items()})
    return observed


def analyze(study):
    study = Path(study).resolve()
    plan, tasks = load(study / 'study.json'), load(study / 'runs.json')
    check_matrix(study, plan, tasks)
    results = []
    for task in tasks:
        samples, inputs, completeness = audit(study, plan, task)
        observed = measure_case(samples, task['case'])
        results.append({**{k: task[k] for k in ['car', 'case', 'repeat', 'scenario_sha256']},
                        'input_sha256': inputs, 'metrics': observed, 'completeness': completeness})
    report = {'schema': 1, 'plan': plan, 'results': results,
              'qualification': 'Observed reference-model simulation; not original-game fidelity or exact replay. '
              'Metrics cover relative tick 240 through final assertion, including stopped tails. '
              'Closed-loop routes match geometry and driver, not identical applied inputs.'}
    write(study / 'analysis.json', report)
    with (study / 'metrics.csv').open('w', newline='') as stream:
        flat = [{**{k: r[k] for k in ['car', 'case', 'repeat']}, **r['metrics']} for r in results]
        writer = csv.DictWriter(stream, fieldnames=sorted(set().union(*(row.keys() for row in flat))))
        writer.writeheader()
        writer.writerows(flat)
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('study', type=Path)
    args = parser.parse_args()
    try:
        result = analyze(args.study)
        print(f"Validated {len(result['results'])} production runs")
    except (ValueError, OSError, KeyError) as error:
        parser.exit(1, f'Invalid study: {error}\n')


if __name__ == '__main__':
    main()
