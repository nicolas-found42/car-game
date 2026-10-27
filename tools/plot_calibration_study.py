#!/usr/bin/env python3
"""Plot audited cohort speeds. Optional dependency: matplotlib>=3.8,<4."""
import argparse
from pathlib import Path
from collections import defaultdict
from analyze_calibration_study import analyze


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('study', type=Path)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    report = analyze(args.study)
    import matplotlib
    matplotlib.use('Agg')
    import matplotlib.pyplot as plt
    groups = defaultdict(list)
    for row in report['results']:
        groups[(row['car'], row['case'])].append(row['metrics']['peak_mps'])
    labels = [f'{car}\n{case}' for car, case in groups]
    values = list(groups.values())
    means = [sum(v) / len(v) for v in values]
    errors = [[mean - min(v) for mean, v in zip(means, values)],
              [max(v) - mean for mean, v in zip(means, values)]]
    figure, axis = plt.subplots(figsize=(max(8, len(groups) * 1.6), 5), layout='constrained')
    axis.bar(labels, means, yerr=errors, capsize=4, color='#287c8e')
    axis.set(ylabel='Peak planar speed (m/s)', title='Reference-model simulation: repeated production scenarios\n'
                   'Bars: repeat mean; whiskers: min/max. Not original-game fidelity.')
    axis.tick_params(axis='x', labelsize=8)
    figure.savefig(args.output, dpi=150)
    plt.close(figure)


if __name__ == '__main__':
    main()
