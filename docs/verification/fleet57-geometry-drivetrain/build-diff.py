"""Compare captured compiled inputs, rather than the much larger dirty Git diff."""
import difflib
import hashlib
import json
from pathlib import Path
out = Path('docs/verification/fleet57-geometry-drivetrain')
before = Path('artifacts/vehicle-reference/fleet-57/car-game-budget.recorded.provenance')
after = Path('artifacts/vehicle-reference/fleet57-geometry-drivetrain/car-game.recorded.provenance')
a = json.load(open(before/'build.json')); b = json.load(open(after/'build.json'))
old = {r['logical_path']: r for r in a['inventory'] if r['logical_path'].startswith('build_source/')}
new = {r['logical_path']: r for r in b['inventory'] if r['logical_path'].startswith('build_source/')}
assert old.keys() == new.keys()
changed = []; metadata = []; patch = []
for name, row in new.items():
    prior = old[name]
    for root, entry in [(before, prior), (after, row)]:
        assert hashlib.sha256((root/entry['object']).read_bytes()).hexdigest() == entry['sha256']
    if prior['sha256'] != row['sha256']:
        if name == 'build_source/dirty.patch':
            metadata.append({'path': name, 'before_sha256': prior['sha256'], 'after_sha256': row['sha256']})
            continue
        changed.append(name)
        x = (before/prior['object']).read_text().splitlines(keepends=True)
        y = (after/row['object']).read_text().splitlines(keepends=True)
        patch.extend(difflib.unified_diff(x, y, fromfile='before/'+name, tofile='after/'+name))
assert set(changed) == {'build_source/assets/cars/roster.json', 'build_source/src/vehicle.rs', 'build_source/src/vehicle/profile.rs'}, changed
(out/'compiled-source-diff.patch').write_text(''.join(patch))
(out/'compiled-source-diff.json').write_text(json.dumps({'baseline_budget_binary_sha256': a['binary_sha256'],
    'after_binary_sha256': b['binary_sha256'], 'changed_compiled_inputs': changed,
    'changed_snapshot_metadata': metadata,
    'same_profile_json': old['build_source/assets/cars/physics.json']['sha256'] == new['build_source/assets/cars/physics.json']['sha256'],
    'baseline_matrix_bridge': 'docs/verification/fleet-57/recorder-build-bridge.json'}, indent=2)+'\n')
print('Changed compiled inputs:', changed)
