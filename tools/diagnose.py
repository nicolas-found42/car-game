#!/usr/bin/env python3
"""Search, inspect and compare local run evidence. Every response is versioned JSON."""
import argparse
import json
import sys
from itertools import zip_longest
from pathlib import Path

from diagnosis_io import incident_metadata, metadata, records, run_summary, selected, stream_records
from diagnosis_rules import PhysicsRules, clock_gap
from recorder_health import inspect_health


class JsonParser(argparse.ArgumentParser):
    def error(self, message):
        print(json.dumps({'schema_version': 1, 'error': 'invalid_arguments', 'detail': message}))
        raise SystemExit(2)


def search(options):
    results, errors = [], []
    for root in options.root or [Path('artifacts/runs')]:
        if not root.is_dir():
            errors.append({'code': 'root_unavailable', 'path': str(root)})
            continue
        for path in sorted(root.rglob('manifest.json')):
            if len(results) >= options.limit:
                return {'runs': results, 'errors': errors, 'truncated': True}
            try:
                row = run_summary(path.parent)
                if matches(row, options):
                    results.append(row)
            except (OSError, ValueError) as error:
                if len(errors) < 16:
                    errors.append({'code': 'metadata_unavailable', 'path': str(path), 'detail': str(error)})
    return {'runs': results, 'errors': errors, 'truncated': False}


def matches(row, options):
    if options.car and options.car not in row['cars']:
        return False
    if options.reason and options.reason not in row['reasons']:
        return False
    fields = [('mode', options.mode), ('recording_status', options.status)]
    if any(value is not None and row[key] != value for key, value in fields):
        return False
    if options.build and not (row.get('build') or '').startswith(options.build):
        return False
    if not dates_match(row, options):
        return False
    if options.scenario:
        path = Path(row['run']) / 'scenario.json'
        if not path.exists():
            return False
        value = metadata(path)
        return options.scenario in str(value.get('definition', value).get('name', ''))
    return True


def dates_match(row, options):
    stamp = row.get('started_at') or ''
    return ((options.since is None or stamp >= options.since)
            and (options.until is None or stamp <= options.until))


def timeline(options):
    errors, rows = [], []
    window = incident_metadata(options.run, options.incident)
    returned_bytes = 0
    for row in stream_records(options.run, options.stream, options.incident, errors):
        if selected(row, options):
            if len(rows) == options.limit:
                return {'records': rows, 'errors': errors, 'truncated': True, 'incident': window}
            returned_bytes += len(json.dumps(row).encode())
            if returned_bytes > 4 * 1024 * 1024:
                return {'records': rows, 'errors': errors, 'truncated': True, 'truncated_by': 'response_bytes', 'incident': window}
            rows.append(row)
    return {'records': rows, 'errors': errors, 'truncated': False, 'incident': window}


def analyze(options):
    errors, rules = [], PhysicsRules()
    manifest = metadata(options.run / 'manifest.json')
    for row in stream_records(options.run, 'physics', options.incident, errors):
        if selected(row, options):
            rules.observe(row)
    gaps, previous = [], None
    for row in stream_records(options.run, 'events', options.incident, errors):
        gap = clock_gap(previous, row)
        if gap:
            gaps.append(gap)
            gaps.sort(key=lambda g: g['evidence']['elapsed_s'], reverse=True)
            gaps = gaps[:32]
        previous = row
    return {'run': str(options.run), 'recording': inspect_health(options.run, manifest),
            'incident': incident_metadata(options.run, options.incident),
            'physics_samples_examined': rules.samples, 'gear_transitions': rules.transitions,
            'findings': rules.findings + gaps, 'errors': errors,
            'scope': {'car': options.car, 'first_tick': options.first, 'last_tick': options.last,
                      'clock_gaps': 'selected incident or whole-run events; host cause unknown'},
            'limits': ['Rules identify symptoms, not proof of a physics defect.',
                       'Legacy traces cannot recover unrecorded decisions.']}


