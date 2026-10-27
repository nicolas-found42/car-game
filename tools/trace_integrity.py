"""Semantic validation of recorded trace streams for the run verifier (#86).

Each finding is a machine-readable record: {code, file, line?, tick?, actor_id?, ...}.
Streams are read incrementally; a single malformed line stops that file's checks.
"""
import hashlib
import json
import math
from pathlib import Path

SUPPORTED_SCHEMA = 1
MAX_FINDINGS = 25
PLAYER = 'player-vehicle'
VECTORS = {'position': 3, 'rotation': 4, 'linear_velocity': 3, 'angular_velocity': 3}


class Findings:
    def __init__(self):
        self.errors = []
        self.unsupported = False

    def add(self, code, file, **where):
        if code == 'trace_schema_unsupported':
            self.unsupported = True
        if len(self.errors) < MAX_FINDINGS:
            self.errors.append({'code': code, 'file': file, **where})

    def has(self, file):
        return any(error['file'] == file for error in self.errors)

    def full(self):
        return len(self.errors) >= MAX_FINDINGS


def rows(run, name, findings):
    """Yield (line, record) for each parseable line; report and stop at the first bad one."""
    path = Path(run) / name
    try:
        handle = path.open('rb')
    except OSError:
        findings.add('trace_missing', name)
        return
    with handle:
        for line, raw in enumerate(handle, 1):
            if not raw.strip():
                continue
            try:
                record = json.loads(raw.decode('utf-8'))
                if not isinstance(record, dict):
                    raise ValueError('record is not an object')
            except ValueError as error:
                findings.add('trace_malformed_record', name, line=line, detail=str(error)[:120])
                return
            yield line, record


def finite(value):
    return type(value) in (int, float) and math.isfinite(value)


def vector_ok(value, size):
    return isinstance(value, list) and len(value) == size and all(finite(item) for item in value)


def invalid_fields(record):
    bad = [key for key, size in VECTORS.items() if not vector_ok(record.get(key), size)]
    if not finite(record.get('speed')):
        bad.append('speed')
    if type(record.get('reset_count')) is not int or record['reset_count'] < 0:
        bad.append('reset_count')
    evidence = record.get('evidence')
    if not isinstance(evidence, dict) or not isinstance(evidence.get('outcome'), str):
        bad.append('evidence.outcome')
    if not isinstance(record.get('input'), dict):
        bad.append('input')
    return bad


def schema_version(record):
    envelope = record.get('envelope')
    return envelope.get('schema_version') if isinstance(envelope, dict) else record.get('schema_version')


def tick_cursor(findings, name, last, line, record, actor=None):
    """Advance one stream's tick; report gaps, duplicates and reordering. Returns the new last tick."""
    tick = record.get('tick')
    if type(tick) is not int or tick < 1:
        findings.add('trace_invalid_value', name, line=line, field='tick', actor_id=actor)
        return last
    where = {'line': line, 'tick': tick, 'actor_id': actor}
    if tick == last:
        findings.add('trace_tick_duplicate', name, **where)
    elif tick < last:
        findings.add('trace_tick_out_of_order', name, previous_tick=last, **where)
    elif tick != last + 1:
        findings.add('trace_tick_gap', name, expected=last + 1, **where)
    return max(tick, last)


def check_reset(findings, state, line, record):
    """Reset one-shot, outcome and counter must agree; fleet car swaps also bump the counter without a reset."""
    actor, tick = record['actor_id'], record['tick']
    reset = record['evidence']['outcome'] == 'reset'
    held = record['input'].get('reset') is True
    delta = record['reset_count'] - state['counts'].get(actor, 0)
    if reset != held or delta not in (0, 1) or (reset and delta != 1):
        findings.add('trace_reset_mismatch', 'physics.jsonl', line=line, tick=tick, actor_id=actor)
    state['counts'][actor] = record['reset_count']
    if reset:
        state['reset_ticks'].add(tick)


def check_physics(run, completed, findings):
    state = {'counts': {}, 'reset_ticks': set(), 'held': {}, 'last': {}}
    name = 'physics.jsonl'
    for line, record in rows(run, name, findings):
        if findings.full():
            break
        actor = record.get('actor_id')
        if schema_version(record) != SUPPORTED_SCHEMA:
            findings.add('trace_schema_unsupported', name, line=line, schema_version=schema_version(record))
            break
        bad = invalid_fields(record) if isinstance(actor, str) else ['actor_id']
        state['last'][actor] = tick_cursor(findings, name, state['last'].get(actor, 0), line, record, actor)
        if bad:
            findings.add('trace_invalid_value', name, line=line, tick=record.get('tick'), actor_id=actor, fields=bad)
            continue
        check_reset(findings, state, line, record)
        if actor == PLAYER:
            state['held'][record['tick']] = record['input']
    if not state['last'] and not findings.has(name):
        findings.add('trace_empty', name, expected_ticks=completed)
    for actor, last in state['last'].items():
        if last != completed and not findings.full():
            findings.add('trace_tick_coverage', name, actor_id=actor, last_tick=last, completed_tick=completed)
    return state


