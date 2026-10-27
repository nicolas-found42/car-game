"""Bounded incident interpretation over the recorder's existing evidence."""
from collections import deque
from pathlib import Path
import os
import shlex
from diagnosis_io import bounded_records, metadata, records
from recorder_health import inspect_health

EVENT_LIMIT = 100000
SUPPORT_LIMIT = 128


def run_paths(root, errors):
    """Use the recorder index when present; recover unindexed directories read-only."""
    root = Path(root)
    if not root.is_dir():
        errors.append({'code': 'root_unavailable', 'path': str(root)})
        return []
    paths = set()
    index = root / 'index.jsonl'
    if index.exists():
        for count, row in enumerate(bounded_records(index, errors)):
            if count >= EVENT_LIMIT:
                errors.append({'code': 'index_scan_limit', 'path': str(index)})
                break
            run_id = row.get('run_id')
            if row.get('event') == 'start' and isinstance(run_id, str) and Path(run_id).name == run_id:
                paths.add(root / run_id)
    # The index can be absent, truncated, or lost after a crash. Preserve
    # recursive searches for historical roots without following directory links.
    for count, (directory, _, files) in enumerate(os.walk(root)):
        if count >= EVENT_LIMIT:
            errors.append({'code': 'directory_scan_limit', 'path': str(root)})
            break
        if 'manifest.json' in files:
            paths.add(Path(directory))
    return sorted(path for path in paths if (path / 'manifest.json').is_file())


def resolve_run(value, root):
    path = Path(value)
    if path.is_dir():
        return path
    if path.name != str(path) or path.name in ('.', '..'):
        raise ValueError('run ID must be a single path component')
    path = Path(root) / path
    if not path.is_dir():
        raise ValueError('run not found: ' + str(value))
    return path


def event_boundaries(run, errors):
    support, boundaries, spans = deque(maxlen=SUPPORT_LIMIT), {}, {}
    scanned_bytes = 0
    for count, row in enumerate(records(run / 'events.jsonl', errors)):
        size = len(str(row).encode())
        scanned_bytes += size
        if scanned_bytes > 64 * 1024 * 1024 or count >= EVENT_LIMIT:
            errors.append({'code': 'event_scan_limit'})
            break
        kind = row.get('kind', '')
        if not relevant(kind):
            continue
        if size > 16384:
            if len(errors) < 16:
                errors.append({'code': 'support_record_too_large', 'kind': kind})
            continue
        support.append(row)
        add_boundary(row, boundaries, spans)
    return support, boundaries, spans


def add_boundary(row, boundaries, spans):
    kind = row['kind']
    if not kind.startswith(('asset.', 'screenshot.', 'scenario.capture', 'scenario.readiness', 'scenario.ready', 'car.start')):
        return
    boundaries[kind] = row
    correlation = row.get('correlation_id')
    if isinstance(correlation, str) and (correlation in spans or len(spans) < SUPPORT_LIMIT):
        span = spans.setdefault(correlation, {'correlation_id': correlation, 'records': []})
        if len(span['records']) < 16:
            span['records'].append(row)
        span['last_observed_boundary'] = kind


def incident(run):
    manifest = metadata(run / 'manifest.json')
    health = inspect_health(run, manifest)
    errors = []
    support, boundaries, spans = event_boundaries(run, errors)
    phases = physics_phases(run, errors)
    failure = optional_metadata(run / 'failure.json')
    scenario = optional_metadata(run / 'scenario.json')
    summary = optional_metadata(run / 'scenario-summary.json')
    status = status_for(manifest, health, failure, summary)
    classify_spans(spans, bool(manifest.get('ended_at')), manifest.get('mode'))
    if status == 'complete' and any(span['status'] == 'incomplete' for span in spans.values() if 'capture' in span['correlation_id']):
        status = 'incomplete'
    return {'run': str(run), 'run_id': manifest.get('run_id'), 'status': status,
            'recording': health, 'failure': failure, 'scenario_summary': summary,
            'last_boundaries': boundaries, 'spans': list(spans.values()), 'supporting_records': list(support),
            'physics_phase_samples': phases,
            'support_truncated': len(support) == SUPPORT_LIMIT,
            'unknowns': ['GPU submission, execution and readback internals are unobserved unless explicitly recorded.',
                         'A missing callback does not establish a root cause.'],
            'reproduction': reproduction(run, scenario), 'errors': errors}


