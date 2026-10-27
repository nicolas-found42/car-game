"""Metrics with explicit tick boundaries for matched handling traces."""
import math

HZ = 120


def planar(velocity):
    return math.hypot(velocity[0], velocity[2])


def rotate(q, point):
    x, y, z, w = q
    px, py, pz = point
    cross = (y * pz - z * py, z * px - x * pz, x * py - y * px)
    cross2 = (y * cross[2] - z * cross[1], z * cross[0] - x * cross[2],
              x * cross[1] - y * cross[0])
    return tuple(p + 2 * (w * c + d) for p, c, d in zip(point, cross, cross2))


def heading(q):
    forward = rotate(q, (0, 0, -1))
    return math.atan2(forward[0], -forward[2])


def yaw_change(samples):
    angles = [heading(sample['rotation']) for sample in samples]
    return math.degrees(sum(math.atan2(math.sin(b - a), math.cos(b - a))
                            for a, b in zip(angles, angles[1:])))


def near_stop(states, action_tick, end_tick):
    """Distance/time to first <=1 m/s sample; later reverse is not failed braking."""
    previous = states[action_tick - 1]['position']
    distance = 0.0
    for tick in range(action_tick, end_tick):
        state = states[tick]
        position = state['position']
        distance += math.hypot(position[0] - previous[0], position[2] - previous[2])
        if planar(state['linear_velocity']) <= 1.0:
            return {'near_stop_s': (tick - action_tick + 1) / HZ,
                    'near_stop_m': distance, 'near_stop_tick': tick}
        previous = position
    return {'near_stop_s': None, 'near_stop_m': None, 'near_stop_tick': None}


def support(samples):
    count = len(samples)
    if not count:
        raise ValueError('empty support measurement window')
    contacts = [s['grounded'] for s in samples]
    loaded = [sum(load > 1.0 for load in s['wheel_loads']) for s in samples]
    tilts = [math.degrees(math.acos(max(-1, min(1, rotate(s['rotation'], (0, 1, 0))[1]))))
             for s in samples]
    return {'support_ticks': count, 'four_contact_pct': 100 * contacts.count(4) / count,
            'min_contacts': min(contacts), 'airborne_ticks': contacts.count(0),
            'four_loaded_pct': 100 * loaded.count(4) / count,
            'peak_tilt_deg': max(tilts),
            'peak_slip_deg': math.degrees(max(s['slip'] for s in samples))}


def body_corners(spec):
    boxes = [((0, 0, spec['body_center_z']),
              (spec['half_width'], .27, spec['half_length'])),
             ((0, max(.32, min(.6, spec['roof_height'] - .25)), spec['body_center_z'] + .13),
              (spec['half_width'] * .8, .25, spec['half_length'] * .5))]
    return [tuple(center[i] + sign[i] * extent[i] for i in range(3))
            for center, extent in boxes for x in [-1, 1] for y in [-1, 1] for z in [-1, 1]
            for sign in [(x, y, z)]]


def clearance(samples, spec):
    """Estimated rotated two-cuboid vertices versus the perimeter's z=436/479.5 limits."""
    corners = body_corners(spec)
    bounds = [s['position'][2] + rotate(s['rotation'], corner)[2]
              for s in samples for corner in corners]
    return min(min(bounds) - 436.0, 479.5 - max(bounds))


def measure(states, physics, case, spec):
    action, end = spec['action_tick'], spec['end_tick']
    samples = [physics[t] for t in range(action, 1440 if case == 'brake' else end)]
    result = {'approach_mps': planar(states[action - 1]['linear_velocity'])}
    if case == 'brake':
        result.update(near_stop(states, action, 1440))
        if result['near_stop_tick'] is not None:
            samples = [physics[t] for t in range(action, result['near_stop_tick'] + 1)]
        result['brake_1s_mps'] = planar(states[action + 119]['linear_velocity'])
        result['brake_2s_mps'] = planar(states[action + 239]['linear_velocity'])
    else:
        direction = spec['steer'] / abs(spec['steer'])
        result['turn_deg'] = direction * yaw_change([physics[t] for t in range(action, end + 1)])
        result['turn_exit_mps'] = planar(states[end - 1]['linear_velocity'])
        result['turn_speed_pct'] = 100 * result['turn_exit_mps'] / result['approach_mps']
        result['turn_lateral_m'] = direction * (states[end - 1]['position'][2]
                                               - states[action - 1]['position'][2])
    result.update(support(samples))
    return result
