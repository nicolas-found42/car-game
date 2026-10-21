#!/usr/bin/env python3
"""Prepare twelve game-owned cars without changing source vertex or image bytes.

Only the selected default scene is assembled. Explicit source wheel ownership is
reparented under four game pivots; matrices retain each mesh's original geometry.
The original 350Z and its receipt remain byte-for-byte unchanged.
"""
import argparse
from copy import deepcopy
import hashlib
import itertools
import json
from pathlib import Path
import re
import struct

from prepare_assets import ROOT, png_pixels

SELECTION = [
    ('fr2-cobra', 'COBRA', 'SVT Cobra'),
    ('fr2-fr500', 'FR500', 'Mustang FR500'),
    ('gt-rx7', 'GT_SIMULATION__AFA7N', 'RX-7 FD A spec'),
    ('gt-viper', 'GT_SIMULATION__DVPGN', 'Viper GTS'),
    ('gt4-m3', 'GT4_BMW_0011', 'BMW M3 CSL'),
    ('gt4-nsx', 'GT4_ACRA0003', 'Acura NSX'),
    ('mc3-skyline', 'MC3_VP_SKYLINE_02', 'Skyline GT-R'),
    ('mc3-supra', 'MC3_VP_SUPRA_98', 'Supra'),
    ('mcla-evo', 'MCLA_VP_MIT_EVO_06', 'Lancer Evolution IX'),
    ('mcla-skyline', 'MCLA_VP_NSN_SKYLINE_99', 'Skyline GT-R V-Spec'),
    ('redline-911', 'REDLINE_ADDON_072_DP_911_PACK_II_REDPLUG_ZIP_DP91130_D40AEBAC2F82_2F607564778B6BAC', 'Porsche 911 SC 3.0'),
    ('redline-gt500', 'REDLINE_ADDON_080_MUSTANG_S197_PACK_ZIP_MUSTANG_NNB_GT500_867D18EE049F_18D1CC5DE2FE0CDE', 'Mustang GT500 (2008)'),
]
WHEELS = ['front_left', 'front_right', 'rear_left', 'rear_right']
IDENTITY = [1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1]


def parse(data):
    magic, version, size = struct.unpack_from('<4sII', data)
    assert (magic, version, size) == (b'glTF', 2, len(data))
    length, kind = struct.unpack_from('<I4s', data, 12)
    assert kind == b'JSON'
    return json.loads(data[20:20 + length]), data[28 + length:]


def encode(doc, binary):
    data = json.dumps(doc, separators=(',', ':')).encode()
    data += b' ' * (-len(data) % 4)
    assert len(binary) % 4 == 0
    return (struct.pack('<4sII', b'glTF', 2, 28 + len(data) + len(binary))
            + struct.pack('<I4s', len(data), b'JSON') + data
            + struct.pack('<I4s', len(binary), b'BIN\0') + binary)


def multiply(a, b):
    # glTF column-major affine matrices.
    return [sum(a[k * 4 + row] * b[col * 4 + k] for k in range(4))
            for col in range(4) for row in range(4)]


def matrix(node):
    if 'matrix' in node:
        return node['matrix'][:]
    x, y, z, w = node.get('rotation', [0, 0, 0, 1])
    sx, sy, sz = node.get('scale', [1, 1, 1])
    return [(1-2*y*y-2*z*z)*sx, (2*x*y+2*z*w)*sx, (2*x*z-2*y*w)*sx, 0,
            (2*x*y-2*z*w)*sy, (1-2*x*x-2*z*z)*sy, (2*y*z+2*x*w)*sy, 0,
            (2*x*z+2*y*w)*sz, (2*y*z-2*x*w)*sz, (1-2*x*x-2*y*y)*sz, 0,
            *node.get('translation', [0, 0, 0]), 1]


def point(m, p):
    return [sum(m[k * 4 + row] * p[k] for k in range(3)) + m[12 + row]
            for row in range(3)]


def mesh_bounds(doc, mesh, transform):
    points = []
    for primitive in doc['meshes'][mesh]['primitives']:
        accessor = doc['accessors'][primitive['attributes']['POSITION']]
        points.extend(point(transform, p) for p in itertools.product(
            *zip(accessor['min'], accessor['max'])))
    return [[min(p[i] for p in points) for i in range(3)],
            [max(p[i] for p in points) for i in range(3)]]


