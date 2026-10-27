"""Correlate persisted actor populations, completed states and command outcomes."""
import json
import math
from collections import Counter
from pathlib import Path
from trace_integrity import rows


def same_state(left, right):
    if isinstance(left, list) and isinstance(right, list):
        return len(left) == len(right) and all(same_state(a, b) for a, b in zip(left, right))
    if type(left) in (int, float) and type(right) in (int, float):
        return math.isclose(left, right, rel_tol=1e-6, abs_tol=1e-7)
    if isinstance(left, bool) or isinstance(right, bool):
        return type(left) is type(right) and left == right
    return left == right


def completed_actor(record, state, seen, findings, line):
    key = (record.get('tick'), record.get('actor_id'))
    sample = state['samples'].get(key)
    if key in seen or sample is None:
        findings.add('trace_actor_mismatch', 'events.jsonl', line=line, tick=key[0], actor_id=key[1])
    seen.add(key)
    fields = record.get('fields', {})
    if sample is not None:
        if (type(record.get('render_frame')) is not int or record['render_frame'] < 0
                or record['render_frame'] != state['frames'].get(key)):
            findings.add('trace_state_mismatch', 'events.jsonl', line=line, field='render_frame')
        for field in ('position', 'linear_velocity', 'reset_count', 'vehicle_id'):
            matches = same_state(sample[field], fields.get(field))
            if field == 'reset_count':
                matches = type(fields.get(field)) is int and sample[field] == fields[field]
            if not matches:
                findings.add('trace_state_mismatch', 'events.jsonl', line=line, tick=key[0], actor_id=key[1], field=field)


def control_effect(record, state, seen, findings, line):
    key = (record.get('tick'), record.get('actor_id'))
    sample = state['samples'].get(key)
    if key in seen or sample is None:
        findings.add('trace_control_mismatch', 'events.jsonl', line=line)
    seen.add(key)
    if sample is not None:
        values = record.get('fields', {})
        if any(not same_state(sample['input'].get(k), values.get(k))
               for k in ('throttle', 'brake', 'steer', 'handbrake', 'reset', 'controller')):
            findings.add('trace_control_mismatch', 'events.jsonl', line=line, tick=key[0], actor_id=key[1])


def event_identity(record, run_id, previous, findings, line):
    if type(record.get('schema_version')) is not int or record.get('schema_version') != 1:
        findings.add('trace_schema_unsupported', 'events.jsonl', line=line)
    seq = record.get('seq')
    if type(seq) is not int or seq != previous + 1:
        findings.add('trace_event_sequence', 'events.jsonl', line=line, previous_seq=previous)
    if record.get('run_id') != run_id:
        findings.add('trace_identity_mismatch', 'events.jsonl', line=line)
    return seq if type(seq) is int else previous


def check_events(run, completed, state, findings, run_id):
    previous, ticks, resets, resolved, seen = 0, {}, Counter(), {}, set()
    effective, increments = set(), Counter()
    for line, record in rows(run, 'events.jsonl', findings):
        if findings.full():
            break
        previous = event_identity(record, run_id, previous, findings, line)
        tick, actor = record.get('tick'), record.get('actor_id')
        kind = record.get('kind')
        if kind == 'simulation.tick_completed':
            if tick != len(ticks) + 1:
                findings.add('trace_event_incomplete', 'events.jsonl', line=line, tick=tick)
            ticks[tick] = record.get('fields', {}).get('vehicle_count')
        elif kind == 'controls.effective':
            control_effect(record, state, effective, findings, line)
        elif kind == 'simulation.completed':
            completed_actor(record, state, seen, findings, line)
        elif kind == 'vehicle.reset':
            resets[(tick, actor)] += 1
            increments[(tick, actor)] += 1
        elif kind == 'car.switched':
            if valid_switch(record, resolved):
                increments[(tick, actor)] += 1
            else:
                findings.add('trace_switch_mismatch', 'events.jsonl', line=line)
        if kind in ('command.outcome', 'vehicle.reset') and record.get('command_id'):
            resolved.setdefault(record['command_id'], []).append(record)
    if set(ticks) != set(range(1, completed + 1)):
        findings.add('trace_event_incomplete', 'events.jsonl', completed_tick=completed)
    check_populations(ticks, state, seen, effective, resets, findings)
    check_counter_deltas(state, increments, findings)
    return resolved


