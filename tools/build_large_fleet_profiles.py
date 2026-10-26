"""Build explicit game-proxy profiles; never publish until the 57-car audit completes."""
import json
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / 'docs/verification/fleet-300'


def read(name):
    return json.loads((OUT / (name + '.json')).read_text())


def rpm(value):
    numbers = [float(n) for n in re.findall(r'\d+(?:\.\d+)?', str(value))]
    return sum(numbers) / len(numbers) if numbers else None


def native_parameters(data):
    fields = data['fields']
    if data['game'] == 'gran-turismo-4':
        engine, gear = fields['ENGINE'], fields['GEAR']
        drive = ['FR', 'FF', '4WD', 'MR', 'RR'][fields['DRIVETRAIN']['drivetype']]
        ratios = [gear[f'Gear{i}{suffix}'] / 1000 for i, suffix in enumerate(
            ['st', 'nd', 'rd', 'th', 'th', 'th', 'th', 'th', 'th', 'th', 'th'], 1)
            if gear.get(f'Gear{i}{suffix}', 0) > 0]
        values = {'mass_kg': data['catalog']['kg'],
                       'power_kw': engine['psvalue'] * .73549875,
                       'torque_nm': engine['torquevalue'] / 100 * 9.80665,
                       'power_rpm': rpm(engine['psrpm']), 'torque_rpm': rpm(engine['torquerpm']),
                       'redline_rpm': engine['revlimit'] * 100,
                       'gears': ratios, 'final_drive': gear['finalgearDF'] / 1000}
        return drive, {k: v for k, v in values.items() if v is not None}
    values = {'mass_kg': fields['mass_kg'],
                             'power_kw': fields['power_hp_game'] * .74569987158,
                             'torque_nm': fields['torque_kgm'] * 9.80665,
                             'power_rpm': fields['power_rpm'], 'torque_rpm': fields['torque_rpm']}
    return fields['drive'], {k:v for k,v in values.items() if v>0}


def estimated_parameters(car, drive, native):
    # Shared, declared priors, not a fit to any expanded-fleet scenario outcome.
    height = max(1., car['roof_height'] + sum(car['radii']) / 4)
    power_rpm = native.get('power_rpm', native.get('redline_rpm', 6500) * .85)
    torque_fallback = power_rpm if 'torque_nm' not in native else native.get('redline_rpm', 6500) * .6
    torque_rpm = native.get('torque_rpm', torque_fallback)
    return {'power_rpm': power_rpm, 'torque_rpm': torque_rpm,
            'redline_rpm': max(power_rpm + 600, torque_rpm + 1000),
            'gears': [3.3, 2.0, 1.4, 1.0, .8], 'final_drive': 3.8,
            'front_weight': {'FF': .62, 'FR': .53, '4WD': .57, 'MR': .42, 'RR': .38}[drive],
            'front_drive': {'FF': 1., 'FR': 0., '4WD': .5, 'MR': 0., 'RR': 0.}[drive],
            'cg_height_m': min(.85, max(.42, height * .4)),
            'ride_frequency_hz': [1.7, 1.8], 'damping_ratio': .68,
            'anti_roll_fraction': [.2, .2], 'tire_mu': 1.05, 'lateral_mu': .93,
            'cornering_per_load': [15., 15.5],
            'drag_area_m2': min(1.1, max(.5, car['half_width'] * 2 * height * .3)),
            'rolling_coefficient': .015, 'driveline_efficiency': .82 if drive == '4WD' else .87,
            'shift_seconds': .3, 'brake_decel_mps2': 9.5,
            'max_steer_rad': .56, 'steer_speed_falloff': .06, 'speed_governor_mps': None}


def profile(car, data):
    drive, native = native_parameters(data)
    params = {**estimated_parameters(car, drive, native), **native}
    if 'torque_nm' not in params:
        params['torque_nm'] = params['power_kw'] * 9549.297 / params['power_rpm']
    assert 250 <= params['mass_kg'] <= 5000 and params['redline_rpm'] >= params['power_rpm']
    source = 'docs/verification/fleet-300/native-specifications.json#' + car['id']
    evidence = {k: {'status': 'game_proxy' if k in native else 'estimate',
                    'basis': ('Decoded native stock display/specification; conversion and exclusions in native-interpretation.md. '
                              'A reduced-order proxy, not a recovered game engine or independently verified real car.')
                    if k in native else 'Shared reduced-order prior fixed before expanded scenario testing; no matched real-world measurement.'}
                for k in params}
    for key in native:
        evidence[key]['source'] = source
    for key in ['torque_nm', 'torque_rpm']:
        if key not in native and data['game']=='gran-turismo':
            evidence[key]['basis']='Native zero is a missing placeholder. Minimum torque consistent with rated power at power RPM, with coincident torque/power peaks, is a declared fallback; not a recovered native torque curve.'
    if drive != '4WD':
        evidence['front_drive'] = {'status': 'game_proxy', 'source': source,
                                   'basis': f'Native {drive} layout establishes the driven axle.'}
    else:
        evidence['front_drive']['basis'] = 'Native AWD layout verified; 50:50 drive split is an estimate, not a decoded differential.'
    return {'id': car['id'], 'corpus_reference': None, 'parameters': params, 'evidence': evidence,
            'applicability': f"{car['name']}, source {car['source_code']}. Game-derived proxy; unmatched market/load/tires. "
                             'Native display outputs and simple stock gearbox only; tire, chassis, delivery and differential behavior estimated.'}


def main():
    original = read('original-physics'); cars = read('prepared-additions'); native = read('native-specifications')
    original['cars'] += [profile(car, native[car['id']]) for car in cars]
    original['sources'] += ['docs/verification/fleet-300/native-specifications.json',
                            'docs/verification/fleet-300/native-interpretation.md']
    assert len(original['cars']) == 300
    assert original['cars'][:57] == read('original-physics')['cars']
    (OUT / 'prepared-physics.json').write_text(json.dumps(original, indent=2) + '\n')
    (OUT / 'prepared-roster.json').write_text(json.dumps(read('original-roster') + cars, indent=2) + '\n')
    print('Prepared 300 profiles; runtime assets unchanged.')


if __name__ == '__main__':
    main()