def merged_bounds(bounds):
    return [[min(b[0][i] for b in bounds) for i in range(3)],
            [max(b[1][i] for b in bounds) for i in range(3)]]


def default_meshes(doc):
    result = []

    def walk(index, parent, ancestry):
        node = doc['nodes'][index]
        world = multiply(parent, matrix(node))
        chain = ancestry + [(index, world)]
        if 'mesh' in node:
            result.append((index, world, chain))
        for child in node.get('children', []):
            walk(child, world, chain)

    for index in doc['scenes'][doc.get('scene', 0)]['nodes']:
        walk(index, IDENTITY, [])
    return result


def wheel_owner(doc, index, world, chain, game):
    node = doc['nodes'][index]
    name = node.get('name', '')
    if game == 'gran-turismo':
        extra = node.get('extras', {})
        if 'nativeWheelIndex' in extra:
            return str(extra['nativeWheelIndex']), extra['nativePivotPositionMetres']
    if game == 'gran-turismo-4' and re.fullmatch(r'(wheel|tire)_(front|rear)_(left|right)', name):
        return name.split('_', 1)[1], world[12:15]
    if game == 'midnight-club-3-remix' and 'nativeWheelJoint' in node.get('extras', {}):
        return node['extras']['nativeWheelJoint'], world[12:15]
    if game == 'midnight-club-los-angeles':
        match = re.search(r'@axl_(\d+)/', name)
        if match:
            return match[1], world[12:15]
    for ancestor, transform in chain:
        name = doc['nodes'][ancestor].get('name', '')
        if game == 'ford-racing-2' and re.match(r'HUB_(FRONT|BACK)_(LEFT|RIGHT)_', name):
            return name.split('_0x')[0], transform[12:15]
        if game == 'redline' and re.fullmatch(r'WHEEL_[0-3]', name):
            return name, transform[12:15]
    return None


def accessor_values(doc, binary, index):
    accessor = doc['accessors'][index]
    view = doc['bufferViews'][accessor['bufferView']]
    kind = {5121: 'B', 5123: 'H', 5125: 'I', 5126: 'f'}[accessor['componentType']]
    count = {'SCALAR': 1, 'VEC3': 3}[accessor['type']]
    fmt = '<' + kind * count
    stride = view.get('byteStride', struct.calcsize(fmt))
    start = view.get('byteOffset', 0) + accessor.get('byteOffset', 0)
    return [struct.unpack_from(fmt, binary, start + i * stride) for i in range(accessor['count'])]


def scope_lens_materials(doc, binary, included, normalization, game):
    """Disentangle shared cabin/lamp shaders while retaining exact source triangles."""
    partitions = 0
    for index, world, _ in included:
        node = doc['nodes'][index]
        mesh = doc['meshes'][node['mesh']]
        name = node.get('name', '').lower()
        if game == 'midnight-club-3-remix':
            role = 'front' if ('headlight' in name or 'hl_glass' in name) else 'rear' if ('tail_light' in name or '_tl_' in name) else None
            if role:
                for primitive in mesh['primitives']:
                    material = doc['materials'][primitive['material']]
                    if material.get('name') in ('colored_glass', 'car_window'):
                        material = deepcopy(material)
                        material.setdefault('extras', {})['gridLightCover'] = role
                        material['extras']['gridSourceMaterial'] = primitive['material']
                        primitive['material'] = len(doc['materials'])
                        doc['materials'].append(material)
        if game != 'redline' or name != 'body':
            continue
        prepared = []
        for source_primitive, primitive in enumerate(mesh['primitives']):
            material = doc['materials'][primitive['material']]
            primitive.setdefault('extras', {})['gridSourcePrimitive'] = source_primitive
            if material.get('alphaMode') != 'BLEND' or material.get('name') != 'mutang_nnb_gt500_uv.pct':
                prepared.append(primitive)
                continue
            transform = multiply(normalization, world)
            positions = accessor_values(doc, binary, primitive['attributes']['POSITION'])
            indices = [v[0] for v in accessor_values(doc, binary, primitive['indices'])]
            cabin, exterior = [], []
            for at in range(0, len(indices), 3):
                triangle = indices[at:at + 3]
                center = point(transform, [sum(positions[j][k] for j in triangle) / 3 for k in range(3)])
                # This GT500 material has a measured gap: cabin triangle centroids
                # are at least .485 high, front lamp covers at most .184. A Z bound
                # would split the connected rear window into two material roles.
                target = cabin if center[1] > 0.30 else exterior
                target.extend(triangle)
            assert cabin and exterior, (name, source_primitive)
            for values, role in ((exterior, 'exterior_lens'), (cabin, 'cabin')):
                new = deepcopy(primitive)
                tinted = deepcopy(material)
                tinted.setdefault('extras', {})['gridGlassRole'] = role
                tinted['extras']['gridSourceMaterial'] = primitive['material']
                new['material'] = len(doc['materials'])
                doc['materials'].append(tinted)
                start = len(binary)
                binary += struct.pack('<' + 'I' * len(values), *values)
                view = len(doc['bufferViews'])
                doc['bufferViews'].append({'buffer': 0, 'byteOffset': start, 'byteLength': len(values) * 4, 'target': 34963})
                new['indices'] = len(doc['accessors'])
                doc['accessors'].append({'bufferView': view, 'componentType': 5125, 'count': len(values), 'type': 'SCALAR'})
                prepared.append(new)
            partitions += 1
        mesh['primitives'] = prepared
    doc['buffers'][0]['byteLength'] = len(binary)
    return binary, partitions