def valid_switch(record, resolved):
    outcomes = resolved.get(record.get('command_id'), [])
    for outcome in outcomes:
        fields = outcome.get('fields', {})
        command = fields.get('command')
        if (outcome.get('kind') == 'command.outcome' and fields.get('disposition') == 'applied'
                and isinstance(command, dict) and command.get('action') in ('select_car', 'cycle_car')
                and outcome.get('actor_id') == record.get('actor_id')
                and outcome.get('tick') == record.get('tick')
                and fields.get('selected_car') == record.get('fields', {}).get('car')):
            return True
    return False


def check_counter_deltas(state, increments, findings):
    for key in state['counter_deltas'].keys() | increments.keys():
        if state['counter_deltas'].get(key, 0) != increments[key]:
            findings.add('trace_reset_mismatch', 'physics.jsonl', tick=key[0], actor_id=key[1])


def check_populations(ticks, state, seen, effective, resets, findings):
    actual = Counter(tick for tick, _ in state['samples'])
    for tick, expected in ticks.items():
        if type(expected) is not int or actual[tick] != expected:
            findings.add('trace_actor_coverage', 'physics.jsonl', tick=tick, expected=expected, actual=actual[tick])
    if effective != set(state['samples']):
        findings.add('trace_control_coverage', 'events.jsonl')
    if seen != set(state['samples']):
        findings.add('trace_actor_mismatch', 'events.jsonl')
    if resets != Counter({key: 1 for key in state['reset_ticks']}):
        findings.add('trace_reset_mismatch', 'events.jsonl')


def same_command(left, right):
    if isinstance(left, dict) and isinstance(right, dict):
        return left.keys() == right.keys() and all(same_command(left[k], right[k]) for k in left)
    if isinstance(left, list) and isinstance(right, list):
        return len(left) == len(right) and all(same_command(a, b) for a, b in zip(left, right))
    if isinstance(left, bool) or isinstance(right, bool):
        return type(left) is type(right) and left == right
    return left == right


def outcome_matches(record, outcome, completed):
    fields = outcome.get('fields', {})
    command = fields.get('command', fields.get('requested_command'))
    actor_matches = outcome.get('actor_id') == record.get('actor_id')
    if outcome.get('kind') == 'vehicle.reset':
        actor_matches = actor_matches or (
            fields.get('vehicle_id') == record.get('actor_id')
            and record.get('command', {}).get('target') == fields.get('vehicle_id'))
    return (actor_matches
            and type(outcome.get('tick')) is int and record['tick'] <= outcome['tick'] <= completed
            and same_command(command, record.get('command'))
            and fields.get('disposition') in ('applied', 'rejected', 'ignored', 'skipped'))


def check_commands(run, completed, resolved, findings, run_id):
    seen = set()
    for line, record in rows(run, 'commands.jsonl', findings):
        if findings.full():
            break
        if type(record.get('schema_version')) is not int or record.get('schema_version') != 1:
            findings.add('trace_schema_unsupported', 'commands.jsonl', line=line)
            break
        identity = record.get('command_id')
        if not isinstance(identity, str) or identity in seen:
            findings.add('trace_command_duplicate', 'commands.jsonl', line=line, command_id=identity)
        seen.add(identity)
        tick = record.get('tick')
        if record.get('run_id') != run_id or type(tick) is not int or tick < 0:
            findings.add('trace_identity_mismatch', 'commands.jsonl', line=line)
            continue
        command_outcomes(record, resolved.get(identity, []), completed, findings, line)
    for identity in resolved.keys() - seen:
        findings.add('trace_command_orphan', 'events.jsonl', command_id=identity)


def command_outcomes(record, outcomes, completed, findings, line):
    identity, tick = record['command_id'], record['tick']
    if len(outcomes) > 1:
        findings.add('trace_command_duplicate', 'events.jsonl', command_id=identity)
    if outcomes and not all(outcome_matches(record, item, completed) for item in outcomes):
        findings.add('trace_command_mismatch', 'commands.jsonl', line=line, command_id=identity)
    if tick > completed:
        findings.add('trace_command_mismatch', 'commands.jsonl', line=line, tick=tick, command_id=identity)
    elif not outcomes and tick < completed:
        findings.add('trace_command_unresolved', 'commands.jsonl', line=line, tick=tick, command_id=identity)


def check_counts(run, findings):
    health = json.loads((Path(run) / 'recorder-health.json').read_text())
    for name in ('events', 'commands', 'physics', 'input'):
        count = sum(1 for _ in rows(run, name + '.jsonl', findings))
        expected = health['streams'][name]['written']
        if count != expected:
            findings.add('trace_record_count', name + '.jsonl', expected=expected, actual=count)
