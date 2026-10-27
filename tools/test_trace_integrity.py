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
                         'tick': tick, 'render_frame': tick, 'boundary': 'completed_tick'},
            'tick': tick, 'step': tick - 1, 'time': tick / 120, 'actor_id': 'player-vehicle',
            'vehicle_id': 'pilot-350z', 'body_id': 'pilot-350z:chassis', 'car': '350z',
            'query_mode': 'synchronized', 'gear': 1, 'grounded': 4, 'ray_misses': 0,
            'wheel_lengths': [0.3] * 4, 'wheel_loads': [3200.] * 4,
            'slip': 0., 'rpm': 900., 'steering': 0., 'boundary': 'completed_tick', 'integration_steps': 1,
            'position': [0.0, 0.6, float(tick)], 'rotation': [0.0, 0.0, 0.0, 1.0],
            'linear_velocity': [0.0, 0.0, 1.0], 'angular_velocity': [0.0, 0.0, 0.0],
            'speed': 1.0, 'reset_count': 1 if tick >= RESET_TICK else 0, 'input': held,
            'evidence': {'query_tick': tick, 'wheels': [
                {'origin': [0., 0., 0.], 'direction': [0., -1., 0.], 'point': None, 'normal': None}
                for _ in range(4)], 'outcome': 'reset' if reset else 'integrated', 'input': held}}


def event(kind, seq, tick, **fields):
    return {'schema_version': 1, 'run_id': RUN_ID, 'kind': kind, 'seq': seq, 'tick': tick,
            'render_frame': tick, 'actor_id': 'player-vehicle' if kind == 'command.outcome' else None, 'command_id': fields.pop('command_id', None), 'fields': fields}


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
    events.append(event('command.outcome', nxt(), 0, command_id=f'{RUN_ID}:command:1', disposition='applied', command=commands[0]['command']))
    for tick in range(1, TICKS + 1):
        controls = physics_record(tick)['input']
        control = event('controls.effective', nxt(), tick, **{k: v for k, v in controls.items() if k not in ('step', 'tick')})
        control['actor_id'] = 'player-vehicle'
        events.append(control)
        if tick == RESET_TICK:
            reset_event = event('vehicle.reset', nxt(), tick)
            reset_event['actor_id'] = 'player-vehicle'
            events.append(reset_event)
        events.append(event('simulation.tick_completed', nxt(), tick, vehicle_count=1))
        sample = physics_record(tick)
        completed = event('simulation.completed', nxt(), tick, **{
            key: sample[key] for key in ('reset_count', 'position', 'linear_velocity', 'vehicle_id')})
        completed['actor_id'] = 'player-vehicle'
        events.append(completed)
    events.append(event('run.exit', nxt(), TICKS, code=0))
    physics = [physics_record(tick) for tick in range(1, TICKS + 1)]
    inputs = [physics_record(tick)['input'] for tick in range(1, TICKS + 1)]
    return {'events': events, 'commands': commands, 'physics': physics, 'input': inputs}


def write_lines(path, rows):
    path.write_text(''.join(json.dumps(row) + '\n' for row in rows))


def read_lines(path):
    return [json.loads(line) for line in path.read_text().splitlines()]


