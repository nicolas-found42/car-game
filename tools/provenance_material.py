"""Verify capture/build/catalog bindings using frozen bundle objects only."""
import json
from pathlib import PurePosixPath

from provenance_binding import FORMAT, derived_rebuild_complete, recompute
from provenance_bundle import BundleError, object_path
from provenance_schema import DIGEST, logical_path, validate_inventory

OUTPUT_ROLES = {'run_metadata', 'run_trace', 'capture', 'capture_index', 'legacy_derived_metadata'}


def entry_for(index, role):
    return next((entry for entry in index['inventory']
                 if entry['role'] == role and entry['availability'] == 'included'), None)


def load_json(root, entry):
    try:
        return json.loads(object_path(root, entry).read_text())
    except (ValueError, UnicodeError):
        raise BundleError('invalid_material_schema', role=entry['role']) from None


def verify_material(root, index):
    capture_entry = entry_for(index, 'capture_index')
    capture = load_json(root, capture_entry) if capture_entry else None
    if index['completeness']['runtime_material_complete']:
        verify_runtime(root, index)
    if capture_entry is not None:
        verify_capture(index, capture_entry, capture)
    if entry_for(index, 'build_metadata') or index['completeness']['rebuild_complete']:
        verify_build(root, index, capture)


def verify_capture(index, entry, capture):
    if not isinstance(capture, dict) or capture.get('schema') != 'car-game.provenance' or (type(capture.get('version')) is not int or capture['version'] != 1):
        raise BundleError('invalid_material_schema', role='capture_index')
    if index.get('capture_sha256') != entry['sha256'] or any(
            capture.get(key) != index.get(key) for key in
            ('run_id', 'policy', 'completeness', 'omissions', 'replay_prerequisites')):
        raise BundleError('capture_binding_mismatch')
    validate_inventory(capture.get('inventory'))
    captured = {entry['logical_path']: entry for entry in capture['inventory']}
    actual = {entry['logical_path']: entry for entry in index['inventory']
              if entry['role'] not in OUTPUT_ROLES}
    if captured != actual:
        raise BundleError('capture_inventory_mismatch')


def required_runtime(catalog):
    if not isinstance(catalog, list) or not catalog:
        raise BundleError('invalid_material_schema', role='embedded_catalog')
    required = {'assets/cars/roster.json': 'receipt_or_catalog'}
    ids, paths = set(), set()
    for car in catalog:
        if not isinstance(car, dict) or not isinstance(car.get('id'), str) or not logical_path(car.get('path')):
            raise BundleError('invalid_material_schema', role='embedded_catalog')
        if car['id'] in ids or car['path'] in paths:
            raise BundleError('duplicate_catalog_binding')
        ids.add(car['id'])
        paths.add(car['path'])
        asset = 'assets/' + car['path']
        required[asset] = 'prepared_asset'
        required[str(PurePosixPath(asset).with_suffix('.json'))] = 'receipt_or_catalog'
    required.update({f'assets/audio/{name}.wav': 'runtime_asset' for name in ('engine', 'tires', 'impact')})
    required.update({f'assets/textures/facade-{index}.png': 'runtime_asset' for index in range(4)})
    return required


def verify_runtime(root, index):
    catalog_entry = entry_for(index, 'embedded_catalog')
    required = required_runtime(load_json(root, catalog_entry))
    actual = {entry['logical_path']: entry for entry in index['inventory'] if entry['availability'] == 'included'}
    missing = [logical for logical, role in required.items()
               if logical not in actual or actual[logical]['role'] != role]
    if missing:
        raise BundleError('invalid_completeness_claim', reason='required_runtime_content_missing', logical_paths=missing)
    if actual['assets/cars/roster.json']['sha256'] != catalog_entry['sha256']:
        raise BundleError('invalid_completeness_claim', reason='embedded_runtime_catalog_mismatch')
    physics = entry_for(index, 'embedded_physics_profile')
    if physics is not None:
        runtime = actual.get('assets/cars/physics.json')
        if not runtime or runtime['sha256'] != physics['sha256']:
            raise BundleError('invalid_completeness_claim', reason='embedded_runtime_physics_mismatch')
    verify_receipts(root, required, actual)
    config = load_json(root, entry_for(index, 'effective_config'))
    if not isinstance(config, dict) or not isinstance(config.get('settings'), dict) or not isinstance(config.get('arguments'), list) or not isinstance(config.get('environment'), dict):
        raise BundleError('invalid_material_schema', role='effective_config')
    verify_input_bindings(config, actual)