def materials(doc, binary, game, ident):
    repairs = []
    used = {p['material'] for m in doc['meshes'] for p in m['primitives']}
    for index in used:
        m = doc['materials'][index]
        name = m.get('name', '')
        pbr = m.setdefault('pbrMetallicRoughness', {})
        # Recovered unlit inspection exports need the same daylight response as GRID.
        if m.get('extensions', {}).pop('KHR_materials_unlit', None) is not None:
            repairs.append([index, 'daylight PBR response instead of inspection-unlit'])
        if not m.get('extensions'):
            m.pop('extensions', None)
        if game == 'gran-turismo':
            pbr['roughnessFactor'] = 0.55
        elif game in ('redline', 'ford-racing-2'):
            pbr['metallicFactor'] = 0.08
            pbr['roughnessFactor'] = 0.42
        if game == 'gran-turismo-4':
            finish = max(m.get('extras', {}).get('gt4_material', {}).get('specular', [0, 0, 0])[:3])
            if finish > 0 and m.get('alphaMode') == 'MASK':
                m['alphaCutoff'] = 1 / 255
                pbr.setdefault('baseColorFactor', [1, 1, 1, 1])[3] = 1
                repairs.append([index, 'retain nonzero paint alpha'])
            if finish > 0:
                pbr['roughnessFactor'] = 0.32
            if m.get('alphaMode') == 'BLEND' and 'baseColorTexture' in pbr:
                pixels = png_pixels(doc, binary, pbr['baseColorTexture']['index'])
                marker = sum(pixels[i] == 8 and pixels[i + 1] == 255 and pixels[i + 2] in (16, 55)
                             and pixels[i + 3] >= 63 for i in range(0, len(pixels), 4))
                if marker >= len(pixels) / 16:
                    # Retain the texture's cutout alpha and neutralize keyed green RGB.
                    pbr['baseColorFactor'] = [0, 0, 0, 0.7]
                    pbr['roughnessFactor'] = 0.18
                    m['doubleSided'] = True
                    repairs.append([index, 'neutral marker-glass preview; original PNG retained'])
        if game == 'midnight-club-3-remix':
            if name in ('car_window', 'colored_glass'):
                pbr.update(baseColorFactor=[0.025, 0.045, 0.06, 1], metallicFactor=0.25, roughnessFactor=0.18)
                repairs.append([index, 'artistic dark glass preview'])
            elif name == 'carpaint':
                color = [0.025, 0.10, 0.34, 1] if ident == 'mc3-skyline' else [0.38, 0.035, 0.025, 1]
                pbr.update(baseColorFactor=color, metallicFactor=0.38, roughnessFactor=0.28)
                repairs.append([index, 'artistic GRID paint; not a recovered factory colour'])
            elif name in ('tire', 'rubber', 'black_matte', 'wheel_inner'):
                pbr.update(baseColorFactor=[0.08, 0.08, 0.08, 1], metallicFactor=0.0, roughnessFactor=0.95)
                repairs.append([index, 'native rubber/black material role to dark rough PBR'])
            elif name in ('more_chrome_wheel_hi_lod', 'default_shiny_rim', 'masked_chrome'):
                pbr.update(baseColorFactor=[0.22, 0.22, 0.22, 1], metallicFactor=0.7, roughnessFactor=0.25)
                repairs.append([index, 'native rim/chrome material role to alloy PBR'])
            light_cover = m.get('extras', {}).get('gridLightCover')
            if light_cover:
                color = [0.55, 0.62, 0.7, 0.15] if light_cover == 'front' else [0.45, 0.008, 0.012, 0.45]
                pbr.update(baseColorFactor=color, metallicFactor=0.1, roughnessFactor=0.12)
                m['alphaMode'] = 'BLEND'
                repairs.append([index, f'clear {light_cover} lamp cover; cabin material remains separate'])
        if game == 'midnight-club-los-angeles':
            shader = m.get('extras', {}).get('sourceShader')
            if shader in ('CarPaintCustomizable', 'CarPaintNormalMapped'):
                color = [0.33, 0.025, 0.02, 1] if ident == 'mcla-evo' else [0.025, 0.08, 0.28, 1]
                pbr.update(baseColorFactor=color, metallicFactor=0.38, roughnessFactor=0.28)
                repairs.append([index, 'artistic GRID paint for explicit native paint shader'])
            elif shader in ('CarGlass', 'BlackOpaqueGlass'):
                pbr.update(baseColorFactor=[0.025, 0.045, 0.06, 1], metallicFactor=0.25, roughnessFactor=0.18)
                repairs.append([index, 'artistic dark glass for explicit native glass shader'])
            elif shader in ('RimMain', 'RimBadge', 'Chrome', 'BrushedMetal'):
                pbr.update(baseColorFactor=[0.28, 0.28, 0.28, 1], metallicFactor=0.7, roughnessFactor=0.25)
                repairs.append([index, 'native metal/rim shader to alloy PBR'])
            elif shader in ('blackmatte', 'Underbody', 'GrillOpaque', 'BumperSubframe'):
                pbr.update(baseColorFactor=[0.025, 0.025, 0.025, 1], roughnessFactor=0.95)
                repairs.append([index, 'native dark trim shader to rough black PBR'])
            if '/taillight' in name and shader in ('CarLight', 'bumpedglass'):
                pbr.update(baseColorFactor=[0.45, 0.008, 0.012, 1], roughnessFactor=0.25)
                repairs.append([index, 'artistic red tailglass on explicit taillight part'])
                if shader == 'bumpedglass':
                    pbr['baseColorFactor'][3] = 0.25
                    m['alphaMode'] = 'BLEND'
                    repairs.append([index, 'translucent rear lamp cover reveals source reflector geometry'])
            if '/headlight' in name and shader == 'bumpedglass':
                pbr.update(baseColorFactor=[0.55, 0.62, 0.7, 0.15], metallicFactor=0.1, roughnessFactor=0.12)
                m['alphaMode'] = 'BLEND'
                repairs.append([index, 'clear headlamp cover reveals source reflector geometry'])
        if game == 'redline':
            role = m.get('extras', {}).get('gridGlassRole')
            if role == 'cabin' or name == 'dp911vitres.png.txr':
                pbr.update(baseColorFactor=[0.025, 0.045, 0.06, 1], metallicFactor=0.25, roughnessFactor=0.18)
                m['alphaMode'] = 'MASK'
                m['alphaCutoff'] = 1 / 255
                repairs.append([index, 'artistic opaque dark cabin preview; preserve alpha-zero cutouts'])
            elif role == 'exterior_lens':
                pbr.update(baseColorFactor=[0.6, 0.65, 0.7, 0.2], metallicFactor=0.1, roughnessFactor=0.12)
                repairs.append([index, 'clear exterior lenses kept separate from cabin tint'])
            elif '_tire.' in name:
                pbr.update(baseColorFactor=[0.12, 0.12, 0.12, 1], metallicFactor=0.0, roughnessFactor=0.95)
                repairs.append([index, 'named tire material to dark rubber PBR'])
            elif '_wheel.' in name or '_brake.' in name:
                pbr.update(baseColorFactor=[0.28, 0.28, 0.28, 1], metallicFactor=0.55, roughnessFactor=0.28)
                repairs.append([index, 'named wheel/brake material to alloy PBR'])
        if name.startswith('tire_'):
            pbr.update(baseColorFactor=[0.12, 0.12, 0.12, 1], roughnessFactor=0.95)
    return repairs


