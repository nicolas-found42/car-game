"""Streaming readers for raw historical evidence; never substitute current assets."""
import gzip
import json
from pathlib import Path
from recorder_health import inspect_health


def metadata(path, limit=2 * 1024 * 1024):
    path = Path(path)
    if path.stat().st_size > limit:
        raise ValueError(f'metadata exceeds {limit} bytes: {path.name}')
    value = json.loads(path.read_text())
    if not isinstance(value, dict):
        raise ValueError('metadata must be an object: ' + path.name)
    return value


def records(path, errors):
    """Bound each line, retain explicit corruption markers, and stream large files."""
    path = Path(path)
    if not path.exists():
        errors.append({'code': 'stream_unavailable', 'path': str(path)})
        return
    opener = gzip.open if path.suffix == '.gz' else open
    with opener(path, 'rt') as stream:
        number = 0
        while line := stream.readline(2 * 1024 * 1024 + 1):
            number += 1
            if len(line) > 2 * 1024 * 1024:
                errors.append({'code': 'oversized_line', 'line': number, 'path': str(path)})
                return
            try:
                value = json.loads(line)
                if not isinstance(value, dict):
                    raise ValueError('record must be an object')
                if 'fields' in value and not isinstance(value['fields'], dict):
                    raise ValueError('event fields must be an object')
                yield value
            except ValueError:
                if len(errors) >= 16:
                    return
                errors.append({'code': 'invalid_record', 'line': number, 'path': str(path)})


def bounded_records(path, errors):
    size = 0
    for count, row in enumerate(records(path, errors)):
        size += len(json.dumps(row).encode())
        if count >= 100000 or size > 64 * 1024 * 1024:
            errors.append({'code': 'stream_scan_limit', 'path': str(path)})
            return
        yield row


def selected(row, options):
    tick = row.get('tick', -1)
    if not isinstance(tick, (int, float)):
        return False
    car = row.get('car', row.get('fields', {}).get('car'))
    identifiers = [('actor_id', 'actor'), ('vehicle_id', 'vehicle_id'), ('command_id', 'command_id'), ('correlation_id', 'correlation_id')]
    envelope = row.get('envelope') or {}
    fields = row.get('fields', {})
    entity = row.get('entity', fields.get('runtime_entity'))
    return (all(getattr(options, flag, None) is None or row.get(key, envelope.get(key, fields.get(key))) == getattr(options, flag) for key, flag in identifiers)
            and (getattr(options, 'entity', None) is None or str(entity) == options.entity)
            and (options.car is None or car == options.car)
            and (options.first is None or tick >= options.first)
            and (options.last is None or tick <= options.last)
            and (getattr(options, 'kind', None) is None or row.get('kind') == options.kind))


def stream_records(run, stream, incident, errors):
    if incident is None:
        yield from bounded_records(run / (stream + '.jsonl'), errors)
        return
    for wrapped in bounded_records(run / 'incidents' / f'{incident:04d}' / 'incident-history.jsonl', errors):
        if (wrapped.get('stream') not in ['events', 'commands', 'physics', 'input']
                or type(wrapped.get('tick')) is not int or type(wrapped.get('recorder_seq')) is not int):
            errors.append({'code': 'invalid_incident_envelope'})
            return
        if wrapped.get('stream') != stream:
            continue
        row = wrapped.get('record')
        if not isinstance(row, dict):
            errors.append({'code': 'invalid_incident_record'})
            return
        if row.get('tick', wrapped.get('tick')) != wrapped.get('tick'):
            errors.append({'code': 'incident_tick_mismatch'})
            return
        row.setdefault('tick', wrapped.get('tick', -1))
        row['_incident_recorder_seq'] = wrapped.get('recorder_seq')
        yield row


def incident_metadata(run, number):
    return metadata(run / 'incidents' / f'{number:04d}' / 'incident.json') if number else None


def run_summary(run):
    manifest = metadata(run / 'manifest.json')
    health = inspect_health(run, manifest)
    incident_paths = [run / 'failure.json', run / 'incident.json', *sorted((run / 'incidents').glob('*/incident.json'))]
    reasons = sorted({metadata(p).get('reason', 'unknown') for p in incident_paths if p.is_file()})
    return {'run': str(run), 'run_id': manifest.get('run_id'), 'mode': manifest.get('mode'),
            'car': manifest.get('car'), 'started_at': manifest.get('started_at'),
            'build': manifest.get('binary_sha256'), 'git': manifest.get('git'),
            'cars': sorted({manifest.get('car', '')} | {p.stem for p in (run / 'cars').glob('*.json')}),
            'recording_status': health['status'],
            'reasons': reasons,
            'behavioral_exit': manifest.get('exit')}
