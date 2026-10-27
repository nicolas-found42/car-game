#!/usr/bin/env python3
"""Check prepared-car integrity, wheel pivots and its source-preservation receipt."""
import hashlib
import json
from pathlib import Path
import struct
import time
import traceback
from collections import Counter

from prepare_fleet import default_meshes, matrix, multiply, mesh_bounds, merged_bounds

ROOT = Path(__file__).resolve().parents[1]


def parse(data):
    magic, version, size = struct.unpack_from('<4sII', data)
    assert (magic, version, size) == (b'glTF', 2, len(data))
    length = struct.unpack_from('<I', data, 12)[0]
    return json.loads(data[20:20 + length]), data[28 + length:]


def triangles(doc, binary, accessor_index):
    accessor = doc['accessors'][accessor_index]
    view = doc['bufferViews'][accessor['bufferView']]
    size = {5121: 1, 5123: 2, 5125: 4}[accessor['componentType']]
    offset = view.get('byteOffset', 0) + accessor.get('byteOffset', 0)
    stride = view.get('byteStride', size)
    values = [int.from_bytes(binary[offset + i * stride:offset + i * stride + size], 'little') for i in range(accessor['count'])]
    assert len(values) % 3 == 0
    return Counter(tuple(values[i:i+3]) for i in range(0, len(values), 3))


def check():
    path = ROOT / 'assets/cars/350z.glb'
    data = path.read_bytes()
    receipt = json.loads(path.with_suffix('.json').read_text())
    assert hashlib.sha256(data).hexdigest() == receipt['prepared_sha256']
    doc, binary = parse(data)
    assert not doc.get('animations') and not doc.get('skins')
    assert not doc.get('extensionsRequired') and not doc.get('extensionsUsed')
    assert all('uri' not in entry for entry in doc['buffers'] + doc['images'])
    assert doc['buffers'][0]['byteLength'] <= len(binary)
    expected = {f'{part}_{axle}_{side}' for part in ('wheel', 'tire')
                for axle in ('front', 'rear') for side in ('left', 'right')}
    nodes = {n.get('name'): n for n in doc['nodes']}
    assert expected <= nodes.keys()
    for name in expected:
        n = nodes[name]
        assert abs(abs(n['translation'][0]) - 0.767) < 1e-6
        assert abs(abs(n['translation'][2]) - 1.325) < 1e-6
    for mesh in doc['meshes']:
        for primitive in mesh['primitives']:
            assert primitive['material'] < len(doc['materials'])
            assert 'extensions' not in primitive
    # Mesh templates are instanced by the eight wheel/tire nodes.
    scene_triangles = sum(sum(doc['accessors'][p['indices']]['count'] // 3
        for p in doc['meshes'][n['mesh']]['primitives']) for n in doc['nodes'] if 'mesh' in n)
    assert scene_triangles == 9310, scene_triangles
    source = ROOT.parent / receipt['source']
    source_checked = source.is_file()
    if source_checked:
        original = source.read_bytes()
        assert hashlib.sha256(original).hexdigest() == receipt['source_sha256']
        original_doc, original_binary = parse(original)
        assert original_binary == binary, 'Embedded geometry and PNG bytes changed'
        assert original_doc['nodes'] == doc['nodes'], 'Wheel transforms changed'
        assert original_doc['accessors'] == doc['accessors'], 'Geometry accessors changed'
        for old, new in zip(original_doc['meshes'], doc['meshes'], strict=True):
            for a, b in zip(old['primitives'], new['primitives'], strict=True):
                assert a['attributes'] == b['attributes'] and a['indices'] == b['indices']
    print(f'ASSETS PASS: {scene_triangles} scene triangles; 8 wheel/tire nodes; '
          f'{len(doc["materials"])} materials; source bytes {"verified" if source_checked else "not present (receipt verified)"}')


def check_fleet():
    roster = json.loads((ROOT / 'assets/cars/roster.json').read_text())
    assert len(roster) == 13 and roster[0]['id'] == '350z'
    assert len({car['id'] for car in roster}) == 13
    assert Counter(car['game_id'] for car in roster[1:]) == {
        'ford-racing-2': 2, 'gran-turismo': 2, 'gran-turismo-4': 2,
        'midnight-club-3-remix': 2, 'midnight-club-los-angeles': 2, 'redline': 2}
    # The game-owned reference edition is immutable across this fleet addition.
    assert hashlib.sha256((ROOT / 'assets/cars/350z.glb').read_bytes()).hexdigest() == 'e79630558ed16c54942afc552838fd465a73bf507754947342bd65a805d37874'
    reports = []
    errors = []
    for car in roster[1:]:
        try:
            reports.append(check_car(car))
        except Exception:
            errors.append({'id': car.get('id'), 'error': traceback.format_exc()})
    output = ROOT / 'artifacts' / f'asset-integrity-{time.strftime("%Y%m%dT%H%M%SZ", time.gmtime())}.json'
    output.parent.mkdir(exist_ok=True)
    output.write_text(json.dumps({'reports': reports, 'errors': errors}, indent=2) + '\n')
    (ROOT / 'artifacts' / 'latest-asset-integrity').write_text(str(output.relative_to(ROOT)) + '\n')
    if errors:
        reasons = '; '.join(f'{error["id"]}: {error["error"].strip().splitlines()[-1]}' for error in errors)
        raise AssertionError(f'fleet asset check failed for {[error["id"] for error in errors]}; partial report: {output.relative_to(ROOT)}; {reasons}')
    print('FLEET ASSETS PASS: 12 additions; 2 per game; 350Z unchanged; all source geometry/PNG bytes and texture references preserved; 4 animated wheel assemblies each')


