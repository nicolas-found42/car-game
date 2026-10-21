#!/usr/bin/env python3
"""Real corruption controls for fleet source-preservation and transform checks."""
from contextlib import redirect_stdout
from io import StringIO
import json
from pathlib import Path
import shutil
import tempfile
import unittest

import check_assets
from prepare_fleet import accessor_values, encode, parse


class IntegrityControls(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.original_root = check_assets.ROOT
        root = Path(self.temp.name) / 'grid'
        shutil.copytree(self.original_root / 'assets/cars', root / 'assets/cars')
        (Path(self.temp.name) / 'carmodels').symlink_to(self.original_root.parent / 'carmodels', target_is_directory=True)
        check_assets.ROOT = root

    def tearDown(self):
        check_assets.ROOT = self.original_root
        self.temp.cleanup()

    def run_check(self):
        with redirect_stdout(StringIO()):
            check_assets.check_fleet()

    def mutate(self, edit):
        path = check_assets.ROOT / 'assets/cars/fr2-cobra.glb'
        doc, binary = parse(path.read_bytes())
        edit(doc)
        data = encode(doc, binary)
        path.write_bytes(data)
        # Update the outer hash: deeper source comparisons must still catch the mutation.
        import hashlib
        receipt = json.loads(path.with_suffix('.json').read_text())
        receipt['prepared_sha256'] = hashlib.sha256(data).hexdigest()
        path.with_suffix('.json').write_text(json.dumps(receipt))

    def test_untouched_roster_passes(self):
        self.run_check()

    def test_wrong_wheel_pose_fails_even_with_updated_hash(self):
        self.mutate(lambda d: d['nodes'][1]['translation'].__setitem__(0, -1.4))
        with self.assertRaises(AssertionError):
            self.run_check()

    def test_missing_texture_fails_even_with_updated_hash(self):
        def remove_texture(d):
            for m in d['materials']:
                pbr = m.get('pbrMetallicRoughness', {})
                if 'baseColorTexture' in pbr:
                    pbr.pop('baseColorTexture')
                    return
        self.mutate(remove_texture)
        with self.assertRaises(AssertionError):
            self.run_check()

    def test_changed_geometry_accessor_fails_even_with_updated_hash(self):
        self.mutate(lambda d: d['accessors'][0].__setitem__('count', 1))
        with self.assertRaises(AssertionError):
            self.run_check()

    def test_shared_importer_and_receipt_placement_error_fails(self):
        self.mutate(lambda d: d['nodes'][5]['matrix'].__setitem__(12, d['nodes'][5]['matrix'][12] - 0.3))
        receipt_path = check_assets.ROOT / 'assets/cars/fr2-cobra.json'
        receipt = json.loads(receipt_path.read_text())
        receipt['mesh_nodes'][0]['source_world_matrix'][12] += 0.3
        receipt_path.write_text(json.dumps(receipt))
        with self.assertRaises(AssertionError):
            self.run_check()

    def test_duplicate_partition_triangle_fails_with_valid_outer_hash(self):
        import hashlib
        path = check_assets.ROOT / 'assets/cars/redline-gt500.glb'
        doc, binary = parse(path.read_bytes())
        binary = bytearray(binary)
        start = doc['bufferViews'][-1]['byteOffset']
        assert binary[start:start+12] != binary[start+12:start+24]
        binary[start:start+12] = binary[start+12:start+24]
        data = encode(doc, bytes(binary))
        path.write_bytes(data)
        receipt = json.loads(path.with_suffix('.json').read_text())
        receipt['prepared_sha256'] = hashlib.sha256(data).hexdigest()
        path.with_suffix('.json').write_text(json.dumps(receipt))
        with self.assertRaisesRegex(AssertionError, 'source triangles'):
            self.run_check()

    def test_gt500_complete_rear_window_uses_one_cabin_material(self):
        path = check_assets.ROOT / 'assets/cars/redline-gt500.glb'
        doc, binary = parse(path.read_bytes())
        receipt = json.loads(path.with_suffix('.json').read_text())
        source, source_binary = parse((check_assets.ROOT.parent / receipt['source']).read_bytes())
        body = next(n for n in source['nodes'] if n.get('name') == 'BODY')
        primitive = source['meshes'][body['mesh']]['primitives'][1]
        indices = [v[0] for v in accessor_values(source, source_binary, primitive['indices'])]
        triangles = [tuple(indices[i:i+3]) for i in range(0, len(indices), 3)]
        positions = accessor_values(source, source_binary, primitive['attributes']['POSITION'])
        points = [{tuple(round(c, 6) for c in positions[v]) for v in tri} for tri in triangles]
        # Source triangle 2 is the lower rear-window edge. Recover its complete
        # position-connected surface, independently of the preparation classifier.
        component = {2}
        while True:
            vertices = set().union(*(points[i] for i in component))
            grown = {i for i, p in enumerate(points) if p & vertices}
            if grown == component:
                break
            component = grown
        self.assertEqual(len(component), 26)
        prepared_body = next(n for n in doc['nodes'] if n.get('name') == 'source/BODY')
        cabin = set()
        for p in doc['meshes'][prepared_body['mesh']]['primitives']:
            if doc['materials'][p['material']].get('extras', {}).get('gridGlassRole') == 'cabin':
                cabin.update(check_assets.triangles(doc, binary, p['indices']))
        self.assertTrue({triangles[i] for i in component} <= cabin,
                        'The entire rear window must use the cabin material')


if __name__ == '__main__':
    unittest.main()
