"""Matched production-city handling scenarios; no gameplay tuning overrides."""
import math

HZ = 120
CASES = {
    'brake': {'action_tick': 960, 'end_tick': 1800, 'steer': 0.0},
    'turn-left': {'action_tick': 600, 'end_tick': 840, 'steer': -0.5},
    'turn-right': {'action_tick': 600, 'end_tick': 840, 'steer': 0.5},
    'fast-left': {'action_tick': 960, 'end_tick': 1140, 'steer': -0.5},
    'fast-right': {'action_tick': 960, 'end_tick': 1140, 'steer': 0.5},
}


def drive(tick, throttle=0.0, brake=0.0, steer=0.0):
    return {'tick': tick, 'command': {'action': 'drive', 'throttle': throttle,
                                    'brake': brake, 'steer': steer}}


def scenario(car, case):
    """Only initial.car, name and car-identity assertion vary across the fleet."""
    spec = CASES[case]
    commands = [drive(240, throttle=1.0)]
    if case == 'brake':
        commands += [drive(spec['action_tick'], brake=1.0), drive(1440),
                     {'tick': 1680, 'command': {'action': 'reset'}}]
    else:
        commands += [drive(spec['action_tick'], throttle=1.0, steer=spec['steer'])]
    return {
        'schema_version': 1, 'name': f'handling-{case}-{car}', 'scene': 'driving',
        'context': {'kind': 'sandbox'}, 'seed': 17,
        'initial': {'car': car, 'position': [-150.0, 0.62, 458.0],
                    'rotation_y': -math.pi / 2},
        'readiness_timeout_ms': 10000, 'max_ticks': spec['end_tick'] + 1,
        'max_wall_ms': 60000, 'required_capture': False, 'commands': commands,
        'assertions': [
            {'tick': spec['end_tick'], 'kind': 'selected_car', 'car': car},
            {'tick': spec['end_tick'], 'kind': 'reset_count',
             'count': 1 if case == 'brake' else 0}],
    }
