#!/usr/bin/env python3
"""Issue #86: the public verifier must reject incomplete or corrupt telemetry streams."""
import copy
import hashlib
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

VERIFY = Path(__file__).with_name('verify_run.py')
RUN_ID = 'integrity-control'
TICKS = 10
RESET_TICK = 5


def physics_record(tick):
    reset = tick == RESET_TICK
    held = {'step': tick - 1, 'tick': tick, 'controller': 'scenario', 'throttle': 1.0,
            'brake': 0.0, 'steer': 0.0, 'handbrake': False, 'reset': reset}
    return {'envelope': {'schema_version': 1, 'run_id': RUN_ID, 'actor_id': 'player-vehicle',
                         'tick': tick, 'boundary': 'completed_tick'},
            'tick': tick, 'step': tick - 1, 'time': tick / 60, 'actor_id': 'player-vehicle',
            'vehicle_id': 'pilot-350z', 'boundary': 'completed_tick', 'integration_steps': 1,
            'position': [0.0, 0.6, float(tick)], 'rotation': [0.0, 0.0, 0.0, 1.0],
            'linear_velocity': [0.0, 0.0, 1.0], 'angular_velocity': [0.0, 0.0, 0.0],
            'speed': 1.0, 'reset_count': 1 if tick >= RESET_TICK else 0, 'input': held,
            'evidence': {'outcome': 'reset' if reset else 'integrated', 'input': held}}


def event(kind, seq, tick, **fields):
    return {'schema_version': 1, 'run_id': RUN_ID, 'kind': kind, 'seq': seq, 'tick': tick,
            'actor_id': None, 'command_id': fields.pop('command_id', None), 'fields': fields}


def command(number, phase, tick):
    return {'schema_version': 1, 'run_id': RUN_ID, 'actor_id': 'player-vehicle', 'phase': phase,
            'command_id': f'{RUN_ID}:command:{number}', 'tick': tick,
            'command': {'action': 'assign_controller', 'controller': 'scenario'}}


def build_streams():
    seq = [0]

    def nxt():
        seq[0] += 1
        return seq[0]
    events = [event('run.start', nxt(), 0)]
    commands = [command(1, 'ingress', 0)]
    events.append(event('command.outcome', nxt(), 0, command_id=f'{RUN_ID}:command:1', disposition='applied'))
    for tick in range(1, TICKS + 1):
        events.append(event('controls.effective', nxt(), tick))
        if tick == RESET_TICK:
            events.append(event('vehicle.reset', nxt(), tick))
        events.append(event('simulation.tick_completed', nxt(), tick, vehicle_count=1))
        events.append(event('simulation.completed', nxt(), tick, reset_count=1 if tick >= RESET_TICK else 0))
    events.append(event('run.exit', nxt(), TICKS, code=0))
    physics = [physics_record(tick) for tick in range(1, TICKS + 1)]
    inputs = [physics_record(tick)['input'] for tick in range(1, TICKS + 1)]
    return {'events': events, 'commands': commands, 'physics': physics, 'input': inputs}


def write_lines(path, rows):
    path.write_text(''.join(json.dumps(row) + '\n' for row in rows))


def read_lines(path):
    return [json.loads(line) for line in path.read_text().splitlines()]


def fixture(root, mode='scenario'):
    run = Path(root)
    streams = build_streams()
    for name, rows in streams.items():
        write_lines(run / f'{name}.jsonl' if name != 'commands' else run / 'commands.jsonl', rows)
    (run / 'run.log').write_text('')
    manifest = {'schema': 1, 'run_id': RUN_ID, 'mode': mode, 'exit': {'code': 0, 'reason': 'success'},
                'ended_at': '2026-10-26T00:00:00Z', 'gpu': None,
                'recorder_health': {'schema': 1, 'required': True, 'path': 'recorder-health.json'}}
    (run / 'manifest.json').write_text(json.dumps(manifest))
    names = ('events', 'commands', 'physics', 'input', 'log')
    health_streams = {n: {'accepted': 0, 'written': 0, 'written_seq': 0, 'flushed_seq': 0, 'synced_seq': 0}
                      for n in names}
    health_streams['events'] = {'accepted': 2, 'written': 2, 'written_seq': 2, 'flushed_seq': 2, 'synced_seq': 2}
    health = {'schema': 1, 'run_id': RUN_ID, 'phase': 'finalized', 'failed': False, 'errors': [],
              'dropped': 0, 'losses': [], 'loss_ranges_truncated': False, 'produced': 2,
              'accepted': 2, 'written': 2, 'durable_seq': 2, 'queued_bytes': 0, 'queued_records': 0,
              'last_durable_tick': TICKS, 'completed_tick': TICKS, 'streams': health_streams}
    (run / 'recorder-health.json').write_text(json.dumps(health))
    return run


