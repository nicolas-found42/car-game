"""Protocol and evidence guards independent of GPU and optional plotting packages."""
import copy
import os
import shutil
import json
import tempfile
import unittest
from pathlib import Path
from calibration_plan import prepare
from calibration_protocol import ALL_CASES, scenario
from calibration_io import digest, local, sha, write
from calibration_evidence import input_digest, physics_window
from analyze_calibration_study import check_matrix
from handling_metrics import near_stop, support, yaw_change


class CalibrationTests(unittest.TestCase):
    def test_protocol(self):
        self.assertEqual(len(ALL_CASES), 24)
        for case in ALL_CASES:
            value = scenario('350z', case)
            self.assertGreater(value['max_ticks'], max(a['tick'] for a in value['assertions']))
            self.assertEqual(value.get('course', 'city'), 'northeast_curve' if case.startswith('curve-') else 'city')
            self.assertFalse(value['required_capture'])

    def test_inactive_and_duplicate_selection_precedes_creation(self):
        with tempfile.TemporaryDirectory() as folder:
            destination = Path(folder) / 'study'
            for cars in [['staged-only'], ['350z', '350z']]:
                with self.assertRaises(ValueError):
                    prepare(destination, '/missing', cars, ['brake'], 2)
                self.assertFalse(destination.exists())

    def test_matrix_rejects_missing_and_duplicate(self):
        tasks = [{'car': '350z', 'case': 'brake', 'repeat': i} for i in range(2)]
        plan = {'cars': ['350z'], 'cases': ['brake'], 'repeats': 2, 'tasks': tasks, 'bindings': {},
                'cohort_sha256': digest([['350z'], ['brake'], 2])}
        for changed in [tasks[:1], [tasks[0], tasks[0]]]:
            with self.assertRaisesRegex(ValueError, 'cohort'):
                check_matrix(Path('/tmp'), plan, changed)

    def test_inputs_include_handbrake_release_and_reset(self):
        definition = {'commands': [{'tick': 1, 'command': {'action': 'drive', 'throttle': .3}},
                                   {'tick': 2, 'command': {'action': 'reset'}}]}
        held = {'throttle': .30000001, 'brake': 0., 'steer': 0., 'handbrake': False,
                'controller': 'scenario', 'reset': False}
        samples = [{'input': held}, {'input': {**held, 'reset': True}}]
        input_digest(samples, definition)
        bad = copy.deepcopy(samples)
        bad[0]['input']['throttle'] = .5
        with self.assertRaisesRegex(ValueError, 'input mismatch'):
            input_digest(bad, definition)

    def test_completed_player_ticks_include_reset_and_ignore_secondary(self):
        with tempfile.TemporaryDirectory() as folder:
            path = Path(folder)
            rows = [{'actor_id': actor, 'tick': tick, 'car': '350z'}
                    for tick in [11, 12, 13] for actor in ['player-vehicle', 'parked']]
            (path / 'physics.jsonl').write_text(''.join(json.dumps(r) + '\n' for r in rows))
            self.assertEqual(len(physics_window(path / 'physics.jsonl', 10, 3, '350z')), 3)
            with (path / 'physics.jsonl').open('a') as stream:
                stream.write(json.dumps(rows[0]) + '\n')
            with self.assertRaisesRegex(ValueError, 'duplicate'):
                physics_window(path / 'physics.jsonl', 10, 3, '350z')

    @unittest.skipUnless(os.environ.get('CALIBRATION_PRODUCTION_FIXTURE'), 'requires a recorded study fixture')
    def test_raw_mutation_cannot_change_accepted_metrics(self):
        from calibration_evidence import audit
        from calibration_io import load
        from analyze_calibration_study import metrics
        source = Path(os.environ['CALIBRATION_PRODUCTION_FIXTURE'])
        plan = load(source / 'study.json')
        task = load(source / 'runs.json')[0]
        with tempfile.TemporaryDirectory() as folder:
            study = Path(folder) / 'study'
            # Hardlink immutable inputs, but copy-before-write the mutable raw view.
            shutil.copytree(source, study, copy_function=os.link)
            before = metrics(audit(study, plan, task)[0])
            path = study / task['run'] / 'physics.jsonl'
            rows = [json.loads(line) for line in path.read_text().splitlines()]
            for row in rows:
                row['slip'] = 1.
            path.unlink()
            path.write_text(''.join(json.dumps(row) + '\n' for row in rows))
            after = metrics(audit(study, plan, task)[0])
            self.assertEqual(before, after)
            self.assertLess(after['peak_slip_deg'], 1.)

    def test_frozen_scenario_mutation_rejected(self):
        with tempfile.TemporaryDirectory() as folder:
            path = Path(folder)
            write(path / 'case.json', {'commands': []})
            task = {'car': '350z', 'case': 'brake', 'repeat': 0,
                    'scenario': 'case.json', 'scenario_sha256': sha(path / 'case.json')}
            plan = {'cars': ['350z'], 'cases': ['brake'], 'repeats': 1, 'tasks': [task],
                    'bindings': {}, 'cohort_sha256': digest([['350z'], ['brake'], 1])}
            check_matrix(path, plan, [task])
            write(path / 'case.json', {'commands': ['changed']})
            with self.assertRaisesRegex(ValueError, 'scenario input changed'):
                check_matrix(path, plan, [task])

    def test_escape_rejected(self):
        with self.assertRaises(ValueError):
            local('/tmp/study', '../elsewhere')

    def test_reset_teleport_is_not_driven_distance(self):
        from analyze_calibration_study import metrics
        samples = [self.sample(i) for i in range(300)]
        for i, row in enumerate(samples[250:], 250):
            row['position'][0] = i + 1000
            row['reset_count'] = 1
        self.assertEqual(metrics(samples)['path_m'], 59.)

    def test_action_support_cannot_replace_full_window(self):
        from analyze_calibration_study import measure_case
        samples = [self.sample(i) for i in range(840)]
        samples[250]['slip'] = 1.
        result = measure_case(samples, 'turn-left')
        self.assertEqual(result['support_ticks'], 601)
        self.assertEqual(result['action_support_ticks'], 240)
        self.assertGreater(result['peak_slip_deg'], 57.)
        self.assertEqual(result['action_peak_slip_deg'], 0.)

    @staticmethod
    def sample(index):
        return {'grounded': 4, 'wheel_loads': [100] * 4, 'rotation': [0, 0, 0, 1],
                'slip': 0., 'position': [index, 0., 0.], 'linear_velocity': [1., 0., 0.],
                'reset_count': 0}

    def test_stop_uses_first_stop_even_if_later_reverse(self):
        states = {i: {'position': [i, 0, 0], 'linear_velocity': [speed, 0, 0]}
                  for i, speed in enumerate([10, 5, .5, -8])}
        self.assertEqual(near_stop(states, 1, 4)['near_stop_tick'], 2)

    def test_contacts_and_loads_are_distinct(self):
        row = {'grounded': 4, 'wheel_loads': [0] * 4, 'rotation': [0, 0, 0, 1], 'slip': 0}
        result = support([row])
        self.assertEqual(result['four_contact_pct'], 100)
        self.assertEqual(result['four_loaded_pct'], 0)
        self.assertEqual(yaw_change([row, row]), 0)


if __name__ == '__main__':
    unittest.main()
