#!/usr/bin/env python3
"""Prepare the explicit expansion, preserving all existing car editions byte-for-byte."""
import argparse
import hashlib
import json
from pathlib import Path

from prepare_fleet import ROOT, prepare


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def existing_materials(roster):
    paths = [ROOT / 'assets' / car['path'] for car in roster]
    return {str(p): digest(p) for path in paths for p in [path, path.with_suffix('.json')]}


def expand(source):
    plan = json.loads((ROOT / 'docs/verification/fleet-40/selection.json').read_text())
    roster_path = ROOT / 'assets/cars/roster.json'
    original = json.loads((ROOT / 'docs/verification/fleet-40/original-roster.json').read_text())
    pinned = existing_materials(original)
    root = source / 'dealership/dealership'
    catalog = {c['code']: c for c in json.loads((root / 'catalog.json').read_text())}
    additions = [prepare(root, catalog, c['id'], c['source_code'], c['name']) for c in plan['cars']]
    roster = original + additions
    assert len(roster) == plan['target_fleet_size'] == len({c['id'] for c in roster})
    assert pinned == existing_materials(original), 'Existing edition was altered'
    roster_path.write_text(json.dumps(roster, indent=2) + '\n')
    evidence = {'existing_material_sha256': pinned, 'added_ids': [c['id'] for c in additions],
                'roster_sha256': digest(roster_path), 'edition_count': len(roster)}
    out = ROOT / 'docs/verification/fleet-40/preparation.json'
    out.write_text(json.dumps(evidence, indent=2) + '\n')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source-root', type=Path, default=ROOT.parent / 'carmodels')
    expand(parser.parse_args().source_root)


if __name__ == '__main__':
    main()
