"""Small real-producer-shaped inventories; no gameplay or rebuild acceptance claims."""
import json
from pathlib import Path

from provenance_legacy import store_bytes


def complete_runtime_index(run):
    entries = []
    def add(role, logical, data):
        entry = store_bytes(run, data, role, logical)
        entries.append(entry)
        return entry
    catalog = json.dumps([{'id': 'fixture', 'path': 'cars/fixture.glb'}]).encode()
    add('executable', 'bin/car-game', b'fixture executable bytes')
    add('embedded_catalog', 'catalog/roster.json', catalog)
    add('receipt_or_catalog', 'assets/cars/roster.json', catalog)
    asset = add('prepared_asset', 'assets/cars/fixture.glb', b'fixture prepared asset')
    add('receipt_or_catalog', 'assets/cars/fixture.json',
        json.dumps({'prepared_sha256': asset['sha256']}).encode())
    for name in ('engine', 'tires', 'impact'):
        add('runtime_asset', f'assets/audio/{name}.wav', b'fixture audio')
    for index in range(4):
        add('runtime_asset', f'assets/textures/facade-{index}.png', b'fixture texture')
    config = {'settings': {'mode': 'smoke', 'car': 'fixture', 'fixed_hz': 120},
              'arguments': ['--smoke', '--offscreen'], 'environment': {'CAR_GAME_MUTE': '1'},
              'asset_root_kind': 'checkout_assets'}
    add('effective_config', 'config/effective.json', json.dumps(config).encode())
    flags = {'inventory_integrity': True, 'runtime_material_complete': True,
             'runtime_replay_complete': False, 'rebuild_complete': False}
    prerequisites = [{'id': name, 'replay_required': True, 'availability': 'external_required'}
                     for name in ('compatible_os_arch_dynamic_libraries', 'gpu_renderer_audio_drivers')]
    capture = {'schema': 'car-game.provenance', 'version': 1, 'policy': 'grid-portable-allowlist-v1',
               'run_id': 'producer-fixture', 'inventory': list(entries), 'completeness': flags,
               'omissions': [], 'replay_prerequisites': prerequisites,
               'binary_build': {'availability': 'unavailable', 'rebuild_complete': False}}
    frozen = add('capture_index', 'provenance/capture.json', json.dumps(capture).encode())
    return {'schema': 2, 'provenance_version': 1, 'policy': capture['policy'],
            'run_id': capture['run_id'], 'inventory': entries, 'completeness': flags,
            'omissions': [], 'replay_prerequisites': prerequisites, 'capture_sha256': frozen['sha256']}
