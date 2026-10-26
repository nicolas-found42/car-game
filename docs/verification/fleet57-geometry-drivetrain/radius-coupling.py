"""Quantify kinematic sensitivities, separately from measured performance changes."""
import json
from pathlib import Path
out = Path('docs/verification/fleet57-geometry-drivetrain')
art = Path('artifacts/vehicle-reference/fleet57-geometry-drivetrain')
a = {r['id']: r for r in json.load(open(art/'roster.json'))}
b = {r['id']: r for r in json.load(open('assets/cars/roster.json'))}
p = {r['id']: r['parameters'] for r in json.load(open('assets/cars/physics.json'))['cars']}
def driven_radius(r, front):
    return 1/(front*(1/r[0]+1/r[1])/2+(1-front)*(1/r[2]+1/r[3])/2)


rows = []
for car, after in b.items():
    before = a[car]; f = p[car]['front_drive']
    old = sum(before['radii'])/4
    old_driven = driven_radius(before['radii'], f); new = driven_radius(after['radii'], f)
    rows.append({'car': car, 'nominal_front_drive': f,
                 'before_engine_radius_m': old, 'old_geometry_driven_radius_m': old_driven,
                 'after_engine_radius_m': new, 'fixed_torque_force_ratio_after_over_before': old/new,
                 'radius_rule_only_force_ratio_on_old_geometry': old/old_driven,
                 'geometry_only_force_ratio_with_driven_rule': old_driven/new})
(out/'radius-coupling.json').write_text(json.dumps({'rows': rows,
    'method': 'Float64 algebra over roster values; runtime uses f32. Ratios assume fixed engine torque, gear and efficiency and do not predict sprint time. AWD uses nominal coupling; dynamic force allocation is separate.'}, indent=2)+'\n')
