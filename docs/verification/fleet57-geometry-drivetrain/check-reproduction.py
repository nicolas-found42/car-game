"""Rebuild the corrected C5 from its archive in isolation, then check idempotence."""
import hashlib
import json
from pathlib import Path
import sys
import tempfile
sys.path.insert(0, 'tools')
import prepare_fleet
from reference_geometry import apply_reference

root = Path.cwd()
actual = next(c for c in json.load(open('assets/cars/roster.json')) if c['id'] == 'ref-a08')
source = root.parent/'carmodels/dealership/dealership'
catalog = {r['code']: r for r in json.load(open(source/'catalog.json'))}
with tempfile.TemporaryDirectory() as temporary:
    parent = Path(temporary)
    prepare_fleet.ROOT = parent/'grid'
    (prepare_fleet.ROOT/'assets/cars').mkdir(parents=True)
    (parent/'carmodels').symlink_to(root.parent/'carmodels', target_is_directory=True)
    car = prepare_fleet.prepare(source, catalog, actual['id'], actual['source_code'], actual['name'])
    car = apply_reference(car)
    assert car == actual
    hashes = {}
    for suffix in ['.glb', '.json']:
        relative = Path('assets/cars/ref-a08'+suffix)
        rebuilt = (prepare_fleet.ROOT/relative).read_bytes()
        original = (root/relative).read_bytes()
        if suffix == '.glb':
            assert rebuilt == original, str(relative)
        else:
            assert json.loads(rebuilt) == json.loads(original), str(relative)
        hashes[str(relative)] = hashlib.sha256(rebuilt).hexdigest()
    assert apply_reference(car) == actual
    for relative, digest in hashes.items():
        assert hashlib.sha256((prepare_fleet.ROOT/relative).read_bytes()).hexdigest() == digest
out = root/'docs/verification/fleet57-geometry-drivetrain/reproduction.json'
out.write_text(json.dumps({'source_to_glb_byte_exact': True, 'receipt_semantically_equal': True, 'receipt_serialization_note': 'JSON key order differs because the initial correction inserted the receipt field before its final hash.', 'idempotent': True, 'files': hashes}, indent=2)+'\n')
print('C5 archive rebuild: GLB byte-exact, receipt semantically equal; correction idempotent')
