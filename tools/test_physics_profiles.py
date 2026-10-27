"""Profile evidence must not validate against substituted runtime tuning."""
import json
from pathlib import Path
import tempfile
import unittest

from provenance_legacy import store_bytes
from provenance_material import verify_runtime
from provenance_schema import BundleError
from test_provenance_fixtures import complete_runtime_index


class PhysicsBindingTests(unittest.TestCase):
    def test_embedded_profiles_require_equal_frozen_runtime_parameters(self):
        with tempfile.TemporaryDirectory() as folder:
            run = Path(folder)
            index = complete_runtime_index(run)
            data = b'{"mass_kg":1160}'
            embedded = store_bytes(run, data, 'embedded_physics_profile', 'catalog/physics.json')
            runtime = store_bytes(run, data, 'receipt_or_catalog', 'assets/cars/physics.json')
            index['inventory'] += [embedded, runtime]
            verify_runtime(run, index)
            index['inventory'].remove(runtime)
            with self.assertRaises(BundleError):
                verify_runtime(run, index)
            substituted = store_bytes(run, b'{"mass_kg":1320}', 'receipt_or_catalog', 'assets/cars/physics.json')
            index['inventory'].append(substituted)
            with self.assertRaises(BundleError):
                verify_runtime(run, index)

    def test_all_fleet_parameters_have_explicit_evidence_and_qualified_proxies(self):
        root = Path(__file__).resolve().parents[1]
        roster = json.loads((root / 'assets/cars/roster.json').read_text())
        profiles = json.loads((root / 'assets/cars/physics.json').read_text())['cars']
        self.assertTrue({p['id'] for p in roster} <= {p['id'] for p in profiles})
        self.assertEqual(len(profiles), len({p['id'] for p in profiles}))
        self.assertEqual(len(profiles), 300)
        for car in profiles:
            self.assertEqual(car['parameters'].keys(), car['evidence'].keys())
            self.assertTrue(car['applicability'])
            for fact in car['evidence'].values():
                self.assertIn(fact['status'], ['reference', 'estimate', 'game_proxy', 'related_reference'])
                self.assertTrue(fact['basis'])
                if fact['status'] != 'estimate':
                    self.assertTrue(fact['source'])


if __name__ == '__main__':
    unittest.main()
