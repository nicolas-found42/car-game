"""Explicitly incomplete legacy inventory; no reconstruction from present assets."""
import json
from pathlib import Path

from provenance_bundle import POLICY, sha256

PRIVATE_KEYS = {'args', 'env', 'cwd', 'source_root', 'assets_root', 'dir', 'log_tail',
                'backtrace', 'message', 'location', 'rustc', 'cargo', 'branch'}


def redact(value):
    if isinstance(value, dict):
        return {key: 'redacted_by_policy' if key in PRIVATE_KEYS else redact(item)
                for key, item in value.items()}
    if isinstance(value, list):
        return [redact(item) for item in value]
    if isinstance(value, str) and (value.startswith('/') or '/Users/' in value or
                                   '/home/' in value or ':\\' in value):
        return 'redacted_by_policy'
    return value


def legacy_manifest(run, destination=None):
    run = Path(run)
    destination = Path(destination) if destination is not None else run
    metadata = json.loads((run / 'manifest.json').read_text())
    report = run / 'report.json'
    inventory = []
    for name in ('manifest.json', 'report.json'):
        path = run / name
        if path.is_file():
            data = json.dumps(redact(json.loads(path.read_text())), indent=2).encode()
            inventory.append(store_bytes(destination, data, 'legacy_derived_metadata', name))
    result = {'schema': 2, 'provenance_version': 1, 'policy': POLICY,
              'run_id': metadata['run_id'], 'inventory': inventory, 'cars': [],
              'report_sha256': sha256(report) if report.is_file() else None,
              'completeness': {'inventory_integrity': True, 'runtime_replay_complete': False,
                               'runtime_material_complete': False, 'rebuild_complete': False},
              'omissions': [{'role': 'capture_time_provenance', 'replay_required': True,
                            'reason': 'legacy_run_no_historical_asset_or_build_claim'}],
              'replay_prerequisites': [],
              'behavioral_run_result': {'exit': metadata.get('exit')}}
    write_index(destination, result)
    return result


def store_bytes(run, data, role, logical):
    import hashlib
    digest = hashlib.sha256(data).hexdigest()
    target = Path(run) / 'objects/sha256' / digest
    target.parent.mkdir(parents=True, exist_ok=True)
    try:
        with target.open('xb') as stream:
            stream.write(data)
    except FileExistsError:
        if sha256(target) != digest:
            raise ValueError('existing_object_hash_mismatch') from None
    return {'role': role, 'logical_path': logical, 'availability': 'included',
            'sha256': digest, 'size_bytes': len(data), 'object': f'objects/sha256/{digest}'}


def write_index(run, index):
    path = Path(run) / 'evidence-manifest.json'
    with path.open('x') as stream:
        stream.write(json.dumps(index, indent=2) + '\n')
    path.with_suffix('.sha256').write_text(sha256(path) + '\n')


def export_legacy(run, destination):
    import tempfile
    from provenance_bundle import export
    with tempfile.TemporaryDirectory(prefix='grid-legacy-export-') as temporary:
        legacy_manifest(run, temporary)
        return export(temporary, destination)
