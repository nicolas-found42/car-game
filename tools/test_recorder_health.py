#!/usr/bin/env python3
"""Public verifier regressions for recording health, independent of GPU availability."""
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

VERIFY = Path(__file__).with_name('verify_run.py')


def fixture(root):
    run = Path(root)
    (run / 'captures').mkdir()
    (run / 'captures/front.png').write_bytes(b'fixture')
    manifest = {'schema': 1, 'run_id': 'health-control', 'mode': 'smoke',
                'exit': {'code': 0}, 'ended_at': '2026-10-22T00:00:00Z',
                'gpu': {'backend': 'fixture'},
                'recorder_health': {'schema': 1, 'required': True, 'path': 'recorder-health.json'}}
    (run / 'manifest.json').write_text(json.dumps(manifest))
    (run / 'events.jsonl').write_text('{"kind":"run.start"}\n{"kind":"run.exit"}\n')
    for name in ('physics.jsonl', 'input.jsonl', 'run.log'):
        (run / name).write_text('')
    (run / 'report.json').write_text(json.dumps([{'id': '350z', 'captures': {
        'front': {'sha256': 'fixture', 'path': 'captures/front.png'}}}]))
    streams = {name: {'accepted': 0, 'written': 0, 'written_seq': 0, 'flushed_seq': 0, 'synced_seq': 0}
               for name in ('events', 'commands', 'physics', 'input', 'log')}
    streams['events'] = {'accepted': 2, 'written': 2, 'written_seq': 2, 'flushed_seq': 2, 'synced_seq': 2}
    health = {'schema': 1, 'run_id': 'health-control', 'phase': 'finalized', 'failed': False,
              'errors': [], 'dropped': 0, 'losses': [], 'loss_ranges_truncated': False,
              'produced': 2, 'accepted': 2, 'written': 2, 'durable_seq': 2,
              'queued_bytes': 0, 'queued_records': 0, 'last_durable_tick': 10,
              'completed_tick': 10, 'streams': streams}
    return run, health


class RecorderHealthTests(unittest.TestCase):
    def test_recorded_write_failure_cannot_pass_even_with_parseable_traces(self):
        with tempfile.TemporaryDirectory() as root:
            run, health = fixture(root)
            health['failed'] = True
            health['errors'] = [{'stage': 'write', 'stream': 'events', 'error': 'Other'}]
            (run / 'recorder-health.json').write_text(json.dumps(health))
            result = subprocess.run([sys.executable, str(VERIFY), str(run)], capture_output=True, text=True)
            self.assertNotEqual(result.returncode, 0, result.stdout)
            self.assertIn('recorder', result.stdout + result.stderr)

    def test_missing_or_interrupted_health_is_unknown_instead_of_healthy(self):
        with tempfile.TemporaryDirectory() as root:
            run, health = fixture(root)
            for case in ('missing', 'interrupted', 'legacy'):
                path = run / 'recorder-health.json'
                if path.exists():
                    path.unlink()
                manifest = json.loads((run / 'manifest.json').read_text())
                if case == 'interrupted':
                    health['phase'] = 'recording'
                    manifest['ended_at'] = None
                    path.write_text(json.dumps(health))
                if case == 'legacy':
                    manifest.pop('recorder_health', None)
                (run / 'manifest.json').write_text(json.dumps(manifest))
                result = subprocess.run([sys.executable, str(VERIFY), '--health-only', '--json', str(run)], capture_output=True, text=True)
                report = json.loads(result.stdout)
                self.assertNotEqual(result.returncode, 0)
                self.assertEqual(report['status'], 'unknown')

    def test_valid_health_requires_an_acknowledged_coherent_final_boundary(self):
        with tempfile.TemporaryDirectory() as root:
            run, health = fixture(root)
            for durable, expected in ((10, 0), (9, 1)):
                health['last_durable_tick'] = durable
                (run / 'recorder-health.json').write_text(json.dumps(health))
                result = subprocess.run([sys.executable, str(VERIFY), '--health-only', '--json', str(run)], capture_output=True, text=True)
                self.assertEqual(result.returncode, expected, result.stdout)

    def test_lost_records_and_invalid_status_types_are_rejected(self):
        with tempfile.TemporaryDirectory() as root:
            run, original = fixture(root)
            for update in ({'failed': 'false'}, {'dropped': 1}, {'losses': [{'first_tick': 4}]}, {'streams': {}}, {'queued_bytes': 10}, {'loss_ranges_unknown': True}):
                health = dict(original, **update)
                (run / 'recorder-health.json').write_text(json.dumps(health))
                result = subprocess.run([sys.executable, str(VERIFY), '--health-only', '--json', str(run)], capture_output=True, text=True)
                self.assertNotEqual(result.returncode, 0, result.stdout)


if __name__ == '__main__':
    unittest.main()
