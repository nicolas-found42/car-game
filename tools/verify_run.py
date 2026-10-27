#!/usr/bin/env python3
"""Check that a persisted GRID run has a complete, internally consistent evidence trail."""
import argparse
import json
import sys
from pathlib import Path
from collections import Counter
from recorder_health import inspect_health
from trace_integrity import Findings, inspect_traces, rows

ROOT = Path(__file__).resolve().parents[1]


def load_json(path: Path):
    return json.loads(path.read_text())


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
    try:
        return verify_gameplay(run, manifest, health, args.json)
    except Exception as error:  # a verifier fault must never read as success
        print(json.dumps({'schema': 2, 'ok': False, 'status': 'unknown', 'run_id': manifest.get('run_id'),
                          'errors': [{'code': 'verifier_internal_error', 'detail': repr(error)[:200]}]}))
        return 1


def lifecycle(run, manifest, findings):
    """Mode-aware lifecycle and acceptance checks; returns the event-kind counts."""
    kinds, first = Counter(), None
    for _, event in rows(run, 'events.jsonl', findings):
        first = first or event.get('kind')
        kinds[event.get('kind')] += 1
    rendered = manifest.get('mode') in ('fleet', 'smoke')
    checks = [
        (first == 'run.start', 'missing_run_start', 'events.jsonl'),
        (kinds['run.exit'] > 0, 'missing_run_exit', 'events.jsonl'),
        (not kinds['run.failure'] and not (run / 'failure.json').exists(), 'run_recorded_failure', 'events.jsonl'),
        (manifest.get('exit', {}).get('code') == 0, 'nonzero_exit', 'manifest.json'),
        (bool(manifest.get('gpu')) or not rendered, 'missing_gpu_record', 'manifest.json'),
        ((run / 'run.log').is_file(), 'missing_run_log', 'run.log'),
    ]
    for ok, code, file in checks:
        if not ok:
            findings.add(code, file)
    return kinds


def fleet_findings(run, kinds, findings):
    try:
        reports = json.loads((run / 'report.json').read_text())
    except (OSError, ValueError):
        findings.add('report_unavailable', 'report.json')
        return 0
    expected = {'car.report': 13, 'screenshot.validated': 52}
    for kind, count in expected.items():
        if kinds[kind] != count:
            findings.add('fleet_event_count', 'events.jsonl', kind=kind, expected=count, actual=kinds[kind])
    if len(reports) != 13:
        findings.add('fleet_report_count', 'report.json', expected=13, actual=len(reports))
    for kind in ('screenshot.requested',):
        if kinds[kind] < 52:
            findings.add('fleet_event_count', 'events.jsonl', kind=kind, expected=52, actual=kinds[kind])
    if not kinds['run.complete']:
        findings.add('missing_run_complete', 'events.jsonl')
    return len(reports)


def verify_gameplay(run, manifest, health, output_json=False):
    findings = Findings()
    kinds = lifecycle(run, manifest, findings)
    reports = fleet_findings(run, kinds, findings) if manifest.get('mode') == 'fleet' else 0
    traces = inspect_traces(run, manifest, health)
    errors = findings.errors + traces['errors']
    status = traces['status'] if not findings.errors else 'failed'
    if errors and status == 'verified':
        status = 'failed'
    result = {'schema': 2, 'ok': not errors, 'status': 'verified' if not errors else status,
              'run_id': manifest.get('run_id'), 'errors': errors, 'recorder_health': health}
    if errors or output_json:
        print(json.dumps(result))
    else:
        print(f'RUN EVIDENCE PASS: {manifest["run_id"]}; {sum(kinds.values())} events, {reports} car reports')
    return 1 if errors else 0


if __name__ == '__main__':
    sys.exit(main())
