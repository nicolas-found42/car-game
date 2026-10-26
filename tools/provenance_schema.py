"""Structural contracts for the recorder's versioned evidence index."""
import re
from pathlib import PurePosixPath

SINGLETONS = {'executable', 'effective_config', 'embedded_catalog', 'embedded_physics_profile', 'capture_index', 'build_metadata'}
FLAGS = {'inventory_integrity', 'runtime_material_complete', 'runtime_replay_complete', 'rebuild_complete'}
DIGEST = re.compile(r'[0-9a-f]{64}')
SYSTEM_PREREQUISITES = {'compatible_os_arch_dynamic_libraries', 'gpu_renderer_audio_drivers'}


class BundleError(Exception):
    def __init__(self, code, **fields):
        self.detail = {'code': code, **fields}
        super().__init__(code)


def logical_path(value):
    return (isinstance(value, str) and bool(value) and '\\' not in value
            and not PurePosixPath(value).is_absolute() and '..' not in PurePosixPath(value).parts
            and PurePosixPath(value).as_posix() == value and value != '.')


def validate_entry(entry):
    if not isinstance(entry, dict) or not re.fullmatch(r'[a-z][a-z0-9_]*', str(entry.get('role', ''))):
        raise BundleError('invalid_inventory_entry')
    if not logical_path(entry.get('logical_path')):
        raise BundleError('unsafe_bundle_path')
    if entry.get('availability') == 'unavailable':
        if not isinstance(entry.get('reason'), str) or not entry['reason'] or type(entry.get('replay_required')) is not bool:
            raise BundleError('invalid_inventory_entry')
        return
    if entry.get('availability') != 'included':
        raise BundleError('invalid_inventory_entry')
    digest, size = entry.get('sha256'), entry.get('size_bytes')
    if not isinstance(digest, str) or not DIGEST.fullmatch(digest) or type(size) is not int or size < 0:
        raise BundleError('invalid_inventory_entry')
    if entry.get('object') != f'objects/sha256/{digest}':
        if not logical_path(entry.get('object')):
            raise BundleError('unsafe_bundle_path')
        raise BundleError('object_binding_mismatch', role=entry['role'], logical_path=entry['logical_path'])


def validate_inventory(entries):
    if not isinstance(entries, list):
        raise BundleError('invalid_bundle_schema', field='inventory')
    paths, singletons = set(), set()
    for entry in entries:
        validate_entry(entry)
        role, logical = entry['role'], entry['logical_path']
        if logical in paths or (role in SINGLETONS and role in singletons):
            raise BundleError('duplicate_inventory_binding', role=role, logical_path=logical)
        paths.add(logical)
        singletons.add(role)


def validate_index(index):
    if not isinstance(index.get('run_id'), str) or not index['run_id'] or index.get('policy') != 'grid-portable-allowlist-v1':
        raise BundleError('invalid_bundle_schema')
    validate_inventory(index.get('inventory'))
    validate_statuses(index)
    validate_explanations(index)
    validate_claim_roles(index)
    validate_completeness(index)
    validate_prerequisite_bindings(index)


def validate_statuses(index):
    flags = index.get('completeness')
    if not isinstance(flags, dict) or set(flags) != FLAGS or any(type(value) is not bool for value in flags.values()):
        raise BundleError('invalid_completeness_claim', reason='explicit_boolean_statuses_required')
    for key in ('omissions', 'replay_prerequisites'):
        if not isinstance(index.get(key), list) or any(not isinstance(item, dict) for item in index[key]):
            raise BundleError('invalid_bundle_schema', field=key)


def validate_claim_roles(index):
    flags = index['completeness']
    roles = {entry['role'] for entry in index['inventory'] if entry['availability'] == 'included'}
    required = set()
    if flags['runtime_material_complete'] or flags['runtime_replay_complete']:
        required |= {'executable', 'effective_config', 'embedded_catalog', 'capture_index'}
    if flags['rebuild_complete']:
        required |= {'executable', 'build_metadata', 'binary_build_snapshot', 'capture_index'}
    if not required <= roles:
        raise BundleError('invalid_completeness_claim', reason='required_material_missing', missing_roles=sorted(required - roles))


def validate_completeness(index):
    flags = index['completeness']
    missing = [item for item in index['omissions'] if item.get('replay_required')]
    external = [item for item in index['replay_prerequisites'] if item.get('replay_required') and item.get('availability') != 'included']
    if flags['runtime_material_complete'] and missing:
        raise BundleError('invalid_completeness_claim', reason='required_runtime_inputs_omitted')
    if flags['runtime_replay_complete'] and (not flags['runtime_material_complete'] or external):
        raise BundleError('invalid_completeness_claim', reason='replay_prerequisites_unavailable')
    if not flags['runtime_material_complete'] and not index['omissions']:
        raise BundleError('invalid_completeness_claim', reason='incomplete_inventory_requires_explanation')


def validate_explanations(index):
    for item in index['omissions']:
        if not isinstance(item.get('role'), str) or not isinstance(item.get('reason'), str) or not item['reason'] or type(item.get('replay_required')) is not bool:
            raise BundleError('invalid_bundle_schema', field='omissions')
    for item in index['replay_prerequisites']:
        if not isinstance(item.get('id'), str) or not item['id'] or type(item.get('replay_required')) is not bool or item.get('availability') not in {'included', 'external_required', 'unavailable'}:
            raise BundleError('invalid_bundle_schema', field='replay_prerequisites')
    if index['completeness']['runtime_material_complete'] and any(
            entry['availability'] == 'unavailable' and entry.get('replay_required') for entry in index['inventory']):
        raise BundleError('invalid_completeness_claim', reason='required_inventory_entry_unavailable')


def validate_prerequisite_bindings(index):
    prerequisites = index['replay_prerequisites']
    ids = [item['id'] for item in prerequisites]
    if len(set(ids)) != len(ids):
        raise BundleError('duplicate_prerequisite_binding')
    if index['completeness']['runtime_material_complete']:
        if not SYSTEM_PREREQUISITES <= set(ids):
            raise BundleError('invalid_completeness_claim', reason='system_prerequisites_undeclared')
    validate_system_prerequisites(prerequisites)
    entries = {entry['logical_path']: entry for entry in index['inventory'] if entry['availability'] == 'included'}
    for prerequisite in prerequisites:
        if prerequisite['availability'] == 'included' and prerequisite.get('replay_required'):
            entry = entries.get(prerequisite.get('logical_path'))
            if entry is None or prerequisite.get('sha256') != entry['sha256']:
                raise BundleError('invalid_completeness_claim', reason='prerequisite_content_binding_missing')


def validate_system_prerequisites(prerequisites):
    for prerequisite in prerequisites:
        if prerequisite['id'] in SYSTEM_PREREQUISITES:
            if prerequisite['replay_required'] is not True or prerequisite['availability'] != 'external_required':
                raise BundleError('invalid_completeness_claim', reason='system_prerequisites_must_remain_required_and_external')