def physics_phases(run, errors):
    path = run / 'physics.jsonl'
    if not path.is_file():
        return {'availability': 'unavailable', 'samples': []}
    samples, size = deque(maxlen=8), 0
    for count, row in enumerate(records(path, errors)):
        size += len(str(row).encode())
        if count >= EVENT_LIMIT or size > 64 * 1024 * 1024:
            errors.append({'code': 'physics_scan_limit'})
            break
        if 'phase_durations' in row:
            samples.append({key: row.get(key) for key in ('tick', 'actor_id', 'vehicle_id', 'boundary', 'envelope', 'phase_durations')})
    return {'availability': 'available' if samples else 'unavailable', 'samples': list(samples), 'path': str(path)}


def classify_spans(spans, ended, mode):
    terminal = {'asset.loaded', 'scenario.ready', 'scenario.capture_saved', 'screenshot.saved'}
    for span in spans.values():
        complete_scene = span['last_observed_boundary'] == 'asset.instantiated' and mode not in ('smoke', 'fleet', 'visual')
        if span['last_observed_boundary'] in terminal or complete_scene:
            span['status'] = 'complete'
        else:
            span['status'] = 'incomplete' if ended else 'ongoing_or_stale'


def relevant(kind):
    return isinstance(kind, str) and kind.startswith(('asset.', 'screenshot.', 'scenario.', 'car.start',
                                                       'window.focus', 'gamepad.', 'diagnostic.'))


def optional_metadata(path):
    return metadata(path) if path.is_file() else None


def status_for(manifest, health, failure, summary):
    exit_info = manifest.get('exit')
    if failure or (summary and summary.get('passed') is False) or (isinstance(exit_info, dict) and exit_info.get('code') not in (None, 0)):
        return 'failed'
    if health['status'] == 'degraded':
        return 'degraded'
    if not manifest.get('ended_at'):
        return 'ongoing_or_stale'
    if not health.get('ok'):
        return 'incomplete'
    return 'complete'


def reproduction(run, scenario):
    if scenario and isinstance(scenario.get('definition'), dict):
        definition = run / 'scenario-definition.json'
        if definition.is_file():
            backend = scenario.get('render_backend')
            flags = {'disabled': ' --render-disabled', 'offscreen': ' --offscreen'}.get(backend, '')
            fault = scenario.get('diagnostic_fault')
            if fault in ('readiness-stall', 'capture-missing'):
                flags += ' --diagnostic-fault ' + fault
            return {'availability': 'available', 'command': 'cargo run -- --scenario ' + shlex.quote(str(definition)) + flags}
        return {'availability': 'unavailable', 'reason': 'standalone scenario definition not recorded'}
    manifest = optional_metadata(run / 'manifest.json') or {}
    args = manifest.get('args', [])
    if '--diagnostic-fault' in args and 'asset-readiness-stall' in args and manifest.get('mode') in ('smoke', 'fleet'):
        mode = '--smoke' if manifest['mode'] == 'smoke' else '--fleet-check'
        return {'availability': 'available', 'command': 'cargo run -- --offscreen ' + mode + ' --diagnostic-fault asset-readiness-stall'}
    replay = run / 'input.jsonl'
    if replay.is_file():
        return {'availability': 'available', 'command': 'cargo run -- --replay-input ' + shlex.quote(str(replay))}
    return {'availability': 'unavailable', 'reason': 'scenario and replay input not recorded'}
