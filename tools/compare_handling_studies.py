#!/usr/bin/env python3
"""Compare complete matched handling studies; reject changed assets or input cohorts."""
import argparse
import csv
import json
from pathlib import Path

METRICS = ['near_stop_m', 'near_stop_s', 'approach_mps', 'turn_deg', 'turn_exit_mps',
           'peak_tilt_deg', 'peak_slip_deg', 'four_contact_pct', 'four_loaded_pct',
           'unexpected_resets', 'recovery_1s_mps']


def load(path):
    return json.loads(path.read_text())


def check_matching(before, after, old_config, new_config):
    for key in ['cars', 'cases', 'backend', 'fixed_hz', 'seed', 'roster_sha256', 'repeats']:
        assert old_config[key] == new_config[key], f'changed experiment setting: {key}'
    key = lambda row: (row['car'], row['case'], row['repeat'])
    old_runs = {key(row): row for row in before['runs']}
    new_runs = {key(row): row for row in after['runs']}
    assert len(old_runs) == len(new_runs) == 130
    assert old_runs.keys() == new_runs.keys()
    for identity, old in old_runs.items():
        new = new_runs[identity]
        for field in ['matched_input_sha256', 'asset_sha256', 'scenario_sha256']:
            assert old[field] == new[field], (identity, field)
    assert old_config['binary_sha256'] != new_config['binary_sha256'], 'identical binaries'


def compare(before, after):
    old = {(row['car'], row['case']): row for row in before['aggregate']}
    new = {(row['car'], row['case']): row for row in after['aggregate']}
    assert len(old) == len(new) == 65 and old.keys() == new.keys()
    rows = []
    for identity, first in sorted(old.items()):
        second = new[identity]
        row = {'car': identity[0], 'case': identity[1], 'repeats_per_version': 2}
        assert first['n'] == second['n'] == 2
        for metric in METRICS:
            if metric in first and metric in second:
                row['before_' + metric] = first[metric]
                row['after_' + metric] = second[metric]
                row['delta_' + metric] = second[metric] - first[metric]
        rows.append(row)
    return rows


def save(output, result):
    output.mkdir(parents=True, exist_ok=True)
    (output / 'comparison.json').write_text(json.dumps(result, indent=2) + '\n')
    fields = sorted({key for row in result['rows'] for key in row})
    with (output / 'comparison.csv').open('w') as stream:
        writer = csv.DictWriter(stream, fields)
        writer.writeheader()
        writer.writerows(result['rows'])


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--before', type=Path, required=True)
    parser.add_argument('--after', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    before, after = [load(path / 'analysis.json') for path in [args.before, args.after]]
    old_config, new_config = [load(path / 'study.json') for path in [args.before, args.after]]
    check_matching(before, after, old_config, new_config)
    result = {'schema': 1, 'before': str(args.before), 'after': str(args.after),
              'before_binary_sha256': old_config['binary_sha256'],
              'after_binary_sha256': new_config['binary_sha256'],
              'matched_runs_per_version': 130, 'rows': compare(before, after),
              'identical_assets_scenarios_and_effective_inputs': True}
    save(args.output, result)
    print(f'Compared {len(result["rows"])} car/case pairs, two repeats per version.')


if __name__ == '__main__':
    main()
