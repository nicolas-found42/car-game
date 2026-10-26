"""Apply two qualified model estimates; preserve every other profile and geometry."""
import json
from pathlib import Path
ART=Path('artifacts/vehicle-reference/fleet57-next-calibration')
OUT=Path('docs/verification/fleet57-next-calibration')
p=json.loads((ART/'before-physics.json').read_text())
changes={'350z':('driveline_efficiency',.80,'z-cd-2003','Effective delivery estimate matched to Track6MT midrange envelope; nominal outputs/mass/gears unchanged. Not a measured driveline loss. Unknown rollout is not converted.'),
         'gt4-nsx':('lateral_mu',.98,'nsx-rt-1990','Conservative period tire-grip envelope estimate: C/D .86g300ft, R&T .93g makeshift circle of unknown diameter. Original Yokohama A-02215/16-inch equipment; build status and test conditions remain qualified. Not an exact measured friction coefficient.')}
for car in p['cars']:
 if car['id'] in changes:
  key,value,source,basis=changes[car['id']]
  car['parameters'][key]=value
  car['evidence'][key]={'status':'estimate','source':source,'basis':basis}
p['sources']+=['docs/vehicle-reference/calibration-supplement-20261025.json',str(OUT/'remaining-targets.json')]
old={r['id']:r for r in json.loads((ART/'before-physics.json').read_text())['cars']}
assert {r['id'] for r in p['cars'] if r!=old[r['id']]}==set(changes)
assert Path('assets/cars/roster.json').read_bytes()==(ART/'before-roster.json').read_bytes()
Path('assets/cars/physics.json').write_text(json.dumps(p,indent=2)+'\n')
(OUT/'profile-changes.json').write_text(json.dumps({'changed':changes,'unchanged_profiles':55,'preserved':['redline-gt500','ref-a08','ref-e12','ref-a14']},indent=2)+'\n')
