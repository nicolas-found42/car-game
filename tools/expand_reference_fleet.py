#!/usr/bin/env python3
"""Reproduce the declared reference expansion while preserving the prior editions."""
import hashlib
import json
from pathlib import Path

from prepare_fleet import ROOT, prepare
from reference_geometry import apply_reference

PLAN = ROOT / 'docs/verification/fleet-57'


def read(path):
    return json.loads(path.read_text())


def expand(source):
    original = read(PLAN / 'original-roster.json')
    physics = read(PLAN / 'original-physics.json')
    pinned = read(PLAN / 'preparation.json')['existing_material_sha256']
    assert all(hashlib.sha256((ROOT / p).read_bytes()).hexdigest() == h
               for p, h in pinned.items())
    selection = read(PLAN / 'selection.json')
    catalog = {c['code']: c for c in read(source / 'catalog.json')}
    additions = [prepare(source, catalog, c['id'], c['source_code'], c['name'])
                 for c in selection['cars']]
    roster = original + [apply_reference(car) for car in additions]
    physics['cars'] += read(PLAN / 'profile-expansion.json')['cars']
    references = {c['id'] for c in read(ROOT / 'docs/vehicle-reference/real-profiles.json')['cars']}
    assert references <= {c['corpus_reference'] for c in physics['cars']}
    assert len(roster) == selection['target_fleet_size'] == len({c['id'] for c in roster})
    for name, value in [('roster', roster), ('physics', physics)]:
        (ROOT / f'assets/cars/{name}.json').write_text(json.dumps(value, indent=2) + '\n')


if __name__ == '__main__':
    expand(ROOT.parent / 'carmodels/dealership/dealership')
