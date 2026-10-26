"""Plot paired production-fixture changes and the largest acceleration effects."""
import json
from pathlib import Path
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
import numpy as np

out = Path('docs/verification/fleet57-geometry-drivetrain')
r = json.loads((out/'comparison.json').read_text())
rows = {v['car']: v for v in r['benchmarks']}
fig, axes = plt.subplots(2, 2, figsize=(13, 8), constrained_layout=True)
colors = ['#9ba8b8', '#167b88']
for ax, metric, title, unit in [
    (axes[0, 0], 'zero_60_mph_s', 'Launch: 0–60 mph', 'seconds'),
    (axes[0, 1], 'sixty_to_hundred_mph_s', 'Midrange: 60–100 mph', 'seconds'),
    (axes[1, 0], 'brake_60_mph_0_m', 'Standardized stop: 60–0 mph', 'metres')]:
    x = np.arange(2)
    for j, state in enumerate(['before', 'after']):
        values = [rows[car]['metrics'][metric][state] for car in ['ref-a08', 'redline-gt500']]
        bars = ax.bar(x+(j-.5)*.32, values, .32, label=state.title(), color=colors[j])
        ax.bar_label(bars, fmt='%.3f', padding=3, fontsize=9)
    ax.set_xticks(x, ['C5 Z06', 'GT500']); ax.set_ylabel(unit); ax.set_title(title)
    ax.set_ylim(0, ax.get_ylim()[1]*1.15); ax.grid(axis='y', alpha=.2); ax.set_axisbelow(True)
axes[0, 0].legend(frameon=False)
names = {row['id']: row['name'] for row in json.load(open('assets/cars/roster.json'))}
ranked = sorted(r['benchmarks'], key=lambda v: abs(v['metrics']['zero_60_mph_s']['delta']), reverse=True)[:10]
ax = axes[1, 1]
ax.barh([names[v['car']] for v in reversed(ranked)], [v['metrics']['zero_60_mph_s']['delta'] for v in reversed(ranked)], color='#167b88')
ax.axvline(0, color='#555', linewidth=.8); ax.set_xlabel('after minus before, seconds')
ax.set_title('Largest changes in 0–60 mph'); ax.grid(axis='x', alpha=.2); ax.set_axisbelow(True)
fig.suptitle('C5 geometry and driven-wheel gearing correction', fontsize=17)
fig.savefig(out/'comparison.png', dpi=170)
plt.close(fig)
