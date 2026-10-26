"""Apply only source-backed RS4 inputs and declared two-car model calibration."""
import hashlib
import json
from pathlib import Path

ROOT = Path.cwd()
OUT = ROOT/'docs/verification/fleet57-reference-calibration'
ART = ROOT/'artifacts/vehicle-reference/fleet57-reference-calibration'
profile = json.loads((ART/'before-physics.json').read_text())
changes = json.loads((ART/'probe-candidate2-overrides.json').read_text())
reference_fields = {'mass_kg':'mass_kg', 'front_weight':'front_weight_fraction',
                    'gears':'gear_ratios', 'final_drive':'final_drive', 'drag_area_m2':'cd * frontal_area_m2'}
patch = []
for car in profile['cars']:
    if car['id'] not in changes:
        continue
    for key, value in changes[car['id']].items():
        previous = car['parameters'][key]; car['parameters'][key] = value
        if car['id'] == 'ref-e12' and key in reference_fields:
            evidence = {'status':'related_reference', 'source':'rs4-us07-oem',
                        'source_field':reference_fields[key],
                        'basis':'Audi US2007 sedan technical sheet; related year to2008US asset. Corrects priorEU2006 mass/estimated gearing. Cd times area for effective drag area. See calibration-supplement-20261025.json; no claim of exact2008 curb mass.'}
        else:
            source = ('rs4-cd07' if car['id'] == 'ref-e12' else
                      'ctsv-aw04' if key == 'lateral_mu' else 'ctsv-mt04')
            evidence = {'status':'estimate', 'source':source,
                        'basis':'Reduced-order model calibration to qualified period test envelope; not measured tire friction, driveline loss or brake hardware. Nominal engine output unchanged. '+
                                ('Lateral limit uses matched skidpad diameter; longitudinal tire coefficient stays separate.' if key=='lateral_mu' else
                                 'Service-brake effective deceleration reproduces the70mph stopping envelope.' if key=='brake_decel_mps2' else
                                 'Effective power delivery includes unmodeled torque-curve/clutch/shift/test effects; not a physical loss measurement.')}
        car['evidence'][key] = evidence
        patch.append({'car':car['id'],'field':key,'before':previous,'after':value,'evidence':evidence})
path = ROOT/'assets/cars/physics.json';path.write_text(json.dumps(profile,indent=2)+'\n')
assert (ROOT/'assets/cars/roster.json').read_bytes() == (ART/'before-roster.json').read_bytes()
old = json.loads((ART/'before-physics.json').read_text())
assert len(profile['cars']) == 57
assert all(a==b for a,b in zip(old['cars'],profile['cars']) if a['id'] not in changes)
(OUT/'profile-changes.json').write_text(json.dumps({'changed_editions':list(changes), 'unchanged_editions':55,
    'roster_sha256':hashlib.sha256((ART/'before-roster.json').read_bytes()).hexdigest(), 'fields':patch},indent=2)+'\n')
