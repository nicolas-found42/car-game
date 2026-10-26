"""Boundary controls for the handling analysis, including reverse and reset gaps."""
import math
import copy

from compare_handling_studies import check_matching
import unittest

from handling_metrics import clearance, near_stop, rotate, support, yaw_change
from handling_protocol import CASES, scenario


def state(x, speed):
    return {'position': [x, 0, 0], 'linear_velocity': [speed, 0, 0]}


class HandlingMetricsTests(unittest.TestCase):
    def test_near_stop_uses_pre_command_position_and_ignores_later_reverse(self):
        states = {9: state(0, 5), 10: state(.1, 4), 11: state(.2, .9),
                  12: state(.1, -4)}
        result = near_stop(states, 10, 13)
        self.assertAlmostEqual(result['near_stop_m'], .2)
        self.assertAlmostEqual(result['near_stop_s'], 2 / 120)
        self.assertEqual(result['near_stop_tick'], 11)

    def test_not_stopped_is_missing_rather_than_zero(self):
        result = near_stop({9: state(0, 5), 10: state(.1, 4)}, 10, 11)
        self.assertIsNone(result['near_stop_s'])
        self.assertIsNone(result['near_stop_m'])

    def test_missing_completed_tick_cannot_shorten_distance(self):
        with self.assertRaises(KeyError):
            near_stop({9: state(0, 5), 11: state(.2, .9)}, 10, 12)

    def test_yaw_unwraps_at_pi(self):
        def sample(angle):
            return {'rotation': [0, -math.sin(angle / 2), 0, math.cos(angle / 2)]}
        self.assertAlmostEqual(yaw_change([sample(math.radians(179)),
                                          sample(math.radians(181))]), 2)
        self.assertAlmostEqual(rotate([0, 0, 0, 1], [0, 1, 0])[1], 1)

    def test_contacts_do_not_imply_loaded_wheels(self):
        row = {'rotation': [0, 0, 0, 1], 'grounded': 4,
               'wheel_loads': [0, 100, 100, 100], 'slip': 0}
        result = support([row])
        self.assertEqual(result['four_contact_pct'], 100)
        self.assertEqual(result['four_loaded_pct'], 0)
        with self.assertRaises(ValueError):
            support([])

    def test_collision_clearance_includes_body_extent(self):
        car = {'body_center_z': 0, 'half_width': 1, 'half_length': 2, 'roof_height': 1}
        row = {'position': [0, .6, 478], 'rotation': [0, 0, 0, 1]}
        self.assertLess(clearance([row], car), 0)

    def test_protocol_changes_only_identity_across_cars(self):
        for case in CASES:
            first, second = scenario('350z', case), scenario('gt-rx7', case)
            for item in [first, second]:
                item.pop('name')
                item['initial'].pop('car')
                item['assertions'][0].pop('car')
            self.assertEqual(first, second)

    def test_version_comparison_rejects_changed_inputs_and_asset_cohorts(self):
        rows = [{'car': str(car), 'case': str(case), 'repeat': repeat,
                 'matched_input_sha256': 'input', 'asset_sha256': {'asset': 'hash'},
                 'scenario_sha256': 'scenario'}
                for car in range(13) for case in range(5) for repeat in [1, 2]]
        before = {'runs': rows}
        config = dict.fromkeys(['cars', 'cases', 'backend', 'fixed_hz', 'seed',
                               'roster_sha256', 'repeats'], 'same')
        old_config, new_config = {**config, 'binary_sha256': 'before'}, {
            **config, 'binary_sha256': 'after'}
        check_matching(before, before, old_config, new_config)
        for field in ['matched_input_sha256', 'asset_sha256', 'scenario_sha256']:
            after = copy.deepcopy(before)
            after['runs'][0][field] = 'changed'
            with self.assertRaises(AssertionError):
                check_matching(before, after, old_config, new_config)


if __name__ == '__main__':
    unittest.main()