def verify(run):
    result = subprocess.run([sys.executable, str(VERIFY), '--json', str(run)], capture_output=True, text=True)
    return result, (json.loads(result.stdout) if result.stdout.strip().startswith('{') else {})


def mutate(run, name, change):
    path = run / f'{name}.jsonl'
    rows = read_lines(path)
    change(rows)
    write_lines(path, rows)


class TraceIntegrityTests(unittest.TestCase):
    def assertRejected(self, run, code, status='failed', file=None):
        result, report = verify(run)
        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertNotIn('RUN EVIDENCE PASS', result.stdout)
        self.assertEqual(result.stderr, '', result.stderr)
        self.assertFalse(report['ok'])
        self.assertEqual(report['status'], status, report)
        codes = [error['code'] for error in report['errors']]
        self.assertIn(code, codes, report)
        if file:
            self.assertIn(file, [error.get('file') for error in report['errors'] if error['code'] == code])
        return report

    def test_valid_control_passes_without_a_report_or_gpu(self):
        with tempfile.TemporaryDirectory() as root:
            result, report = verify(fixture(root, 'interactive'))
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            self.assertTrue(report['ok'])
            self.assertEqual(report['status'], 'verified')

    def test_empty_input_and_physics_traces_of_a_completed_run_fail(self):
        with tempfile.TemporaryDirectory() as root:
            run = fixture(root)
            (run / 'physics.jsonl').write_text('')
            (run / 'input.jsonl').write_text('')
            report = self.assertRejected(run, 'trace_empty', file='physics.jsonl')
            self.assertIn('input.jsonl', [e.get('file') for e in report['errors']])

    def test_dropped_tick_is_located(self):
        with tempfile.TemporaryDirectory() as root:
            run = fixture(root)
            mutate(run, 'physics', lambda rows: rows.pop(3))
            report = self.assertRejected(run, 'trace_tick_gap', file='physics.jsonl')
            gap = next(e for e in report['errors'] if e['code'] == 'trace_tick_gap')
            self.assertEqual((gap['tick'], gap['line'], gap['actor_id']), (5, 4, 'player-vehicle'))

    def test_duplicated_tick_is_rejected(self):
        with tempfile.TemporaryDirectory() as root:
            run = fixture(root)
            mutate(run, 'input', lambda rows: rows.insert(3, copy.deepcopy(rows[2])))
            self.assertRejected(run, 'trace_tick_duplicate', file='input.jsonl')

    def test_out_of_order_tick_is_rejected(self):
        with tempfile.TemporaryDirectory() as root:
            run = fixture(root)
            mutate(run, 'physics', lambda rows: rows.insert(2, rows.pop(3)))
            self.assertRejected(run, 'trace_tick_out_of_order', file='physics.jsonl')

    def test_truncated_final_tick_disagrees_with_recorder_health(self):
        with tempfile.TemporaryDirectory() as root:
            run = fixture(root)
            mutate(run, 'physics', lambda rows: rows.pop())
            self.assertRejected(run, 'trace_tick_coverage', file='physics.jsonl')

    def test_corrupt_tail_is_rejected_with_its_line(self):
        with tempfile.TemporaryDirectory() as root:
            run = fixture(root)
            path = run / 'physics.jsonl'
            path.write_text(path.read_text() + '{"tick": 11, "actor_i')
            report = self.assertRejected(run, 'trace_malformed_record', file='physics.jsonl')
            self.assertEqual(report['errors'][0]['line'], TICKS + 1)

    def test_invalid_utf8_is_rejected_without_a_traceback(self):
        with tempfile.TemporaryDirectory() as root:
            run = fixture(root)
            with (run / 'input.jsonl').open('ab') as handle:
                handle.write(b'\xff\xfe\n')
            self.assertRejected(run, 'trace_malformed_record', file='input.jsonl')

    def test_non_finite_and_wrongly_typed_values_are_rejected(self):
        for field, value in (('position', [0.0, float('nan'), 1.0]), ('speed', '1.0'),
                             ('linear_velocity', [0.0, 1.0])):
            with self.subTest(field=field), tempfile.TemporaryDirectory() as root:
                run = fixture(root)
                mutate(run, 'physics', lambda rows: rows[2].__setitem__(field, value))
                report = self.assertRejected(run, 'trace_invalid_value', file='physics.jsonl')
                self.assertEqual(report['errors'][0]['tick'], 3)

    def test_missing_reset_evidence_is_rejected(self):
        with tempfile.TemporaryDirectory() as root:
            run = fixture(root)
            mutate(run, 'physics', lambda rows: rows[RESET_TICK - 1]['evidence'].__setitem__('outcome', 'integrated'))
            self.assertRejected(run, 'trace_reset_mismatch', file='physics.jsonl')

    def test_reset_without_event_or_counter_transition_is_rejected(self):
        with tempfile.TemporaryDirectory() as root:
            run = fixture(root)
            mutate(run, 'events', lambda rows: rows.__delitem__(
                next(i for i, row in enumerate(rows) if row['kind'] == 'vehicle.reset')))
            self.assertRejected(run, 'trace_reset_mismatch', file='events.jsonl')
        with tempfile.TemporaryDirectory() as root:
            run = fixture(root)
            mutate(run, 'physics', lambda rows: rows[RESET_TICK - 1].__setitem__('reset_count', 0))
            self.assertRejected(run, 'trace_reset_mismatch', file='physics.jsonl')

    def test_effective_input_must_match_completed_state_input(self):
        with tempfile.TemporaryDirectory() as root:
            run = fixture(root)
            mutate(run, 'input', lambda rows: rows[2].__setitem__('throttle', 0.25))
            self.assertRejected(run, 'trace_input_mismatch', file='input.jsonl')

    def test_duplicate_command_and_missing_outcome_are_rejected(self):
        with tempfile.TemporaryDirectory() as root:
            run = fixture(root)
            mutate(run, 'commands', lambda rows: rows.append(copy.deepcopy(rows[0])))
            self.assertRejected(run, 'trace_command_duplicate', file='commands.jsonl')
        with tempfile.TemporaryDirectory() as root:
            run = fixture(root)
            mutate(run, 'events', lambda rows: rows.__delitem__(1))
            self.assertRejected(run, 'trace_command_unresolved', file='commands.jsonl')

    def test_event_sequence_and_tick_completion_must_be_continuous(self):
        with tempfile.TemporaryDirectory() as root:
            run = fixture(root)
            mutate(run, 'events', lambda rows: rows.__delitem__(
                next(i for i, row in enumerate(rows) if row['kind'] == 'simulation.tick_completed' and row['tick'] == 4)))
            self.assertRejected(run, 'trace_event_incomplete', file='events.jsonl')
        with tempfile.TemporaryDirectory() as root:
            run = fixture(root)
            mutate(run, 'events', lambda rows: rows[4].__setitem__('seq', 1))
            self.assertRejected(run, 'trace_event_sequence', file='events.jsonl')

    def test_capture_hash_mismatch_is_rejected(self):
        with tempfile.TemporaryDirectory() as root:
            run = fixture(root, 'smoke')
            manifest = json.loads((run / 'manifest.json').read_text())
            manifest['gpu'] = {'backend': 'fixture'}
            (run / 'manifest.json').write_text(json.dumps(manifest))
            (run / 'captures').mkdir()
            (run / 'captures/front.png').write_bytes(b'fixture')
            good = hashlib.sha256(b'fixture').hexdigest()
            (run / 'report.json').write_text(json.dumps([{'id': '350z', 'captures': {
                'front': {'sha256': good, 'path': 'captures/front.png'}}}]))
            result, _ = verify(run)
            self.assertEqual(result.returncode, 0, result.stdout)
            (run / 'captures/front.png').write_bytes(b'tampered')
            self.assertRejected(run, 'capture_hash_mismatch')

    def test_unknown_record_schema_is_unsupported_not_failed_or_verified(self):
        with tempfile.TemporaryDirectory() as root:
            run = fixture(root)
            mutate(run, 'physics', lambda rows: rows[0]['envelope'].__setitem__('schema_version', 99))
            self.assertRejected(run, 'trace_schema_unsupported', status='unsupported', file='physics.jsonl')

    def test_zero_tick_startup_failure_is_never_a_verified_simulation(self):
        with tempfile.TemporaryDirectory() as root:
            run = fixture(root)
            health = json.loads((run / 'recorder-health.json').read_text())
            health['last_durable_tick'] = health['completed_tick'] = 0
            (run / 'recorder-health.json').write_text(json.dumps(health))
            for name in ('physics', 'input'):
                (run / f'{name}.jsonl').write_text('')
            manifest = json.loads((run / 'manifest.json').read_text())
            manifest['exit'] = {'code': 1, 'reason': 'startup_failure'}
            (run / 'manifest.json').write_text(json.dumps(manifest))
            result, report = verify(run)
            self.assertNotEqual(result.returncode, 0)
            self.assertNotEqual(report['status'], 'verified')
            self.assertIn('simulation_not_started', [e['code'] for e in report['errors']])


if __name__ == '__main__':
    unittest.main()