def compare(options):
    errors = []
    left = records(options.run / 'physics.jsonl', errors)
    right = records(options.other / 'physics.jsonl', errors)
    fields = ['step', 'tick', 'actor_id', 'time', 'car', 'position', 'rotation', 'linear_velocity',
              'angular_velocity', 'gear', 'rpm', 'speed', 'slip', 'steering', 'input', 'ray_misses',
              'grounded', 'wheel_lengths', 'wheel_loads', 'drive_demands', 'longitudinal_forces', 'reset_count']
    count, unavailable = 0, set()
    optional = {'drive_demands', 'longitudinal_forces'}
    for a, b in zip_longest(left, right):
        if a is None or b is None:
            return {'equal': False, 'matched_samples': count, 'reason': 'length_mismatch', 'errors': errors}
        compared, missing = comparable_fields(a, b, fields, optional, unavailable)
        if missing:
            return {'equal': False, 'matched_samples': count, 'errors': [{'code':'missing_physics_fields','fields':missing}]}
        changes = [key for key in compared if json.dumps(a[key], sort_keys=True, allow_nan=False)
                   != json.dumps(b[key], sort_keys=True, allow_nan=False)]
        if changes:
            return {'equal': False, 'matched_samples': count, 'left_tick': a.get('tick'),
                    'right_tick': b.get('tick'), 'different_fields': changes, 'errors': errors}
        count += 1
    return {'equal': not errors and count > 0, 'matched_samples': count, 'errors': errors,
            'unavailable_fields': sorted(unavailable),
            'comparison': 'exact available listed physics fields, ordered samples; excludes added telemetry'}


def comparable_fields(a, b, fields, optional, unavailable):
    unavailable.update(key for key in optional if key not in a and key not in b)
    compared = [key for key in fields if key not in optional or key in a or key in b]
    missing = [key for key in compared if key not in a or key not in b]
    return compared, missing


def parser():
    root = JsonParser(description=__doc__)
    commands = root.add_subparsers(dest='action', required=True)
    find = commands.add_parser('search')
    find.add_argument('--root', type=Path, action='append')
    for key in ['car', 'mode', 'status', 'build', 'since', 'until', 'scenario', 'reason']:
        find.add_argument('--' + key)
    find.add_argument('--limit', type=int, default=100)
    for name in ['analyze', 'timeline', 'compare']:
        command = commands.add_parser(name)
        command.add_argument('run', type=Path)
        if name == 'compare':
            command.add_argument('other', type=Path)
        else:
            command.add_argument('--car')
            command.add_argument('--incident', type=int)
            command.add_argument('--first', '--from-tick', type=int)
            command.add_argument('--last', '--to-tick', type=int)
        if name == 'timeline':
            command.add_argument('--stream', choices=['physics', 'commands', 'input', 'events'], default='physics')
            command.add_argument('--kind')
            for flag in ['actor', 'command-id', 'correlation-id']:
                command.add_argument('--' + flag)
            command.add_argument('--limit', type=int, default=100)
    return root


def main():
    options = parser().parse_args()
    try:
        if hasattr(options, 'limit') and not 1 <= options.limit <= 1000:
            raise ValueError('limit must be between 1 and 1000')
        if getattr(options, 'incident', None) is not None and not 1 <= options.incident <= 8:
            raise ValueError('incident must be between 1 and 8')
        if getattr(options, 'first', None) is not None and options.first < 0:
            raise ValueError('first tick must be nonnegative')
        if getattr(options, 'last', None) is not None and options.last < (options.first or 0):
            raise ValueError('last tick must not precede first tick')
        result = globals()[options.action](options)
        print(json.dumps({'schema_version': 1, 'action': options.action, **result}, allow_nan=False))
        return int(bool(result.get('errors')))
    except (OSError, ValueError, KeyError, TypeError) as error:
        print(json.dumps({'schema_version': 1, 'error': type(error).__name__, 'detail': str(error)}))
        return 2


if __name__ == '__main__':
    sys.exit(main())
