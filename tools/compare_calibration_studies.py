#!/usr/bin/env python3
"""Compare audited matched cohorts; reject changed stimuli instead of hiding them."""
import argparse
from pathlib import Path
from analyze_calibration_study import analyze
from calibration_io import identity, load, local, require, write


def compare(left_path, right_path):
    left, right = analyze(left_path), analyze(right_path)
    a, b = left['plan'], right['plan']
    require(a['cohort_sha256'] == b['cohort_sha256'] and a['policy'] == b['policy'], 'unmatched cohort or policy')
    stable = set(a['bindings']) - {'assets/cars/physics.json'}
    require(stable == set(b['bindings']) - {'assets/cars/physics.json'}, 'unmatched source inventory')
    require(all(a['bindings'][k]['sha256'] == b['bindings'][k]['sha256'] for k in stable),
            'unmatched roster, protocol, driver or course')
    require(a['assets'] == b['assets'], 'unmatched prepared player assets')
    tasks_b = {identity(t): t for t in b['tasks']}
    for task in a['tasks']:
        other = tasks_b[identity(task)]
        require(load(local(left_path, task['scenario'])) ==
                load(local(right_path, other['scenario'])), 'unmatched scenario stimulus')
    rows_b = {identity(r): r for r in right['results']}
    deltas = []
    for row in left['results']:
        other = rows_b[identity(row)]
        task = next(t for t in a['tasks'] if identity(t) == identity(row))
        closed_loop = 'route' in load(local(left_path, task['scenario']))
        require(closed_loop or row['input_sha256'] == other['input_sha256'], 'unmatched effective open-loop inputs')
        deltas.append({**{k: row[k] for k in ['car', 'case', 'repeat']},
                       'closed_loop': closed_loop,
                       'delta': {k: (other['metrics'][k] - value if value is not None and other['metrics'][k] is not None else None)
                                 for k, value in row['metrics'].items()}})
    return {'schema': 1, 'matched': True, 'deltas': deltas,
            'binary_sha256': [a['binary_sha256'], b['binary_sha256']],
            'profile_sha256': [p['bindings']['assets/cars/physics.json']['sha256'] for p in [a, b]],
            'qualification': 'Repeated deterministic simulations measure repeatability, not real-world variance. '
            'Binary/profile changes are declared factors; differences do not establish causation.'}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('left', type=Path)
    parser.add_argument('right', type=Path)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    try:
        result = compare(args.left, args.right)
        write(args.output, result)
        print(f"Matched {len(result['deltas'])} run pairs")
    except (ValueError, OSError, KeyError) as error:
        parser.exit(1, f'Unmatched studies: {error}\n')


if __name__ == '__main__':
    main()
