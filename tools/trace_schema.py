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
    if (type(value.get('tick')) is not int or type(value.get('step')) is not int
            or value.get('tick') != tick or value.get('step') != tick - 1):
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
        for field in ('origin', 'direction', 'point', 'normal'):
            value = wheel.get(field)
            if value is None and field in ('point', 'normal'):
                continue
            if (not isinstance(value, list) or len(value) != 3
                    or any(type(x) not in (int, float) or not math.isfinite(x) for x in value)):
                return ['evidence.wheels.' + field]
    return []


def number(value):
    return type(value) in (int, float) and math.isfinite(value)


def numeric_vector(value, size):
    return isinstance(value, list) and len(value) == size and all(number(x) for x in value)


def state_errors(record):
    bad = []
    for name in ('time', 'speed', 'slip', 'rpm', 'steering'):
        if not number(record.get(name)):
            bad.append(name)
    for name in ('gear', 'grounded', 'ray_misses'):
        if type(record.get(name)) is not int:
            bad.append(name)
    for name in ('vehicle_id', 'body_id', 'car', 'query_mode'):
        if not isinstance(record.get(name), str) or not record[name]:
            bad.append(name)
    for name in ('wheel_lengths', 'wheel_loads'):
        if not numeric_vector(record.get(name), 4):
            bad.append(name)
    return bad


def completed_errors(record, run_id):
    envelope = record.get('envelope', {})
    bad = []
    if not isinstance(envelope, dict):
        bad.append('envelope')
        envelope = {}
    for name, expected in (('run_id', run_id), ('actor_id', record.get('actor_id')),
                           ('tick', record.get('tick')), ('boundary', 'completed_tick')):
        if envelope.get(name) != expected:
            bad.append('envelope.' + name)
    if type(envelope.get('tick')) is not int or type(envelope.get('render_frame')) is not int or envelope.get('render_frame', -1) < 0:
        bad.append('envelope.clock')
    if record.get('boundary') != 'completed_tick':
        bad.append('boundary')
    if not finite_tree(record):
        bad.append('nonfinite')
    bad.extend(input_errors(record.get('input'), record.get('tick', 0)))
    evidence = record.get('evidence')
    if not isinstance(evidence, dict):
        return bad + ['evidence']
    if evidence.get('input') != record.get('input'):
        bad.append('evidence.input')
    if evidence.get('query_tick') != record.get('tick'):
        bad.append('evidence.query_tick')
    bad.extend(wheel_errors(evidence))
    bad.extend(state_errors(record))
    tick, time = record.get('tick'), record.get('time')
    if (type(tick) is int and (type(time) not in (int, float)
            or not math.isclose(time, tick / 120, rel_tol=1e-5, abs_tol=1e-4))):
        bad.append('time_seconds_at_120hz')
    if record.get('step') != record.get('tick', 0) - 1:
        bad.append('step')
    if record.get('integration_steps') not in (0, 1):
        bad.append('integration_steps')
    return bad