def prepare(root, catalog, ident, code, title):
    source = root / 'models' / f'{code}.glb'
    original = source.read_bytes()
    doc, binary = parse(original)
    entry = catalog[code]
    game = entry['game']
    meshes = default_meshes(doc)
    groups = {}
    included = []
    excluded = []
    for index, world, chain in meshes:
        node = doc['nodes'][index]
        if 'HELMET' in node.get('name', '').upper():
            excluded.append(index)
            continue
        owner = wheel_owner(doc, index, world, chain, game)
        if owner:
            groups.setdefault(owner[0], {'pivot': owner[1], 'nodes': []})['nodes'].append((index, world))
        included.append((index, world, owner[0] if owner else None))
    assert len(groups) == 4, (code, groups.keys())
    rotation = IDENTITY[:]
    if game in ('ford-racing-2', 'redline'):
        rotation[0] = rotation[10] = -1
    pivots = [point(rotation, g['pivot']) for g in groups.values()]
    center = [sum(p[i] for p in pivots) / 4 for i in range(3)]
    normalization = rotation[:]
    normalization[12:15] = [-center[0], -0.20 - center[1], -center[2]]
    source_materials = sorted({p['material'] for i, _, _ in included for p in doc['meshes'][doc['nodes'][i]['mesh']]['primitives']})
    source_binary = binary
    binary, partitions = scope_lens_materials(doc, binary, included, normalization, game)
    wheel_data = {}
    for key, group in groups.items():
        pivot = point(normalization, group['pivot'])
        slot = ('front' if pivot[2] < 0 else 'rear') + ('_left' if pivot[0] < 0 else '_right')
        bounds = merged_bounds([mesh_bounds(doc, doc['nodes'][i]['mesh'], multiply(normalization, w))
                                for i, w in group['nodes']])
        radius = (bounds[1][1] - bounds[0][1]) / 2
        assert 0.22 < radius < 0.5, (code, slot, radius)
        wheel_data[slot] = {'key': key, 'pivot': pivot, 'radius': radius}
    assert set(wheel_data) == set(WHEELS)
    nodes = [{'name': 'GRID body', 'children': []}]
    for slot in WHEELS:
        nodes.append({'name': 'wheel_' + slot, 'translation': wheel_data[slot]['pivot'], 'children': []})
    mapping = []
    body_bounds = []
    for index, world, owner in included:
        original_node = doc['nodes'][index]
        transform = multiply(normalization, world)
        parent = 0
        if owner is not None:
            slot = next(s for s, g in wheel_data.items() if g['key'] == owner)
            parent = WHEELS.index(slot) + 1
            pivot = wheel_data[slot]['pivot']
            transform[12:15] = [transform[12+i] - pivot[i] for i in range(3)]
        else:
            body_bounds.append(mesh_bounds(doc, original_node['mesh'], transform))
        new_index = len(nodes)
        nodes[parent]['children'].append(new_index)
        nodes.append({'name': 'source/' + original_node.get('name', str(index)),
                      'mesh': original_node['mesh'], 'matrix': transform,
                      'extras': {'sourceNode': index}})
        mapping.append({'source_node': index, 'prepared_node': new_index,
                        'source_world_matrix': world, 'wheel': parent - 1 if parent else None})
    source_scene = doc.get('scene', 0)
    triangles = sum(sum(doc['accessors'][p['indices']]['count'] // 3 for p in doc['meshes'][doc['nodes'][i]['mesh']]['primitives']) for i, _, _ in included)
    doc['nodes'] = nodes
    doc['scenes'] = [{'name': title, 'nodes': list(range(5))}]
    doc['scene'] = 0
    # Keep only meshes used by this scene; binary buffers and all source accessors stay exact.
    used_meshes = sorted({n['mesh'] for n in nodes if 'mesh' in n})
    mesh_map = {old: new for new, old in enumerate(used_meshes)}
    doc['meshes'] = [doc['meshes'][i] for i in used_meshes]
    for node in nodes:
        if 'mesh' in node:
            node['mesh'] = mesh_map[node['mesh']]
    repairs = materials(doc, binary, game, ident)
    used_materials = sorted({p['material'] for m in doc['meshes'] for p in m['primitives']})
    material_map = {old: new for new, old in enumerate(used_materials)}
    doc['materials'] = [doc['materials'][i] for i in used_materials]
    for mesh in doc['meshes']:
        for primitive in mesh['primitives']:
            primitive['material'] = material_map[primitive['material']]
            primitive.pop('extensions', None)
    doc.pop('extensions', None)
    doc.pop('extensionsRequired', None)
    doc.pop('extensionsUsed', None)
    # No animations/skins exist in these static source profiles.
    assert not doc.get('animations') and not doc.get('skins')
    bounds = merged_bounds(body_bounds)
    receipt = {'source': f'carmodels/dealership/dealership/models/{code}.glb',
               'source_sha256': hashlib.sha256(original).hexdigest(), 'source_scene': source_scene,
               'source_meshes': used_meshes, 'source_materials': source_materials,
               'normalization': normalization, 'mesh_nodes': mapping, 'excluded_nodes': excluded,
               'repairs': repairs, 'scene_triangles': triangles,
               'source_binary_sha256': hashlib.sha256(source_binary).hexdigest(),
               'index_partitions': partitions,
               'limits': ['Source-era geometry and textures retained; not original game shader emulation.',
                          'PBR/glass translation is an explicit GRID preview choice.']}
    doc.setdefault('extras', {})['gridPreparation'] = {'id': ident, 'source_sha256': receipt['source_sha256']}
    output = ROOT / 'assets/cars' / f'{ident}.glb'
    result = encode(doc, binary)
    output.write_bytes(result)
    receipt['prepared_sha256'] = hashlib.sha256(result).hexdigest()
    output.with_suffix('.json').write_text(json.dumps(receipt, indent=2) + '\n')
    print(f'{ident}: {triangles:,} triangles, {len(included)} mesh nodes, radii {[round(wheel_data[s]["radius"], 4) for s in WHEELS]}')
    return {'id': ident, 'name': title, 'game': entry['gameLabel'], 'game_id': game,
            'source_code': code, 'variant': 'add-on' if entry.get('sourceVariant') == 'addon' else '',
            'path': f'cars/{ident}.glb', 'visual_offset': 0.0,
            'wheels': [wheel_data[s]['pivot'] for s in WHEELS],
            'radii': [wheel_data[s]['radius'] for s in WHEELS],
            'half_width': min(1.1, max(0.65, (bounds[1][0] - bounds[0][0]) / 2 * 0.96)),
            'half_length': (bounds[1][2] - bounds[0][2]) / 2 * 0.97,
            'body_center_z': (bounds[1][2] + bounds[0][2]) / 2,
            'roof_height': bounds[1][1], 'wheel_nodes': 4, 'triangles': triangles}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source-root', type=Path, default=ROOT.parent / 'carmodels')
    args = parser.parse_args()
    root = args.source_root / 'dealership/dealership'
    catalog = {c['code']: c for c in json.loads((root / 'catalog.json').read_text())}
    baseline = {'id': '350z', 'name': 'Nissan 350Z / Z33 / 2003', 'game': 'Gran Turismo 4',
                'game_id': 'gran-turismo-4', 'source_code': 'GT4_NISS0004', 'variant': 'original baseline',
                'path': 'cars/350z.glb', 'visual_offset': -0.4,
                'wheels': [[x, y, z] for z, y in [(-1.325, 0.20985), (1.325, 0.21885)] for x in [-0.767, 0.767]],
                'radii': [0.32985, 0.32985, 0.33885, 0.33885],
                'half_width': 0.86, 'half_length': 1.99, 'body_center_z': 0.0,
                'roof_height': 0.74, 'wheel_nodes': 8, 'triangles': 9310}
    roster = [baseline] + [prepare(root, catalog, *selection) for selection in SELECTION]
    (ROOT / 'assets/cars/roster.json').write_text(json.dumps(roster, indent=2) + '\n')


if __name__ == '__main__':
    main()
