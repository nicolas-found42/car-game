#!/usr/bin/env python3
"""Validate the staged prepared catalogue without enabling new runtime fleet IDs."""
import json
from pathlib import Path
import check_assets
import tempfile


def check():
    root = Path(__file__).resolve().parents[1]
    catalog = json.loads((root / 'staged-fleet/catalog.json').read_text())
    active = json.loads((root / 'assets/cars/roster.json').read_text())
    ids = [car['id'] for car in catalog]
    assert len(ids) == len(set(ids)) == 287
    assert not set(ids) & {car['id'] for car in active}
    with tempfile.TemporaryDirectory() as directory:
        temporary = Path(directory)
        grid = temporary / 'grid'
        grid.mkdir()
        (grid / 'assets').symlink_to(root / 'staged-fleet/assets', target_is_directory=True)
        (temporary / 'carmodels').symlink_to(root.parent / 'carmodels', target_is_directory=True)
        original = check_assets.ROOT
        try:
            check_assets.ROOT = grid
            for car in catalog:
                check_assets.check_car(car)
        finally:
            check_assets.ROOT = original
    print(json.dumps({'status': 'passed', 'prepared_assets': len(catalog), 'runtime_cars': len(active),
                      'scope': 'source geometry, image bytes, authored transforms and receipt identity'}))


if __name__ == '__main__':
    check()