def check_input(run, completed, held, findings):
    name, last, seen = 'input.jsonl', 0, False
    for line, record in rows(run, name, findings):
        if findings.full():
            break
        seen = True
        last = tick_cursor(findings, name, last, line, record)
        if held.get(record.get('tick')) != record:
            findings.add('trace_input_mismatch', name, line=line, tick=record.get('tick'))
    if not seen and not findings.has(name):
        findings.add('trace_empty', name, expected_ticks=completed)
    elif seen and last != completed:
        findings.add('trace_tick_coverage', name, last_tick=last, completed_tick=completed)


def check_events(run, completed, reset_ticks, findings):
    name, previous, ticks, resets, resolved = 'events.jsonl', 0, [], set(), set()
    for line, record in rows(run, name, findings):
        if findings.full():
            break
        if record.get('schema_version') != SUPPORTED_SCHEMA:
            findings.add('trace_schema_unsupported', name, line=line, schema_version=record.get('schema_version'))
            break
        if type(record.get('seq')) is not int or record['seq'] <= previous:
            findings.add('trace_event_sequence', name, line=line, previous_seq=previous)
        previous = record['seq'] if type(record.get('seq')) is int else previous
        if record.get('command_id'):
            resolved.add(record['command_id'])
        kind = record.get('kind')
        if kind == 'simulation.tick_completed':
            ticks.append(record.get('tick'))
        elif kind == 'vehicle.reset':
            resets.add(record.get('tick'))
    if ticks != list(range(1, completed + 1)) and not findings.has(name):
        missing = sorted(set(range(1, completed + 1)) - set(ticks))[:5]
        findings.add('trace_event_incomplete', name, completed_tick=completed, missing_ticks=missing)
    if resets != reset_ticks and not findings.full():
        findings.add('trace_reset_mismatch', name, event_ticks=sorted(resets), trace_ticks=sorted(reset_ticks))
    return resolved


def check_commands(run, completed, resolved, findings):
    name, seen = 'commands.jsonl', set()
    for line, record in rows(run, name, findings):
        if findings.full():
            break
        if record.get('schema_version') != SUPPORTED_SCHEMA:
            findings.add('trace_schema_unsupported', name, line=line, schema_version=record.get('schema_version'))
            break
        command_id = record.get('command_id')
        if command_id in seen:
            findings.add('trace_command_duplicate', name, line=line, command_id=command_id)
        seen.add(command_id)
        # Ingress at the final completed boundary has no later tick to take effect in.
        if command_id not in resolved and (record.get('tick') or 0) < completed:
            findings.add('trace_command_unresolved', name, line=line, tick=record.get('tick'), command_id=command_id)


def check_captures(run, manifest, findings):
    path = Path(run) / 'report.json'
    if not path.is_file():
        return
    try:
        reports = json.loads(path.read_text())
        entries = [(r.get('id'), view, c) for r in reports for view, c in r.get('captures', {}).items()]
    except (ValueError, AttributeError, OSError):
        findings.add('report_malformed', 'report.json')
        return
    for car, view, capture in entries:
        target = Path(run) / str(capture.get('path'))
        digest = hashlib.sha256(target.read_bytes()).hexdigest() if target.is_file() else None
        if digest != capture.get('sha256'):
            findings.add('capture_hash_mismatch', 'report.json', car=car, view=view, path=capture.get('path'))


def inspect_traces(run, manifest, health):
    """Return {'status', 'errors'} for one finalized run; 'verified' only when nothing was found."""
    findings = Findings()
    completed = health['completed_tick']
    if completed == 0:
        code = 'simulation_not_started'
        findings.add(code, 'recorder-health.json', completed_tick=0, exit=manifest.get('exit'))
        return {'status': 'failed', 'errors': findings.errors}
    state = check_physics(run, completed, findings)
    check_input(run, completed, state['held'], findings)
    resolved = check_events(run, completed, state['reset_ticks'], findings)
    check_commands(run, completed, resolved, findings)
    check_captures(run, manifest, findings)
    status = 'verified'
    if findings.errors:
        status = 'unsupported' if findings.unsupported else 'failed'
    return {'status': status, 'errors': findings.errors}
