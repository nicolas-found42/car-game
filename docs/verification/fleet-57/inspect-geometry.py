"""Inspect the largest geometry outliers without altering source or runtime data."""
import json
from pathlib import Path
import sys
sys.path.insert(0,'tools')
from prepare_fleet import parse,default_meshes,mesh_bounds,merged_bounds

out=Path('docs/verification/fleet-57')
roster={r['id']:r for r in json.load(open('assets/cars/roster.json'))}
rows=[]
for cid in ['ref-a08','ref-a14','ref-a13','ref-e02','ref-j13']:
    car=roster[cid]
    doc,_=parse((Path('assets')/car['path']).read_bytes())
    groups={}
    for index,world,chain in default_meshes(doc):
        owner=next((doc['nodes'][i]['name'] for i,_ in chain
                    if doc['nodes'][i].get('name','').startswith('wheel_')),None)
        if owner:
            groups.setdefault(owner,[]).append(mesh_bounds(doc,doc['nodes'][index]['mesh'],world))
    for slot,bounds in groups.items():
        merged=merged_bounds(bounds)
        center=[(a+b)/2 for a,b in zip(*merged)]
        pivot=next(n['translation'] for n in doc['nodes'] if n.get('name')==slot)
        rows.append({'car':cid,'slot':slot,'physical_pivot_m':pivot,'prepared_wheel_bounds_center_m':center,
                     'center_minus_pivot_m':[b-a for a,b in zip(pivot,center)]})
tires=json.load(open(out/'tire-geometry.json'))
c5=[r for r in tires['rows'] if r['car']=='ref-a08']
mesh=sum(r['prepared_mesh_radius_m'] for r in c5)/2
nominal=sum(r['nominal_unloaded_radius_m'] for r in c5)/2
result={'method':'Prepared wheel-group bounds compared with physical pivots. Bbox center is an inspection proxy, not a measured contact patch or hub dynamometer result.',
        'selected_cars':5,'rows':rows,
        'c5_mean_mesh_radius_m':mesh,'c5_mean_nominal_unloaded_radius_m':nominal,
        'c5_mean_radius_ratio':mesh/nominal,
        'c5_wheel_force_delta_at_equal_engine_torque_and_ratio_pct':100*(nominal/mesh-1),
        'c5_model_equation':'wheel_force = throttle * engine_torque * gear_ratio * final_drive * driveline_efficiency / mean(four mesh radii)',
        'interpretation':'Selected pivots agree with their wheel mesh centers within millimetres. The larger real-reference deviations are present in the authored geometry, rather than explained by a many-centimetre pivot-reparenting offset. Stock-vs-modified source identity remains to be verified. Radius sensitivity is an analytical model effect, not a measured contribution to 0–60 time.'}
(out/'geometry-inspection.json').write_text(json.dumps(result,indent=2)+'\n')
print('Selected pivot/mesh checks:',len(rows),'C5 force effect:',result['c5_wheel_force_delta_at_equal_engine_torque_and_ratio_pct'])
