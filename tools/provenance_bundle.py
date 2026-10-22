"""Frozen recorder inventory transport. This module never reads workspace assets."""
import hashlib
import json
import re
import shutil
from pathlib import Path, PurePosixPath

VERSION = 1
POLICY = 'grid-portable-allowlist-v1'


from provenance_schema import BundleError, SYSTEM_PREREQUISITES, validate_index


def sha256(path):
    digest = hashlib.sha256()
    with Path(path).open('rb') as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b''):
            digest.update(chunk)
    return digest.hexdigest()


def safe_path(root, logical):
    if not isinstance(logical, str):
        raise BundleError('unsafe_bundle_path')
    path = PurePosixPath(logical)
    if path.is_absolute() or '..' in path.parts or '\\' in logical:
        raise BundleError('unsafe_bundle_path')
    current = Path(root)
    if current.is_symlink():
        raise BundleError('unsafe_bundle_path')
    for part in path.parts:
        current = current / part
        if current.is_symlink():
            raise BundleError('unsafe_bundle_path')
    return current


def object_path(root, entry):
    logical = entry.get('object', '')
    if not re.fullmatch(r'objects/sha256/[0-9a-f]{64}', logical):
        raise BundleError('unsafe_bundle_path')
    safe_path(root, entry.get('logical_path'))
    return safe_path(root, logical)


def load_index(root):
    path = safe_path(root, 'evidence-manifest.json')
    if not path.is_file():
        raise BundleError('missing_evidence_index')
    try:
        index = json.loads(path.read_text())
    except (ValueError, UnicodeError):
        raise BundleError('invalid_evidence_index') from None
    if not isinstance(index, dict):
        raise BundleError('invalid_evidence_index')
    if type(index.get('schema')) is not int or type(index.get('provenance_version')) is not int or index['schema'] != 2 or index['provenance_version'] != VERSION:
        raise BundleError('unsupported_bundle_version')
    expected = safe_path(root, 'evidence-manifest.sha256')
    if not expected.is_file():
        raise BundleError('missing_index_digest')
    if expected.read_text().strip() != sha256(path):
        raise BundleError('index_hash_mismatch')
    validate_index(index)
    return index


def mismatch_code(role):
    return {'executable': 'binary_hash_mismatch', 'effective_config': 'config_hash_mismatch',
            'binary_build_snapshot': 'source_build_mismatch', 'build_metadata': 'source_build_mismatch'}.get(role, 'content_hash_mismatch')


def verify_entry(root, entry):
    path = object_path(root, entry)
    fields = {'role': entry.get('role'), 'logical_path': entry.get('logical_path')}
    if not path.is_file():
        raise BundleError('missing_required_object', **fields)
    actual = sha256(path)
    if actual != entry.get('sha256') or path.stat().st_size != entry.get('size_bytes'):
        raise BundleError(mismatch_code(entry.get('role')), **fields,
                          expected_sha256=entry.get('sha256'), actual_sha256=actual)


def verify(root, require_replay=False):
    index = load_index(root)
    errors = []
    for entry in index.get('inventory', []):
        if entry.get('availability') != 'included':
            continue
        try:
            verify_entry(root, entry)
        except BundleError as error:
            errors.append(error.detail)
    if not errors:
        from provenance_material import verify_material
        try:
            verify_material(root, index)
        except BundleError as error:
            errors.append(error.detail)
    if require_replay:
        errors.extend(replay_errors(index))
    return {'ok': not errors, 'errors': errors, 'run_id': index.get('run_id'),
            'inventory_entries': len(index.get('inventory', [])),
            'index_sha256': sha256(Path(root) / 'evidence-manifest.json'),
            'completeness': index.get('completeness', {}),
            'material_status': 'complete_runtime_material' if index['completeness']['runtime_material_complete'] else 'explicitly_incomplete_runtime_material',
            'behavioral_run_result': index.get('behavioral_run_result')}


def replay_errors(index):
    unavailable = [item for item in index.get('replay_prerequisites', [])
                   if (item.get('replay_required') or item.get('id') in SYSTEM_PREREQUISITES) and item.get('availability') != 'included']
    unavailable += [item for item in index.get('omissions', []) if item.get('replay_required')]
    if not index.get('completeness', {}).get('runtime_replay_complete'):
        unavailable.append({'id': 'exact_replay_not_established'})
    return [{'code': 'replay_prerequisite_unavailable',
             'prerequisite': item.get('id', item.get('role', 'unspecified'))}
            for item in unavailable]


def export(root, destination):
    result = verify(root)
    if not result['ok']:
        return result
    destination = Path(destination)
    if destination.exists():
        raise BundleError('export_destination_exists')
    index = load_index(root)
    destination.mkdir(parents=True)
    for name in ('evidence-manifest.json', 'evidence-manifest.sha256'):
        shutil.copyfile(safe_path(root, name), destination / name)
    for entry in index['inventory']:
        if entry.get('availability') == 'included':
            source = object_path(root, entry)
            target = object_path(destination, entry)
            target.parent.mkdir(parents=True, exist_ok=True)
            if not target.exists():
                shutil.copyfile(source, target)
    return verify(destination)


def captured_catalog(root):
    index = load_index(root)
    for entry in index['inventory']:
        if entry.get('role') == 'embedded_catalog':
            verify_entry(root, entry)
            return json.loads(object_path(root, entry).read_text())
    raise BundleError('replay_prerequisite_unavailable', prerequisite='captured_catalog')


def captured_entry(root, role=None, logical=None):
    for entry in load_index(root)['inventory']:
        if (role is None or entry['role'] == role) and (logical is None or entry['logical_path'] == logical):
            verify_entry(root, entry)
            return entry, object_path(root, entry)
    raise BundleError('missing_required_object', role=role, logical_path=logical)
