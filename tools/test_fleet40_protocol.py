"""Expanded study must retain meaningful scenarios and unchanged evidence bytes."""
import hashlib
import json
from pathlib import Path
import tempfile
import unittest

from fleet40_protocol import ALL_CASES, ROOT, ROUTES, scenario
from run_fleet40_study import compact


class Fleet40Tests(unittest.TestCase):
    def test_full_roster_and_scenarios_have_outcomes(self):
        roster=json.loads((ROOT/'assets/cars/roster.json').read_text())
        plan=json.loads((ROOT/'docs/verification/fleet-57/selection.json').read_text())
        self.assertGreaterEqual(len(roster),plan['target_fleet_size'])
        self.assertTrue({r['id'] for r in plan['cars']} <= {r['id'] for r in roster})
        self.assertEqual(len(ALL_CASES),24)
        for car in roster:
            for case in ALL_CASES:
                value=scenario(car['id'],case)
                self.assertTrue(value['assertions'])
                self.assertEqual(value['initial']['car'],car['id'])
                if case in ROUTES:
                    self.assertIn('route_completed',{a['kind'] for a in value['assertions']})
                    self.assertEqual(value['initial']['position'][::2],value['route']['points'][0])

    def test_shared_objects_keep_each_bundle_hash_and_path(self):
        with tempfile.TemporaryDirectory() as folder:
            root=Path(folder);data=b'compiler-bound immutable material'
            digest=hashlib.sha256(data).hexdigest()
            objects=[]
            for tag in ['first','second']:
                obj=root/tag/'objects/sha256'/digest
                obj.parent.mkdir(parents=True);obj.write_bytes(data);objects.append(obj)
                compact(root/tag,root/'shared')
            self.assertEqual(objects[0].stat().st_ino,objects[1].stat().st_ino)
            self.assertTrue(all(hashlib.sha256(p.read_bytes()).hexdigest()==digest for p in objects))

    def test_corrupt_object_is_rejected_before_deduplication(self):
        with tempfile.TemporaryDirectory() as folder:
            root=Path(folder);obj=root/'run/objects/sha256'/('a'*64)
            obj.parent.mkdir(parents=True);obj.write_bytes(b'wrong content')
            with self.assertRaises(AssertionError):compact(root/'run',root/'shared')
            self.assertEqual(obj.read_bytes(),b'wrong content')


if __name__=='__main__':unittest.main()
