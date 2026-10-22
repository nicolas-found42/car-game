#!/usr/bin/env python3
"""Build a content-addressed evidence manifest for one persisted GRID run."""
import argparse
import hashlib
import json
import sys
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open('rb') as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b''):
            digest.update(chunk)
    return digest.hexdigest()


def build_manifest(run: Path) -> dict:
    run = run.resolve()
    manifest = json.loads((run / 'manifest.json').read_text())
    report_path = run / 'report.json'
    reports = json.loads(report_path.read_text())
    roster = {
        car['id']: car
        for car in json.loads((ROOT / 'assets/cars/roster.json').read_text())
    }
    cars = []
    for report in reports:
        car = roster[report['id']]
        asset = ROOT / 'assets' / car['path']
        receipt = asset.with_suffix('.json')
        captures = {}
        for view, record in (report.get('captures') or {}).items():
            capture_path = run / record['path']
            captures[view] = {
                'path': record['path'],
                'sha256': sha256(capture_path) if capture_path.is_file() else None,
                'metrics': record.get('metrics'),
            }
        cars.append({
            'id': car['id'],
            'asset_path': car['path'],
            'asset_sha256': sha256(asset),
            'receipt_sha256': sha256(receipt),
            'captures': captures,
        })
    evidence = {
        'schema': 1,
        'run_id': manifest['run_id'],
        'generated_at': datetime.now(timezone.utc).isoformat(),
        'started_at': manifest.get('started_at'),
        'ended_at': manifest.get('ended_at'),
        'git': manifest.get('git'),
        'binary_sha256': manifest.get('binary_sha256'),
        'args': manifest.get('args'),
        'mode': manifest.get('mode'),
        'car': manifest.get('car'),
        'gpu': manifest.get('gpu'),
        'window': manifest.get('window'),
        'render_target': (reports[0].get('render_target') if reports else None),
        'report_sha256': sha256(report_path),
        'cars': cars,
    }
    (run / 'evidence-manifest.json').write_text(json.dumps(evidence, indent=2) + '\n')
    return evidence


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--run', type=Path, help='run directory; defaults to artifacts/latest')
    args = parser.parse_args()
    run = args.run
    if run is None:
        latest = ROOT / 'artifacts/latest'
        if not latest.is_file():
            raise SystemExit('No run selected and artifacts/latest is missing')
        run = Path(latest.read_text().strip())
        if not run.is_absolute():
            run = ROOT / run
    evidence = build_manifest(run)
    print(f'{len(evidence["cars"])} cars; report {evidence["report_sha256"][:12]}; {run / "evidence-manifest.json"}')
    return 0


if __name__ == '__main__':
    sys.exit(main())
