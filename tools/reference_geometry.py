"""Explicit C5 stock geometry correction, separate from raw archive dimensions.

Only affine node transforms change; source vertex, index and image bytes stay
intact. The source shell remains an approximation (including wheel-arch styling).
Nominal tire sizes are unloaded dimensions, not measured dynamic rolling radii.
"""
from copy import deepcopy

from prepare_fleet import IDENTITY, WHEELS, mesh_bounds, merged_bounds, multiply

C5 = {
    'reference': 'a08', 'source': 'am_c5_specs',
    'wheelbase_m': 2.6543, 'tracks_m': [1.58496, 1.59004],
    'radii_m': [0.3219, 0.33185],
    'width_m': 1.86944, 'length_m': 4.56438, 'height_m': 1.21158,
    'basis': '2003 Z06: 104.5/62.4/62.6/73.6/179.7/47.7 inches; 265/40R17 and 295/35R18',
}


def affine(scale, offset):
    result = IDENTITY[:]
    result[0], result[5], result[10] = scale
    result[12:15] = offset
    return result


def correction(ident, wheel_data, bounds):
    if ident != 'ref-a08':
        return None
    before = deepcopy(wheel_data)
    old_ground = sum(before[s]['pivot'][1] - before[s]['radius'] for s in WHEELS) / 4
    ground = -0.20 - sum(C5['radii_m']) / 2
    scale = [C5['width_m'] / (bounds[1][0] - bounds[0][0]),
             C5['height_m'] / (bounds[1][1] - old_ground),
             C5['length_m'] / (bounds[1][2] - bounds[0][2])]
    body = affine(scale, [0, ground - old_ground * scale[1], 0])
    wheels = []
    for i, slot in enumerate(WHEELS):
        old = before[slot]
        radius = C5['radii_m'][i // 2]
        pivot = [(-1 if i % 2 == 0 else 1) * C5['tracks_m'][i // 2] / 2,
                 ground + radius, (-1 if i < 2 else 1) * C5['wheelbase_m'] / 2]
        factor = radius / old['radius']
        # Scale each wheel isotropically about its source hub, then relocate it.
        wheels.append(affine([factor] * 3, [pivot[k] - factor * old['pivot'][k] for k in range(3)]))
        wheel_data[slot].update(pivot=pivot, radius=radius)
    return {'target': deepcopy(C5), 'body_matrix': body, 'wheel_matrices': wheels,
            'authored_wheels': before,
            'limits': 'Affine stock envelope and wheel correction; source shell arches are approximate; unloaded tire radii.'}


def transform_for(receipt, wheel):
    if receipt is None:
        return IDENTITY
    return receipt['body_matrix'] if wheel is None else receipt['wheel_matrices'][wheel]


def corrected_bounds(doc, included, normalization, body_matrix):
    return merged_bounds([mesh_bounds(doc, doc['nodes'][i]['mesh'],
                                     multiply(body_matrix, multiply(normalization, world)))
                          for i, world, owner in included if owner is None])


def from_source(car, source, normalization):
    from prepare_fleet import default_meshes, point, wheel_owner
    included, groups = [], {}
    for i, world, chain in default_meshes(source):
        if 'HELMET' in source['nodes'][i].get('name', '').upper():
            continue
        owner = wheel_owner(source, i, world, chain, car['game_id'])
        included.append((i, world, owner[0] if owner else None))
        if owner:
            groups.setdefault(owner[0], {'pivot': owner[1], 'nodes': []})['nodes'].append((i, world))
    wheels = {}
    for key, group in groups.items():
        pivot = point(normalization, group['pivot'])
        slot = ('front' if pivot[2] < 0 else 'rear') + ('_left' if pivot[0] < 0 else '_right')
        bounds = merged_bounds([mesh_bounds(source, source['nodes'][i]['mesh'], multiply(normalization, w))
                                for i, w in group['nodes']])
        wheels[slot] = {'key': key, 'pivot': pivot, 'radius': (bounds[1][1] - bounds[0][1]) / 2}
    bounds = corrected_bounds(source, included, normalization, IDENTITY)
    result = correction(car['id'], wheels, bounds)
    return result, wheels, corrected_bounds(source, included, normalization, transform_for(result, None))


def validate_receipt(car, source, receipt):
    if car['id'] != 'ref-a08':
        assert 'reference_geometry' not in receipt, 'Unexpected geometry correction'
        return None
    geometry, target_wheels, bounds = from_source(car, source, receipt['normalization'])
    assert receipt.get('reference_geometry') == geometry, 'C5 reference transform differs from source-derived correction'
    for i, slot in enumerate(WHEELS):
        assert car['wheels'][i] == target_wheels[slot]['pivot']
        assert abs(car['radii'][i] - target_wheels[slot]['radius']) < 1e-7
    assert abs(car['half_width'] - (bounds[1][0] - bounds[0][0]) / 2 * 0.96) < 1e-7
    assert abs(car['half_length'] - (bounds[1][2] - bounds[0][2]) / 2 * 0.97) < 1e-7
    assert abs(car['roof_height'] - bounds[1][1]) < 1e-7
    return geometry


def apply_reference(car):
    """Postprocess the source-preserving edition; safe to repeat from its archive."""
    if car['id'] != 'ref-a08':
        return car
    import hashlib
    import json
    from prepare_fleet import ROOT, encode, parse
    path = ROOT / 'assets' / car['path']
    doc, binary = parse(path.read_bytes())
    receipt = json.loads(path.with_suffix('.json').read_text())
    source, _ = parse((ROOT.parent / receipt['source']).read_bytes())
    geometry, wheels, bounds = from_source(car, source, receipt['normalization'])
    for record in receipt['mesh_nodes']:
        wheel = record['wheel']
        world = multiply(transform_for(geometry, wheel), multiply(receipt['normalization'], record['source_world_matrix']))
        if wheel is not None:
            pivot = wheels[WHEELS[wheel]]['pivot']
            world[12:15] = [world[12 + i] - pivot[i] for i in range(3)]
        doc['nodes'][record['prepared_node']]['matrix'] = world
    for i, slot in enumerate(WHEELS):
        doc['nodes'][i + 1]['translation'] = wheels[slot]['pivot']
    result = encode(doc, binary)
    path.write_bytes(result)
    receipt.update(reference_geometry=geometry, prepared_sha256=hashlib.sha256(result).hexdigest())
    path.with_suffix('.json').write_text(json.dumps(receipt, indent=2) + '\n')
    car = deepcopy(car)
    car.update(wheels=[wheels[s]['pivot'] for s in WHEELS], radii=[wheels[s]['radius'] for s in WHEELS],
               half_width=(bounds[1][0] - bounds[0][0]) / 2 * 0.96,
               half_length=(bounds[1][2] - bounds[0][2]) / 2 * 0.97,
               body_center_z=(bounds[1][2] + bounds[0][2]) / 2, roof_height=bounds[1][1])
    return car
