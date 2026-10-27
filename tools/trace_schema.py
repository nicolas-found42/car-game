"""The schema-1 completed-step contract (SI units, fixed tick and actor identity)."""
import math


def finite_tree(value):
    if isinstance(value, float):
        return math.isfinite(value)
    if isinstance(value, dict):
        return all(finite_tree(item) for item in value.values())
    if isinstance(value, list):
        return all(finite_tree(item) for item in value)
    return True


def input_errors(value, tick):
    if not isinstance(value, dict):
        return ['input']
    bad = []
    for name in ('throttle', 'brake', 'steer'):
        number = value.get(name)
        if type(number) not in (int, float) or not math.isfinite(number):
            bad.append('input.' + name)
    for name in ('handbrake', 'reset'):
        if type(value.get(name)) is not bool:
            bad.append('input.' + name)
    if value.get('tick') != tick or value.get('step') != tick - 1:
        bad.append('input.tick')
    if not isinstance(value.get('controller'), str):
        bad.append('input.controller')
    return bad


def wheel_errors(evidence):
    wheels = evidence.get('wheels')
    if not isinstance(wheels, list) or len(wheels) != 4:
        return ['evidence.wheels']
    for wheel in wheels:
        if not isinstance(wheel, dict):
            return ['evidence.wheels']
        for field in ('origin', 'direction'):
            value = wheel.get(field)
            if not isinstance(value, list) or len(value) != 3:
                return ['evidence.wheels.' + field]
        for field in ('point', 'normal'):
            value = wheel.get(field)
            if value is not None and (not isinstance(value, list) or len(value) != 3):
                return ['evidence.wheels.' + field]
    return []


def completed_errors(record, run_id):
    envelope = record.get('envelope', {})
    bad = []
    for name, expected in (('run_id', run_id), ('actor_id', record.get('actor_id')),
                           ('tick', record.get('tick')), ('boundary', 'completed_tick')):
        if envelope.get(name) != expected:
            bad.append('envelope.' + name)
    if record.get('boundary') != 'completed_tick':
        bad.append('boundary')
    if not finite_tree(record):
        bad.append('nonfinite')
    bad.extend(input_errors(record.get('input'), record.get('tick', 0)))
    evidence = record.get('evidence', {})
    if evidence.get('input') != record.get('input'):
        bad.append('evidence.input')
    if evidence.get('query_tick') != record.get('tick'):
        bad.append('evidence.query_tick')
    bad.extend(wheel_errors(evidence))
    tick, time = record.get('tick'), record.get('time')
    if (type(tick) is int and (type(time) not in (int, float)
            or not math.isclose(time, tick / 120, rel_tol=1e-5, abs_tol=1e-4))):
        bad.append('time_seconds_at_120hz')
    if record.get('step') != record.get('tick', 0) - 1:
        bad.append('step')
    if record.get('integration_steps') not in (0, 1):
        bad.append('integration_steps')
    return bad
