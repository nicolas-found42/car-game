#!/usr/bin/env python3
"""Finalize, export, verify or inspect immutable recorder provenance."""
import argparse
import json
import sys
from pathlib import Path

from provenance_bundle import BundleError, export, load_index, verify
from provenance_legacy import export_legacy, legacy_manifest

ROOT = Path(__file__).resolve().parents[1]


def build_manifest(run: Path) -> dict:
    """Existing entry point; never infers historical assets from this checkout."""
    path = Path(run) / 'evidence-manifest.json'
    if path.is_file():
        existing = json.loads(path.read_text())
        if existing.get('schema') == 2:
            return load_index(run)
        # Preserve old baseline evidence rather than rewrite it with current hashes.
        return {**existing, 'legacy_provenance_unavailable': True}
    if (Path(run) / 'provenance/capture.json').is_file():
        raise BundleError('recording_not_finalized', prerequisite='finish_recorder_before_export')
    return legacy_manifest(run)


def selected_run(args):
    if args.run is not None:
        return args.run
    latest = ROOT / 'artifacts/latest'
    if not latest.is_file():
        raise BundleError('missing_run_selection')
    run = Path(latest.read_text().strip())
    return run if run.is_absolute() else ROOT / run


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--run', type=Path)
    parser.add_argument('--action', choices=['finalize', 'export', 'verify', 'inspect'], default='finalize')
    parser.add_argument('--output', type=Path)
    parser.add_argument('--require-replay', action='store_true')
    args = parser.parse_args()
    try:
        run = selected_run(args)
        if args.action == 'export':
            if args.output is None:
                raise BundleError('missing_export_destination')
            metadata = json.loads((run / 'evidence-manifest.json').read_text())
            if metadata.get('schema') == 1:
                result = export_legacy(run, args.output)
            else:
                result = export(run, args.output)
        elif args.action == 'verify':
            result = verify(run, args.require_replay)
        elif args.action == 'inspect':
            result = verify(run)
            if result['ok']:
                result['bundle'] = load_index(run)
        else:
            evidence = build_manifest(run)
            result = {'ok': True, 'cars': len(evidence.get('cars', [])),
                      'report_sha256': evidence.get('report_sha256'),
                      'legacy_provenance_unavailable': evidence.get('legacy_provenance_unavailable', False)}
    except BundleError as error:
        result = {'ok': False, 'errors': [error.detail]}
    except (OSError, ValueError, KeyError, TypeError):
        result = {'ok': False, 'errors': [{'code': 'invalid_or_unavailable_bundle'}]}
    print(json.dumps(result))
    return 0 if result['ok'] else 1


if __name__ == '__main__':
    sys.exit(main())
