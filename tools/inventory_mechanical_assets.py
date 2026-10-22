#!/usr/bin/env python3
"""Print receipt-linked render-asset evidence; unknown mechanics stay unknown."""
import argparse
import hashlib
import json
from pathlib import Path
import struct

ROOT = Path(__file__).resolve().parents[1]


def sha256(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def glb(path):
    data = path.read_bytes()
    magic, version, size, length = struct.unpack_from('<4sIII', data)
    assert (magic, version, size) == (b'glTF', 2, len(data)), path
    return json.loads(data[20:20 + length])


def inspect_car(car, source_root):
    path = ROOT / 'assets' / car['path']
    receipt_path = path.with_suffix('.json')
    receipt = json.loads(receipt_path.read_text())
    source_path = source_root / Path(receipt['source']).relative_to('carmodels')
    assert sha256(source_path) == receipt['source_sha256'], source_path
    assert sha256(path) == receipt['prepared_sha256'], path
    prepared, source = glb(path), glb(source_path)
    primitives = [p for mesh in prepared['meshes'] for p in mesh['primitives']]
    return {
        'id': car['id'], 'source_game': car['game'],
        'source_path': receipt['source'], 'source_sha256': sha256(source_path),
        'prepared_path': car['path'], 'prepared_sha256': sha256(path),
        'receipt_sha256': sha256(receipt_path), 'source_hash_verified': True,
        'source_node_names': [n.get('name', '') for n in source['nodes']],
        'prepared_node_names': [n.get('name', '') for n in prepared['nodes']],
        'mesh_count': len(prepared['meshes']), 'primitive_count': len(primitives),
        'uv0_primitives': sum('TEXCOORD_0' in p['attributes'] for p in primitives),
        'scene_triangles': car['triangles'],
        'wheel_pivot_positions': car['wheels'], 'wheel_radii': car['radii'],
        'animated_visual_node_count': car['wheel_nodes'],
        'collider': {
            'classification': 'authored arcade fixture; generated compound cuboids, not source mechanical colliders',
            **{key: car[key] for key in ('half_width', 'half_length', 'roof_height', 'body_center_z')},
        },
        'mechanical_topology': 'missing authored operating/attachment manifest; model names do not establish topology',
        'verified_internal_part_coverage': None, 'verified_fastener_coverage': None,
        'uv_limit': 'TEXCOORD_0 presence only; no per-panel decal or deformation UV acceptance established',
        'provenance_limit': 'local source and edition hashes verify derivation; no legal-license or stock-calibration claim',
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source-root', type=Path, default=ROOT.parent / 'carmodels')
    args = parser.parse_args()
    roster = json.loads((ROOT / 'assets/cars/roster.json').read_text())
    cars = [inspect_car(car, args.source_root) for car in roster]
    print(json.dumps({
        'schema': 1,
        'inventory_scope': 'All 13 selected prepared GLBs and exact receipt-linked source GLBs. Does not assert that unselected content in the sibling collection has no useful mechanical assets.',
        'classification': 'verified source render geometry/textures/pivots plus game-authored collider/arcade tuning fixtures; complete mechanical content remains unauthored/unverified',
        'cars': cars,
    }, indent=2))


if __name__ == '__main__':
    main()
