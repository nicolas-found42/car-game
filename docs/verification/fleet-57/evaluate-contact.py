"""Bind the 22B contact-loss observation to controlled environment replays."""
import json
import math
from pathlib import Path

out=Path('docs/verification/fleet-57')
art=Path('artifacts/vehicle-reference/fleet-57')
rows=[json.loads(s) for s in (out/'incremental-results.jsonl').read_text().splitlines()]
actual=next(r for r in rows if r['car']=='ref-j13' and r['case']=='trail-brake-right' and r['repeat']==1)
run=Path(actual['run'])
ready=next(e['fields']['tick'] for e in map(json.loads,(run/'events.jsonl').read_text().splitlines()) if e['kind']=='scenario.ready')
raw=[json.loads(s) for s in (run/'physics.jsonl').read_text().splitlines()]
loss=[r for r in raw if r['tick']>=ready+240 and r['grounded']<4]
assert len(loss)==8 and all(r['grounded']==3 and r['gear']==-1 for r in loss)
first=(art/'contact-probe-r1.json').read_bytes()
assert first==(art/'contact-probe-r2.json').read_bytes()
probe=json.loads(first)
conditions=[]
for row in probe:
    missing=[r['tick'] for r in row['samples'] if r['tick']>=240 and r['grounded']<4]
    expected=8 if row['case']=='trail-brake-right' and row['surface'] in ['wall','both'] else 0
    assert len(missing)==expected
    conditions.append({k:v for k,v in row.items() if k!='samples'}|{'contact_loss_ticks':missing})
combined=next(r for r in probe if r['case']=='trail-brake-right' and r['surface']=='both')
assert combined['four_contact_pct']==actual['four_contact_pct']
assert [r['tick']-ready for r in loss]==next(r['contact_loss_ticks'] for r in conditions if r['case']=='trail-brake-right' and r['surface']=='both')
by={r['tick']-ready:r for r in raw}
# Production logs pose before Rapier SyncBackend. The fixture reads after app.update.
diffs=[math.dist(r['position'],by[r['tick']+1]['position']) for r in combined['samples'] if r['tick']>=240 and r['tick']+1 in by]
assert max(diffs)<.001
result={'car':'ref-j13','case':'trail-brake-right','source_run':actual['run'],'repeated_primary_observation':next(r['trajectory_sha256'] for r in rows if r['car']=='ref-j13' and r['case']=='trail-brake-right' and r['repeat']==2)==actual['trajectory_sha256'],
        'lost_ray_contact_ticks':8,'duration_seconds':8/120,'relative_tick_interval':[loss[0]['tick']-ready,loss[-1]['tick']-ready],
        'minimum_contacts':3,'fully_airborne_ticks':0,'missing_wheel':'rear_right',
        'speed_during_loss_mps':[min(math.hypot(r['linear_velocity'][0],r['linear_velocity'][2]) for r in loss),max(math.hypot(r['linear_velocity'][0],r['linear_velocity'][2]) for r in loss)],
        'controls_during_loss':loss[0]['input'],'gear_during_loss':-1,
        'position_ranges_during_loss_m':[[min(r['position'][i] for r in loss),max(r['position'][i] for r in loss)] for i in range(3)],
        'conditions':conditions,'probe_repeats_byte_identical':True,'pose_sample_epoch_alignment_ticks':1,'maximum_pose_error_after_declared_epoch_alignment_m':max(diffs),
        'interpretation':'The boundary wall is sufficient to reproduce the brief loss and removing it eliminates the loss in the controlled fixture. The curb alone does not cause it. This is a brake-held reverse/wall interaction, not evidence that the AWD car generally lacks tire grip. Collision realism and tire fidelity remain unvalidated.'}
(out/'contact-investigation.json').write_text(json.dumps(result,indent=2)+'\n')
print(result['interpretation'])