def verify_receipts(root, required, actual):
    for logical, role in required.items():
        if role == 'prepared_asset':
            receipt = actual[str(PurePosixPath(logical).with_suffix('.json'))]
            material = load_json(root, receipt)
            if not isinstance(material, dict) or material.get('prepared_sha256') != actual[logical]['sha256']:
                raise BundleError('receipt_binding_mismatch', logical_path=logical)


def verify_input_bindings(config, actual):
    for flag, role in (('--scenario', 'scenario'), ('--replay-input', 'replay_input')):
        if flag in config['arguments']:
            logical = 'inputs/' + role
            if logical not in actual or actual[logical]['role'] != role:
                raise BundleError('invalid_completeness_claim', reason='required_input_missing', role=role)


def verify_build(root, index, capture):
    metadata, asserted = validated_build(root, index)
    if not isinstance(capture, dict) or capture.get('compiled_source_fingerprint') != asserted:
        raise BundleError('source_build_mismatch', reason='compiled_binding_mismatch')
    if capture.get('binary_build') != metadata:
        raise BundleError('source_build_mismatch', reason='capture_build_metadata_mismatch')
    executable = entry_for(index, 'executable')
    if metadata.get('binary_sha256') != executable['sha256']:
        raise BundleError('source_build_mismatch', reason='executable_binding_mismatch')
    verify_build_sources(index, metadata)
    if not contains(object_path(root, executable), asserted.encode()):
        raise BundleError('source_build_mismatch', reason='compiler_binding_absent_from_executable')


def validated_build(root, index):
    metadata = load_json(root, entry_for(index, 'build_metadata'))
    if not isinstance(metadata, dict) or type(metadata.get('schema')) is not int or metadata['schema'] != 1:
        raise BundleError('invalid_material_schema', role='build_metadata')
    if metadata.get('binding_format') != FORMAT:
        raise BundleError('unsupported_build_binding_format', role='build_metadata')
    try:
        fingerprint = recompute(metadata)
        complete = derived_rebuild_complete(metadata)
    except (KeyError, ValueError, TypeError):
        raise BundleError('source_build_mismatch', reason='binding_inputs_unavailable') from None
    if metadata.get('rebuild_complete') is not complete or index['completeness']['rebuild_complete'] is not complete:
        raise BundleError('invalid_completeness_claim', reason='build_status_contradicts_bound_omissions')
    asserted = metadata.get('source_fingerprint')
    if not isinstance(asserted, str) or not DIGEST.fullmatch(asserted) or fingerprint != asserted:
        raise BundleError('source_build_mismatch', reason='recomputed_fingerprint_mismatch')
    return metadata, asserted


def verify_build_sources(index, metadata):
    sources = metadata.get('inventory')
    validate_inventory(sources)
    actual = [entry for entry in index['inventory'] if entry['role'] == 'binary_build_snapshot']
    if sorted(sources, key=lambda item: item['logical_path']) != sorted(actual, key=lambda item: item['logical_path']):
        raise BundleError('source_build_mismatch', reason='source_inventory_mismatch')
    paths = {entry['logical_path'] for entry in sources}
    required = {'build_source/' + name for name in
                ('Cargo.toml', 'Cargo.lock', 'src/main.rs', 'assets/cars/roster.json', 'dirty.patch')}
    if 'build_source/src/vehicle/profile.rs' in paths:
        required.add('build_source/assets/cars/physics.json')
    if not required <= paths or any(entry['role'] != 'binary_build_snapshot' for entry in sources):
        raise BundleError('invalid_completeness_claim', reason='required_build_source_missing')


def contains(path, needle):
    tail = b''
    with path.open('rb') as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b''):
            data = tail + chunk
            if needle in data:
                return True
            tail = data[-len(needle):]
    return False
