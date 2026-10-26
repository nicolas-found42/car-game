"""Guard expansion identity, native-unit boundaries and complete batch coverage."""
import gzip
import io
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

from run_large_fleet import execute_stage

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / 'docs/verification/fleet-300'


class FakeProcess:
    def __init__(self, rows):
        self.stdout = io.StringIO(''.join(json.dumps(r) + '\n' for r in rows))

    def wait(self):
        return 0


class ExpansionTests(unittest.TestCase):
    def test_original_fleet_and_every_parameter_evidence_are_preserved(self):
        prepared = json.loads((OUT / 'prepared-physics.json').read_text())['cars']
        original = json.loads((OUT / 'original-physics.json').read_text())['cars']
        self.assertEqual(prepared[:57], original)
        self.assertEqual(len(prepared), 300)
        self.assertEqual(len({r['id'] for r in prepared}), 300)
        for car in prepared[57:]:
            self.assertEqual(car['parameters'].keys(), car['evidence'].keys())
            self.assertTrue(car['applicability'])
            for key in ['mass_kg','power_kw','torque_nm','power_rpm','torque_rpm','redline_rpm']:
                self.assertGreater(car['parameters'][key],0,(car['id'],key))
            for field in car['evidence'].values():
                self.assertIn(field['status'], ['estimate', 'game_proxy'])
                self.assertTrue(field['basis'])
                if field['status'] == 'game_proxy':
                    self.assertTrue(field['source'])

    def test_no_unresolved_extra_native_reductions_enter_simple_gearboxes(self):
        native = json.loads((OUT / 'native-specifications.json').read_text())
        for car in native.values():
            if car['game'] != 'gran-turismo-4':
                continue
            gear = car['fields']['GEAR']
            for field in ['gearflag', 'ExtraFinalGearRatio', 'ExtraFinalGearUsage']:
                self.assertEqual(gear[field], 0)

    def rows(self, count):
        return [{'car': 'test', 'case': i, 'repeats': 2,
                 'repeat_hashes': ['abc', 'abc'], 'failures': [[], []]} for i in range(count)]

    def test_batch_receipt_counts_actual_runs_and_rejects_missing_coverage(self):
        for count, expected in [(5000, True), (4999, False)]:
            with tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                with patch('run_large_fleet.subprocess.Popen', return_value=FakeProcess(self.rows(count))):
                    if expected:
                        receipt = execute_stage((root, root, root), 'test', 'stress')
                        self.assertEqual(receipt['executions'], 10000)
                        with gzip.open(receipt['result'], 'rt') as stream:
                            self.assertEqual(sum(1 for _ in stream), 5000)
                    else:
                        with self.assertRaises(AssertionError):
                            execute_stage((root, root, root), 'test', 'stress')
                        receipt = json.loads((root / 'test/stress.receipt.json').read_text())
                        self.assertFalse(receipt['complete_coverage'])
                        self.assertEqual(receipt['executions'], 9998)

    def test_revised_slope_sweep_covers_the_same_500_ids_for_every_car(self):
        rows=self.rows(500)
        for row in rows:
            row['case']+=4000
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory)
            with patch('run_large_fleet.subprocess.Popen',return_value=FakeProcess(rows)) as launch:
                receipt=execute_stage((root,root,root),'test','stress',(4000,4500))
                self.assertEqual(receipt['executions'],1000)
                self.assertEqual(launch.call_args.args[0][-4:],['--start','4000','--end','4500'])

    def test_native_zero_torque_placeholders_cannot_disable_a_powered_car(self):
        native=json.loads((OUT/'native-specifications.json').read_text())['exp-161']
        self.assertGreater(native['fields']['power_hp_game'],0)
        self.assertEqual(native['fields']['torque_kgm'],0)
        rows=json.loads((OUT/'prepared-physics.json').read_text())['cars']
        car=next(r for r in rows if r['id']=='exp-161');p=car['parameters']
        self.assertGreater(p['torque_nm'],0)
        self.assertGreater(p['torque_rpm'],0)
        self.assertGreaterEqual(p['torque_nm']+.001,p['power_kw']*9549.297/p['power_rpm'])
        self.assertEqual(car['evidence']['torque_nm']['status'],'estimate')
        self.assertEqual(car['evidence']['torque_rpm']['status'],'estimate')


if __name__ == '__main__':
    unittest.main()
