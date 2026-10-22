#!/usr/bin/env python3
"""Exercise inventory evidence at its command-line boundary."""
import json
from pathlib import Path
import subprocess
import sys
import unittest

ROOT = Path(__file__).resolve().parents[1]


class MechanicalInventoryTests(unittest.TestCase):
    def test_existing_350z_edition_is_identified_without_inventing_mechanics(self):
        result = subprocess.run(
            [sys.executable, str(ROOT / 'tools/inventory_mechanical_assets.py')],
            capture_output=True, text=True, check=True,
        )
        cars = json.loads(result.stdout)['cars']
        self.assertEqual(len(cars), 13)
        baseline = cars[0]
        self.assertEqual(baseline['id'], '350z')
        self.assertEqual(baseline['scene_triangles'], 9310)
        self.assertEqual(baseline['prepared_sha256'],
                         'e79630558ed16c54942afc552838fd465a73bf507754947342bd65a805d37874')
        self.assertIsNone(baseline['verified_fastener_coverage'])
        self.assertIsNone(baseline['verified_internal_part_coverage'])


if __name__ == '__main__':
    unittest.main()