def fixture(root, mode='interactive'):
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
    for name, rows in streams.items():
        health_streams[name].update(accepted=len(rows), written=len(rows))
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
            run = fixture(root)
            self.add_captures(run)
            result, _ = verify(run)
            self.assertEqual(result.returncode, 0, result.stdout)
            (run / 'captures/front.png').write_bytes(b'tampered')
            self.assertRejected(run, 'capture_hash_mismatch')

    def add_captures(self, run):
        (run / 'captures').mkdir()
        events = read_lines(run / 'events.jsonl')
        captures = {}
        for view in ('front', 'side', 'rear', 'driving'):
            path = f'captures/{view}.png'
            (run / path).write_bytes(b'fixture')
            metrics = {'nonblank': True}
            captures[view] = {'sha256': hashlib.sha256(b'fixture').hexdigest(),
                              'path': path, 'metrics': metrics}
            for kind in ('screenshot.requested', 'screenshot.validated'):
                row = event(kind, len(events) + 1, TICKS, car='350z', view=view,
                            path=path, metrics=metrics)
                row['actor_id'] = 'player-vehicle'
                events.append(row)
        write_lines(run / 'events.jsonl', events)
        (run / 'report.json').write_text(json.dumps([{'id': '350z', 'passed': True, 'captures': captures}]))
        health = json.loads((run / 'recorder-health.json').read_text())
        health['streams']['events'].update(accepted=len(events), written=len(events))
        (run / 'recorder-health.json').write_text(json.dumps(health))

    def test_declared_actor_cannot_be_entirely_absent(self):
        with tempfile.TemporaryDirectory() as root:
            run = fixture(root)
            mutate(run, 'events', lambda rows: [r['fields'].__setitem__('vehicle_count', 2)
                   for r in rows if r['kind'] == 'simulation.tick_completed'])
            self.assertRejected(run, 'trace_actor_coverage')

    def test_unrelated_event_cannot_resolve_a_command(self):
        with tempfile.TemporaryDirectory() as root:
            run = fixture(root)
            mutate(run, 'events', lambda rows: [r.__setitem__('kind', 'unrelated.event')
                   for r in rows if r['kind'] == 'command.outcome'])
            self.assertRejected(run, 'trace_command_unresolved')

    def test_outcome_actor_action_and_order_must_match(self):
        for change, code in ((lambda r: r.__setitem__('actor_id', 'other'), 'trace_command_mismatch'),
                             (lambda r: r['fields'].__setitem__('command', {'action': 'reset'}), 'trace_command_mismatch'),
                             (lambda r: r.__setitem__('tick', -1), 'trace_invalid_value')):
            with self.subTest(change=change), tempfile.TemporaryDirectory() as root:
                run = fixture(root)
                mutate(run, 'events', lambda rows: change(rows[1]))
                self.assertRejected(run, code)

    def test_nested_nonfinite_and_wrong_completed_identity_fail(self):
        for change in (lambda r: r['evidence'].__setitem__('wheel_loads', [float('nan')]),
                       lambda r: r['envelope'].__setitem__('actor_id', 'other'),
                       lambda r: r['envelope'].__setitem__('boundary', 'pre_step')):
            with self.subTest(change=change), tempfile.TemporaryDirectory() as root:
                run = fixture(root)
                mutate(run, 'physics', lambda rows: change(rows[0]))
                self.assertRejected(run, 'trace_invalid_value')

    def test_capture_frame_or_actor_mismatch_fails(self):
        for change in (lambda r: r.__setitem__('actor_id', 'other'),
                       lambda r: r.__setitem__('render_frame', -1)):
            with self.subTest(change=change), tempfile.TemporaryDirectory() as root:
                run = fixture(root)
                self.add_captures(run)
                mutate(run, 'events', lambda rows: change(rows[-1]))
                self.assertRejected(run, 'capture_link_mismatch')

    def test_completed_state_and_effective_controls_are_correlated(self):
        for kind, field, value, code in (
                ('simulation.completed', 'position', [99, 0, 0], 'trace_state_mismatch'),
                ('controls.effective', 'throttle', 0.25, 'trace_control_mismatch')):
            with self.subTest(kind=kind), tempfile.TemporaryDirectory() as root:
                run = fixture(root)
                mutate(run, 'events', lambda rows: next(r for r in rows if r['kind'] == kind)['fields'].__setitem__(field, value))
                self.assertRejected(run, code)

    def test_reset_duplicates_and_wrong_actors_fail(self):
        with tempfile.TemporaryDirectory() as root:
            run = fixture(root)
            mutate(run, 'events', lambda rows: next(r for r in rows if r['kind'] == 'vehicle.reset').__setitem__('actor_id', 'other'))
            self.assertRejected(run, 'trace_reset_mismatch')

    def test_scenario_assertions_and_capture_must_be_complete(self):
        with tempfile.TemporaryDirectory() as root:
            run = fixture(root, 'scenario')
            (run / 'scenario.json').write_text(json.dumps({'definition': {
                'assertions': [{'kind': 'speed_at_least', 'tick': 1, 'mps': 1}], 'required_capture': True}}))
            (run / 'scenario-summary.json').write_text(json.dumps({
                'passed': True, 'assertions_declared': 1, 'assertions_executed': 0,
                'assertions_passed': 0, 'results': [], 'capture': None}))
            self.assertRejected(run, 'scenario_incomplete')
            self.assertRejected(run, 'capture_link_missing')

    def test_valid_secondary_actor_can_start_later(self):
        with tempfile.TemporaryDirectory() as root:
            run = fixture(root)
            physics = read_lines(run / 'physics.jsonl')
            events = read_lines(run / 'events.jsonl')
            extra = []
            for tick in range(4, TICKS + 1):
                sample = physics_record(tick)
                sample['actor_id'] = sample['envelope']['actor_id'] = 'secondary'
                sample['vehicle_id'] = 'secondary'
                extra.append(sample)
                complete = event('simulation.completed', 0, tick, **{
                    key: sample[key] for key in ('position', 'linear_velocity', 'reset_count', 'vehicle_id')})
                complete['actor_id'] = 'secondary'
                controls = event('controls.effective', 0, tick, **{
                    k: v for k, v in sample['input'].items() if k not in ('tick', 'step')})
                controls['actor_id'] = 'secondary'
                events.extend([controls, complete])
                if tick == RESET_TICK:
                    reset = event('vehicle.reset', 0, tick)
                    reset['actor_id'] = 'secondary'
                    events.append(reset)
            for row in events:
                if row['kind'] == 'simulation.tick_completed' and row['tick'] >= 4:
                    row['fields']['vehicle_count'] = 2
            events.sort(key=lambda r: r['tick'])
            for seq, row in enumerate(events, 1):
                row['seq'] = seq
            write_lines(run / 'physics.jsonl', sorted(physics + extra, key=lambda r: r['tick']))
            write_lines(run / 'events.jsonl', events)
            health = json.loads((run / 'recorder-health.json').read_text())
            health['streams']['physics'].update(accepted=len(physics + extra), written=len(physics + extra))
            health['streams']['events'].update(accepted=len(events), written=len(events))
            (run / 'recorder-health.json').write_text(json.dumps(health))
            result, _ = verify(run)
            self.assertEqual(result.returncode, 0, result.stdout)

    def test_scenario_capture_hash_and_frame_links(self):
        with tempfile.TemporaryDirectory() as root:
            run = fixture(root, 'scenario')
            assertion = {'kind': 'speed_at_least', 'tick': 1, 'mps': 0}
            (run / 'scenario.json').write_text(json.dumps({'definition': {
                'assertions': [assertion], 'required_capture': True}}))
            (run / 'captures').mkdir()
            (run / 'captures/scenario.png').write_bytes(b'fixture')
            capture = {'path': 'captures/scenario.png', 'metrics': {'nonblank': True},
                       'sha256': hashlib.sha256(b'fixture').hexdigest()}
            summary = {'passed': True, 'assertions_declared': 1, 'assertions_executed': 1,
                       'assertions_passed': 1, 'completed_tick': TICKS,
                       'results': [{'assertion': assertion, 'passed': True}], 'capture': capture}
            (run / 'scenario-summary.json').write_text(json.dumps(summary))
            events = read_lines(run / 'events.jsonl')
            for kind in ('scenario.capture_requested', 'scenario.capture_validated'):
                row = event(kind, len(events) + 1, TICKS, path=capture['path'], metrics=capture['metrics'])
                row['actor_id'] = 'player-vehicle'
                events.append(row)
            events.append(event('scenario.finished', len(events) + 1, TICKS, **summary))
            write_lines(run / 'events.jsonl', events)
            health = json.loads((run / 'recorder-health.json').read_text())
            health['streams']['events'].update(accepted=len(events), written=len(events))
            (run / 'recorder-health.json').write_text(json.dumps(health))
            result, _ = verify(run)
            self.assertEqual(result.returncode, 0, result.stdout)
            (run / 'captures/scenario.png').write_bytes(b'corrupt')
            self.assertRejected(run, 'capture_hash_mismatch')

    def test_capture_free_scenario_cannot_claim_a_different_completion_tick(self):
        with tempfile.TemporaryDirectory() as root:
            run = fixture(root, 'scenario')
            assertion = {'kind': 'speed_at_least', 'tick': 1, 'mps': 0}
            (run / 'scenario.json').write_text(json.dumps({'definition': {
                'assertions': [assertion], 'required_capture': False}}))
            summary = {'passed': True, 'assertions_declared': 1, 'assertions_executed': 1,
                       'assertions_passed': 1, 'completed_tick': TICKS,
                       'results': [{'assertion': assertion, 'passed': True}], 'capture': None}
            events = read_lines(run / 'events.jsonl')
            events.append(event('scenario.finished', len(events) + 1, TICKS, **summary))
            write_lines(run / 'events.jsonl', events)
            health = json.loads((run / 'recorder-health.json').read_text())
            health['streams']['events'].update(accepted=len(events), written=len(events))
            (run / 'recorder-health.json').write_text(json.dumps(health))
            (run / 'scenario-summary.json').write_text(json.dumps(summary))
            result, _ = verify(run)
            self.assertEqual(result.returncode, 0, result.stdout)
            summary['completed_tick'] = 999
            events[-1]['fields'] = summary
            write_lines(run / 'events.jsonl', events)
            (run / 'scenario-summary.json').write_text(json.dumps(summary))
            self.assertRejected(run, 'scenario_tick_mismatch')

    def test_targeted_player_reset_uses_vehicle_identity_at_ingress(self):
        with tempfile.TemporaryDirectory() as root:
            run = fixture(root)
            target = {'action': 'vehicle', 'target': 'pilot-350z', 'command': {'action': 'reset'}}
            ingress = command(2, 'ingress', RESET_TICK - 1)
            ingress.update(actor_id='pilot-350z', command=target)
            commands = read_lines(run / 'commands.jsonl') + [ingress]
            write_lines(run / 'commands.jsonl', commands)
            def apply(rows):
                reset = next(row for row in rows if row['kind'] == 'vehicle.reset')
                reset['command_id'] = ingress['command_id']
                reset['fields'].update(vehicle_id='pilot-350z', requested_command=target, disposition='applied')
            mutate(run, 'events', apply)
            health = json.loads((run / 'recorder-health.json').read_text())
            health['streams']['commands'].update(accepted=len(commands), written=len(commands))
            (run / 'recorder-health.json').write_text(json.dumps(health))
            result, _ = verify(run)
            self.assertEqual(result.returncode, 0, result.stdout)
            for value in (None, [], 'reset'):
                commands[1]['command'] = value
                write_lines(run / 'commands.jsonl', commands)
                self.assertRejected(run, 'trace_invalid_value', file='commands.jsonl')

    def test_completed_time_units_and_wheel_shapes_are_checked(self):
        for change in (lambda r: r.__setitem__('time', r['tick'] / 60),
                       lambda r: r['evidence'].__setitem__('wheels', [])):
            with self.subTest(change=change), tempfile.TemporaryDirectory() as root:
                run = fixture(root)
                mutate(run, 'physics', lambda rows: change(rows[2]))
                self.assertRejected(run, 'trace_invalid_value')

    def test_visual_only_reports_require_three_views(self):
        with tempfile.TemporaryDirectory() as root:
            run = fixture(root)
            self.add_captures(run)
            report = json.loads((run / 'report.json').read_text())
            report[0]['visual_capture_only'] = True
            del report[0]['captures']['driving']
            (run / 'report.json').write_text(json.dumps(report))
            events = [r for r in read_lines(run / 'events.jsonl') if r['fields'].get('view') != 'driving']
            write_lines(run / 'events.jsonl', events)
            health = json.loads((run / 'recorder-health.json').read_text())
            health['streams']['events'].update(accepted=len(events), written=len(events))
            (run / 'recorder-health.json').write_text(json.dumps(health))
            result, _ = verify(run)
            self.assertEqual(result.returncode, 0, result.stdout)

    def test_nonnumeric_wheel_vectors_are_corrupt_records(self):
        with tempfile.TemporaryDirectory() as root:
            run = fixture(root)
            mutate(run, 'physics', lambda rows: rows[1]['evidence']['wheels'][0].__setitem__('origin', ['bad', None, False]))
            self.assertRejected(run, 'trace_invalid_value', file='physics.jsonl')

    def test_null_evidence_and_invalid_actor_tick_shapes_keep_locations(self):
        for stream, change in (
                ('physics', lambda row: row.__setitem__('evidence', None)),
                ('physics', lambda row: row.__setitem__('actor_id', [])),
                ('events', lambda row: row.__setitem__('tick', []))):
            with self.subTest(stream=stream), tempfile.TemporaryDirectory() as root:
                run = fixture(root)
                mutate(run, stream, lambda rows: change(rows[0]))
                self.assertRejected(run, 'trace_invalid_value', file=stream + '.jsonl')

    def test_same_capture_basename_in_another_directory_is_not_linked(self):
        with tempfile.TemporaryDirectory() as root:
            run = fixture(root)
            self.add_captures(run)
            (run / 'other').mkdir()
            (run / 'other/front.png').write_bytes((run / 'captures/front.png').read_bytes())
            report = json.loads((run / 'report.json').read_text())
            report[0]['captures']['front']['path'] = 'other/front.png'
            (run / 'report.json').write_text(json.dumps(report))
            self.assertRejected(run, 'capture_link_mismatch')

    def test_empty_smoke_report_cannot_verify_without_capture_evidence(self):
        with tempfile.TemporaryDirectory() as root:
            run = fixture(root, 'smoke')
            manifest = json.loads((run / 'manifest.json').read_text())
            manifest['gpu'] = {'backend': 'fixture'}
            (run / 'manifest.json').write_text(json.dumps(manifest))
            (run / 'report.json').write_text('[]')
            self.assertRejected(run, 'report_incomplete')

    def test_malformed_envelope_and_completed_scalar_shapes_are_located(self):
        for field, value in [('envelope', None), ('gear', 'first'), ('rpm', None),
                             ('wheel_lengths', [False] * 4)]:
            with self.subTest(field=field), tempfile.TemporaryDirectory() as root:
                run = fixture(root)
                def change(rows):
                    rows[1][field] = value
                    if field == 'envelope':
                        rows[1]['schema_version'] = 1
                mutate(run, 'physics', change)
                self.assertRejected(run, 'trace_invalid_value', file='physics.jsonl')

    def test_unexplained_counter_increment_is_rejected(self):
        with tempfile.TemporaryDirectory() as root:
            run = fixture(root)
            def bump(rows):
                for row in rows:
                    if row['tick'] >= 7:
                        row['reset_count'] += 1
            mutate(run, 'physics', bump)
            def match_completed(rows):
                for row in rows:
                    if row['kind'] == 'simulation.completed' and row['tick'] >= 7:
                        row['fields']['reset_count'] += 1
            mutate(run, 'events', match_completed)
            self.assertRejected(run, 'trace_reset_mismatch', file='physics.jsonl')

    def test_same_car_reselection_can_increment_counter_twice_in_one_tick(self):
        with tempfile.TemporaryDirectory() as root:
            run = fixture(root)
            events = read_lines(run / 'events.jsonl')
            commands = read_lines(run / 'commands.jsonl')
            insert = next(i for i, row in enumerate(events) if row['tick'] == 7)
            additions = []
            for number in (2, 3):
                cmd = command(number, 'ingress', 7)
                cmd['command'] = {'action': 'select_car', 'car': '350z', 'reset_to_spawn': True}
                commands.append(cmd)
                for kind, fields in [('command.outcome', {'disposition': 'applied',
                        'command': cmd['command'], 'selected_car': '350z'}),
                        ('car.switched', {'car': '350z'})]:
                    row = event(kind, 0, 7, command_id=cmd['command_id'], **fields)
                    row['actor_id'] = 'player-vehicle'
                    additions.append(row)
            events[insert:insert] = additions
            for seq, row in enumerate(events, 1):
                row['seq'] = seq
                if row['kind'] == 'simulation.completed' and row['tick'] >= 7:
                    row['fields']['reset_count'] += 2
            write_lines(run / 'events.jsonl', events)
            write_lines(run / 'commands.jsonl', commands)
            physics = read_lines(run / 'physics.jsonl')
            for row in physics:
                if row['tick'] >= 7:
                    row['reset_count'] += 2
            write_lines(run / 'physics.jsonl', physics)
            health = json.loads((run / 'recorder-health.json').read_text())
            for name, rows in [('events', events), ('commands', commands)]:
                health['streams'][name].update(accepted=len(rows), written=len(rows))
            (run / 'recorder-health.json').write_text(json.dumps(health))
            result, report = verify(run)
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            self.assertTrue(report['ok'])

    def test_unhashable_command_ids_are_located_without_a_traceback(self):
        for stream in ('events', 'commands'):
            for identity in ([], {}):
                with self.subTest(stream=stream, identity=identity), tempfile.TemporaryDirectory() as root:
                    run = fixture(root)
                    mutate(run, stream, lambda rows: rows[0].__setitem__('command_id', identity))
                    self.assertRejected(run, 'trace_invalid_value', file=stream + '.jsonl')

    def test_boolean_numeric_aliases_do_not_match_completed_controls_or_state(self):
        for kind, field in [('controls.effective', 'throttle'), ('simulation.completed', 'reset_count')]:
            with self.subTest(kind=kind), tempfile.TemporaryDirectory() as root:
                run = fixture(root)
                def corrupt(rows):
                    row = next(r for r in rows if r['kind'] == kind and r['tick'] == 7)
                    row['fields'][field] = True
                mutate(run, 'events', corrupt)
                code = 'trace_control_mismatch' if kind == 'controls.effective' else 'trace_state_mismatch'
                self.assertRejected(run, code, file='events.jsonl')

    def test_malformed_switch_command_is_a_located_failure(self):
        for value in (None, [], 'select_car'):
            with self.subTest(value=value), tempfile.TemporaryDirectory() as root:
                run = fixture(root)
                events = read_lines(run / 'events.jsonl')
                events[1]['fields']['command'] = value
                switched = event('car.switched', 0, 0, car='350z',
                                 command_id=f'{RUN_ID}:command:1')
                switched['actor_id'] = 'player-vehicle'
                events.insert(2, switched)
                for seq, row in enumerate(events, 1):
                    row['seq'] = seq
                write_lines(run / 'events.jsonl', events)
                self.assertRejected(run, 'trace_switch_mismatch', file='events.jsonl')

    def test_command_payload_boolean_numeric_alias_is_rejected(self):
        with tempfile.TemporaryDirectory() as root:
            run = fixture(root)
            commands = read_lines(run / 'commands.jsonl')
            commands[0]['command'] = {'action': 'drive', 'handbrake': 1}
            write_lines(run / 'commands.jsonl', commands)
            events = read_lines(run / 'events.jsonl')
            events[1]['fields']['command'] = {'action': 'drive', 'handbrake': True}
            write_lines(run / 'events.jsonl', events)
            self.assertRejected(run, 'trace_command_mismatch', file='commands.jsonl')

    def test_boolean_clock_aliases_are_rejected(self):
        for field, value in [('render_frame', True), ('query_tick', True),
                             ('step', False), ('integration_steps', True)]:
            with self.subTest(field=field), tempfile.TemporaryDirectory() as root:
                run = fixture(root)
                if field == 'render_frame':
                    def corrupt(rows):
                        next(r for r in rows if r['kind'] == 'simulation.completed' and r['tick'] == 1)[field] = value
                    mutate(run, 'events', corrupt)
                    self.assertRejected(run, 'trace_state_mismatch', file='events.jsonl')
                else:
                    def corrupt(rows):
                        target = rows[0]['evidence'] if field == 'query_tick' else rows[0]
                        target[field] = value
                    mutate(run, 'physics', corrupt)
                    self.assertRejected(run, 'trace_invalid_value', file='physics.jsonl')

    def test_future_command_outcome_cannot_prove_a_completed_action(self):
        with tempfile.TemporaryDirectory() as root:
            run = fixture(root)
            mutate(run, 'events', lambda rows: rows[1].__setitem__('tick', TICKS + 1))
            self.assertRejected(run, 'trace_command_mismatch', file='commands.jsonl')

    def test_boolean_scenario_counts_and_numeric_report_passes_are_rejected(self):
        for name in ('assertions_declared', 'assertions_executed', 'assertions_passed'):
            with self.subTest(name=name), tempfile.TemporaryDirectory() as root:
                run = fixture(root, 'scenario')
                assertion = {'kind': 'speed_at_least', 'tick': 1, 'mps': 0}
                (run / 'scenario.json').write_text(json.dumps({'definition': {'assertions': [assertion]}}))
                summary = {'passed': True, 'assertions_declared': 1, 'assertions_executed': 1,
                           'assertions_passed': 1, 'completed_tick': TICKS,
                           'results': [{'assertion': assertion, 'passed': True}]}
                summary[name] = True
                (run / 'scenario-summary.json').write_text(json.dumps(summary))
                events = read_lines(run / 'events.jsonl')
                events.append(event('scenario.finished', len(events) + 1, TICKS, **summary))
                write_lines(run / 'events.jsonl', events)
                health = json.loads((run / 'recorder-health.json').read_text())
                health['streams']['events'].update(accepted=len(events), written=len(events))
                (run / 'recorder-health.json').write_text(json.dumps(health))
                self.assertRejected(run, 'scenario_incomplete')
        with tempfile.TemporaryDirectory() as root:
            run = fixture(root)
            self.add_captures(run)
            path = run / 'report.json'
            reports = json.loads(path.read_text())
            reports[0]['passed'] = 1
            path.write_text(json.dumps(reports))
            self.assertRejected(run, 'report_incomplete')

    def test_completed_reset_counter_requires_an_exact_integer(self):
        for tick, value in [(1, 1e-8), (7, 1.0)]:
            with self.subTest(value=value), tempfile.TemporaryDirectory() as root:
                run = fixture(root)
                def corrupt(rows):
                    next(r for r in rows if r['kind'] == 'simulation.completed' and r['tick'] == tick)['fields']['reset_count'] = value
                mutate(run, 'events', corrupt)
                self.assertRejected(run, 'trace_state_mismatch', file='events.jsonl')

    def test_missing_scenario_definition_is_a_json_failure(self):
        with tempfile.TemporaryDirectory() as root:
            run = fixture(root, 'scenario')
            (run / 'scenario.json').write_text('{}')
            (run / 'scenario-summary.json').write_text('{}')
            self.assertRejected(run, 'scenario_unavailable')

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
