#!/usr/bin/env python3
"""Check that a persisted GRID run has a complete, internally consistent evidence trail."""
import argparse
import json
import sys
from pathlib import Path
from recorder_health import inspect_health

ROOT = Path(__file__).resolve().parents[1]


def load_json(path: Path):
    return json.loads(path.read_text())


def read_events(run: Path) -> list[dict]:
    events = []
    for line_number, line in enumerate((run / 'events.jsonl').read_text().splitlines(), 1):
        if not line.strip():
            continue
        try:
            events.append(json.loads(line))
        except json.JSONDecodeError as error:
            raise AssertionError(f'events.jsonl:{line_number}: {error}') from error
    return events


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('run', nargs='?', type=Path, help='run directory; defaults to artifacts/latest')
    parser.add_argument("--health-only", action="store_true", help="inspect recording health without gameplay/GPU acceptance")
    parser.add_argument("--json", action="store_true", help="print machine-readable health or verification result")
    args = parser.parse_args()
    run = args.run
    if run is None:
        latest = ROOT / 'artifacts/latest'
        if not latest.is_file():
            raise SystemExit('No run selected and artifacts/latest is missing')
        run = Path(latest.read_text().strip())
        if not run.is_absolute():
            run = ROOT / run
    run = run.resolve()

    try:
        manifest = load_json(run / 'manifest.json')
        if not isinstance(manifest, dict):
            raise ValueError('manifest must be an object')
    except (OSError, ValueError):
        print(json.dumps({'schema': 1, 'ok': False, 'status': 'unknown',
                          'errors': [{'code': 'run_manifest_unavailable'}]}))
        return 1
    health = inspect_health(run, manifest)
    if args.health_only or not health['ok']:
        print(json.dumps(health))
        return 0 if health['ok'] else 1
    return verify_gameplay(run, manifest, health, args.json)


def verify_gameplay(run, manifest, health, output_json=False):
    events = read_events(run)
    kinds = [event['kind'] for event in events]
    assert kinds[0] == 'run.start', 'missing run.start'
    assert 'run.exit' in kinds, 'missing run.exit'
    assert 'run.failure' not in kinds, 'run recorded a failure'
    assert not (run / 'failure.json').exists(), 'failure.json exists'
    assert manifest.get('exit', {}).get('code') == 0, manifest.get('exit')
    assert manifest.get('gpu'), 'manifest has no GPU/environment record'
    assert (run / 'physics.jsonl').is_file(), 'missing physics.jsonl'
    assert (run / 'input.jsonl').is_file(), 'missing input.jsonl'
    assert (run / 'run.log').is_file(), 'missing run.log'

    reports = load_json(run / 'report.json')
    if manifest.get('mode') == 'fleet':
        assert len(reports) == 13, f'expected 13 reports, got {len(reports)}'
        assert kinds.count('car.report') == 13, f'expected 13 car.report events, got {kinds.count("car.report")}'
        assert kinds.count('screenshot.validated') == 52, (
            f'expected 52 validated captures, got {kinds.count("screenshot.validated")}'
        )
        assert kinds.count('screenshot.requested') >= 52
        assert 'run.complete' in kinds, 'missing run.complete'
    for report in reports:
        assert report.get('captures'), f'{report.get("id")}: missing captures'
        for view, capture in report['captures'].items():
            assert capture.get('sha256'), f'{report["id"]}/{view}: missing capture hash'
            assert (run / capture['path']).is_file(), f'{report["id"]}/{view}: capture file missing'
    if output_json:
        print(json.dumps({'schema': 1, 'ok': True, 'run_id': manifest['run_id'], 'recorder_health': health}))
    else:
        print(f'RUN EVIDENCE PASS: {manifest["run_id"]}; {len(events)} events, {len(reports)} car reports')
    return 0


if __name__ == '__main__':
    sys.exit(main())
