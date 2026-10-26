"""Retain every contact exception and its matched baseline, without filtering failures."""
import json
from pathlib import Path
out = Path('docs/verification/fleet57-reference-calibration')
a = json.load(open('docs/verification/fleet57-geometry-drivetrain/results.json'))
b = json.load(open(out/'results.json'))
old = {(r['car'], r['case'], r['repeat']): r for r in a['runs']}
new = {(r['car'], r['case'], r['repeat']): r for r in b['runs']}
keys = {k for mapping in [old, new] for k, r in mapping.items() if r['four_contact_pct'] < 100}
fields = ['four_contact_pct', 'four_loaded_pct', 'minimum_contacts', 'airborne_ticks', 'peak_tilt_deg', 'run']
rows = [{'car': k[0], 'case': k[1], 'repeat': k[2],
         'before': {f: old[k][f] for f in fields}, 'after': {f: new[k][f] for f in fields},
         'identical_trajectory': old[k]['trajectory_sha256'] == new[k]['trajectory_sha256']}
        for k in sorted(keys)]
(out/'contact-comparison.json').write_text(json.dumps({'exceptions': rows,
    'baseline_investigation': 'docs/verification/fleet-57/contact-investigation.json',
    'interpretation': 'Baseline 22B right trail-brake contact loss was reproduced by wall-only controls. Changed approaches require trace inspection before assigning the same cause.'}, indent=2)+'\n')