def check_car(car):
    data = (ROOT / 'assets' / car['path']).read_bytes()
    receipt = json.loads((ROOT / 'assets' / car['path']).with_suffix('.json').read_text())
    assert hashlib.sha256(data).hexdigest() == receipt['prepared_sha256'], car['id']
    doc, binary = parse(data)
    original_data = (ROOT.parent / receipt['source']).read_bytes()
    assert hashlib.sha256(original_data).hexdigest() == receipt['source_sha256']
    source, source_binary = parse(original_data)
    assert binary[:len(source_binary)] == source_binary, f'{car["id"]}: source vertex/image bytes changed'
    assert doc['accessors'][:len(source['accessors'])] == source['accessors']
    assert doc['bufferViews'][:len(source['bufferViews'])] == source['bufferViews'] and doc['images'] == source['images']
    assert doc['textures'] == source['textures'] and doc['samplers'] == source['samplers']
    assert not doc.get('animations') and not doc.get('skins')
    assert all('uri' not in x for x in doc['images'] + doc['buffers'])
    assert doc['scene'] == 0 and doc['scenes'][0]['nodes'] == list(range(5))
    parents = {c: i for i, n in enumerate(doc['nodes']) for c in n.get('children', [])}
    active_source = {i for i, _, _ in default_meshes(source)}
    source_world = {i: world for i, world, _ in default_meshes(source)}
    assert {m['source_node'] for m in receipt['mesh_nodes']} == active_source - set(receipt['excluded_nodes'])
    assert all('HELMET' in source['nodes'][i].get('name', '').upper() for i in receipt['excluded_nodes'])
    from reference_geometry import validate_receipt, transform_for
    geometry = validate_receipt(car, source, receipt)
    scene_triangles = 0
    for record in receipt['mesh_nodes']:
        node = doc['nodes'][record['prepared_node']]
        old_node = source['nodes'][record['source_node']]
        assert record['source_world_matrix'] == source_world[record['source_node']]
        expected = multiply(receipt['normalization'], source_world[record['source_node']])
        expected = multiply(transform_for(geometry, record['wheel']), expected)
        actual = multiply(matrix(doc['nodes'][parents[record['prepared_node']]]), matrix(node))
        assert all(abs(a-b) < 1e-6 for a, b in zip(expected, actual)), car['id']
        mesh = doc['meshes'][node['mesh']]
        old_mesh = source['meshes'][old_node['mesh']]
        recovered = {i: Counter() for i in range(len(old_mesh['primitives']))}
        for index, new in enumerate(mesh['primitives']):
            source_index = new.get('extras', {}).get('gridSourcePrimitive', index)
            old = old_mesh['primitives'][source_index]
            for field in ('attributes', 'mode', 'targets'):
                assert old.get(field) == new.get(field), (car['id'], field)
            recovered[source_index].update(triangles(doc, binary, new['indices']))
            old_material = source['materials'][old['material']]
            material = doc['materials'][new['material']]
            # Every textured surface keeps its original texture and sampler references.
            assert old_material.get('pbrMetallicRoughness', {}).get('baseColorTexture') == material.get('pbrMetallicRoughness', {}).get('baseColorTexture')
            scene_triangles += doc['accessors'][new['indices']]['count'] // 3
        for index, old in enumerate(old_mesh['primitives']):
            assert recovered[index] == triangles(source, source_binary, old['indices']), f'{car["id"]}: source triangles lost, duplicated or changed'
    assert scene_triangles == receipt['scene_triangles'] == car['triangles']
    for i, name in enumerate(('front_left', 'front_right', 'rear_left', 'rear_right')):
        wheel = doc['nodes'][i + 1]
        assert wheel['name'] == 'wheel_' + name and wheel['children']
        assert wheel['translation'] == car['wheels'][i]
        assert (wheel['translation'][0] < 0) == (i % 2 == 0)
        assert (wheel['translation'][2] < 0) == (i < 2)
        bounds = merged_bounds([mesh_bounds(doc, doc['nodes'][child]['mesh'], matrix(doc['nodes'][child])) for child in wheel['children']])
        assert abs((bounds[1][1] - bounds[0][1]) / 2 - car['radii'][i]) < 1e-6
        assert 0.22 < car['radii'][i] < 0.5
    assert 1.0 < car['half_length'] < 3.0 and 0.65 <= car['half_width'] <= 1.1
    return {'id': car['id'], 'game': car['game_id'], 'triangles': scene_triangles,
            'source_geometry_and_images_unchanged': True, 'four_wheel_pivots': True}


if __name__ == '__main__':
    try:
        check()
        check_fleet()
    except Exception:
        stamp = time.strftime('%Y%m%dT%H%M%SZ', time.gmtime())
        output = ROOT / 'artifacts' / f'asset-check-failure-{stamp}.json'
        output.parent.mkdir(exist_ok=True)
        output.write_text(json.dumps({
            'at': stamp,
            'error': traceback.format_exc(),
            'cwd': str(ROOT),
        }, indent=2) + '\n')
        print(f'ASSET CHECK FAILURE retained at {output}')
        raise
