#!/usr/bin/env python3
"""Audit the complete expanded matrix and summarize actual production traces."""
import argparse
from collections import defaultdict
import csv
import hashlib
import json
import math
from pathlib import Path
import statistics

from analyze_profile_study import same_json
from fleet40_protocol import ROOT, ALL_CASES, ROUTES, scenario
from handling_protocol import CASES
from analyze_handling_study import check_inputs
from provenance_bundle import captured_entry
from recorder_health import inspect_health
from run_handling_study import sha


def read(path):
    return json.loads(path.read_text())


def lines(path):
    with path.open() as source:
        for line in source:
            yield json.loads(line)


def event_checks(run, car, profile):
    reset_reasons=[]; applied=[]; ready=[]
    for event in lines(run/'events.jsonl'):
        if event['kind']=='vehicle.physics_profile' and event['fields']['id']==car:
            applied.append(same_json(event['fields'],profile))
        if event['kind']=='vehicle.reset':reset_reasons.append(event['fields']['reason'])
        if event['kind']=='scenario.ready':ready.append(event['fields']['tick'])
    assert applied and all(applied),'Compiled profile mismatch'
    assert len(ready)==1,'Readiness boundary missing or repeated'
    return ready[0],reset_reasons


def same_configuration(first, second):
    if isinstance(first,dict) and isinstance(second,dict):
        return first.keys()==second.keys() and all(same_configuration(first[k],second[k]) for k in first)
    if isinstance(first,list) and isinstance(second,list):
        return len(first)==len(second) and all(same_configuration(a,b) for a,b in zip(first,second))
    if isinstance(first,(int,float)) and isinstance(second,(int,float)):
        return math.isclose(first,second,rel_tol=1e-7,abs_tol=1e-7)
    return first==second


def compiled_definition(value):
    result=json.loads(json.dumps(value));result.setdefault('context',{'kind':'sandbox'})
    for command in result['commands']:
        command.setdefault('tick',None);command.setdefault('elapsed_ms',None)
        if command['command']['action']=='drive':command['command'].setdefault('handbrake',False)
    return result


def check_timed_inputs(consumed, task, ready):
    timeline={c['tick']:c['command'] for c in scenario(task['car'],task['case'])['commands']}
    held={'throttle':0.,'brake':0.,'steer':0.,'handbrake':False}
    for row in consumed:
        tick=row['tick']-ready;command=timeline.get(tick,{})
        if command.get('action')=='drive':
            held={key:command.get(key,False if key=='handbrake' else 0.) for key in held}
        expected={**held,'reset':command.get('action')=='reset'}
        assert all(math.isclose(row[key],value,rel_tol=1e-7,abs_tol=1e-7) for key,value in expected.items()),(task,tick)


def powered_forward(row):
    controls=row['input']
    return row['speed']>1. and controls['throttle']>.05 and controls['brake']==0. and not controls['handbrake']


def check_axles(samples, profile):
    acceleration=list(filter(powered_forward,samples))
    observed=[any(sum(row[field][start:start+2])>1. for row in acceleration)
              for field in ['drive_demands','longitudinal_forces'] for start in [0,2]]
    front=profile['parameters']['front_drive'];expected=[front>0.,front<1.]
    assert acceleration and observed==expected+expected,'Driven axle mismatch'
    return {'front_drive_observed':observed[2],'rear_drive_observed':observed[3]}


def checked_physics(run, ready, car, case, ticks):
    data=list(lines(run/'physics.jsonl')); assert data and all(r['car']==car for r in data)
    represented={r['tick']-ready for r in data if r['tick']>ready}
    assert len(data)==len({r['tick'] for r in data}),'Duplicate physics tick'
    missing={1680} if case=='brake' else set()
    assert set(range(1,ticks+1))-represented==missing,'Missing physics samples'
    assert max(represented)<=ticks,'Physics advanced beyond scenario bound'
    return data


def trace_metrics(run, ready, car, profile, case, ticks):
    data=checked_physics(run,ready,car,case,ticks)
    end=1680 if case=='brake' else float('inf')
    moving=[r for r in data if 240<=r['tick']-ready<end]
    tilt=lambda r:math.degrees(math.acos(max(-1.,min(1.,1.-2.*(r['rotation'][0]**2+r['rotation'][2]**2)))))
    speed=lambda r:math.hypot(r['linear_velocity'][0],r['linear_velocity'][2])
    loaded=lambda r:sum(v>1. for v in r['wheel_loads'])
    axles=check_axles(moving,profile)
    canonical=[[r['tick']-ready,r['position'],r['rotation'],r['linear_velocity'],r['grounded']] for r in moving]
    return {'samples':len(moving),'four_contact_pct':100*sum(r['grounded']==4 for r in moving)/len(moving),
            'minimum_contacts':min(r['grounded'] for r in moving),'airborne_ticks':sum(r['grounded']==0 for r in moving),
            'four_loaded_pct':100*sum(loaded(r)==4 for r in moving)/len(moving),
            'peak_tilt_deg':max(map(tilt,moving)),'peak_slip_deg':max(abs(r['slip']) for r in moving)*180/math.pi,
            'peak_speed_mps':max(map(speed,moving)),'final_speed_mps':speed(moving[-1]),
            **axles,
            'trajectory_sha256':hashlib.sha256(json.dumps(canonical,separators=(',',':')).encode()).hexdigest()}


def analyze(task, study, profiles):
    run=ROOT/task['run']; summary=read(run/'scenario-summary.json'); manifest=read(run/'manifest.json')
    definition=scenario(task['car'],task['case'])
    assert summary['assertions_executed']==len(definition['assertions'])
    assert summary['ticks']==definition['max_ticks']-1
    assert sha(ROOT/task['scenario'])==task['scenario_sha256']
    assert read(ROOT/task['scenario'])==scenario(task['car'],task['case'])
    assert manifest['binary_sha256']==study['binary_sha256']
    assert inspect_health(run,manifest)['ok'],'Recorder was unhealthy'
    frozen=read(run/'scenario.json'); assert same_configuration(frozen['definition'],compiled_definition(read(ROOT/task['scenario'])))
    assert frozen['render_backend']=='disabled'
    _,config_path=captured_entry(run,role='effective_config')
    config=read(config_path)['settings'];assert config['fast_scenario'] and config['fixed_hz']==120
    assert config['car']==task['car'] and config['seed']==17
    ready,resets=event_checks(run,task['car'],profiles[task['car']])
    inputs=list(lines(run/'input.jsonl'))
    consumed=[r for r in inputs if ready<r['tick']<=ready+summary['ticks']]
    assert len(consumed)==len({r['tick'] for r in consumed})==summary['ticks']
    assert all(r['controller']=='scenario' for r in consumed)
    if task['case'] not in ROUTES:
        check_timed_inputs(consumed,task,ready)
    if task['case'] in CASES:
        input_sha=check_inputs(inputs,task,ready)
    else:
        input_sha=sha(run/'input.jsonl')
    route=[r['observed'] for r in summary['results'] if r['assertion']['kind']=='route_completed']
    result={**task,'scenario_passed':summary['passed'],'ticks':summary['ticks'],
            'assertions_executed':summary['assertions_executed'],'input_sha256':input_sha,
            'unexpected_resets':sum(r!='requested-reset' for r in resets),
            'requested_resets':resets.count('requested-reset'),**trace_metrics(run,ready,task['car'],profiles[task['car']],task['case'],summary['ticks'])}
    if route:result.update(route[0]); result['route_passed']=route[0]['passed']
    return result


def aggregate(rows):
    groups=defaultdict(list)
    for row in rows:groups[row['car'],row['case']].append(row)
    result=[]
    for (car,case),items in sorted(groups.items()):
        entry={'car':car,'case':case,'runs':len(items),'scenario_passed':all(r['scenario_passed'] for r in items),
               'identical_trajectories':len({r['trajectory_sha256'] for r in items})==1}
        metrics=['peak_speed_mps','final_speed_mps','peak_tilt_deg','peak_slip_deg','four_contact_pct',
                 'four_loaded_pct','maximum_cross_track_m','progress_m','total_m','unexpected_resets']
        for name in metrics:
            values=[r[name] for r in items if name in r]
            if values:entry[name]=statistics.mean(values);entry[name+'_range']=[min(values),max(values)]
        result.append(entry)
    return result


def save(out, study, rows):
    out.mkdir(parents=True,exist_ok=True)
    result={'schema':1,'study':study,'runs':rows,'aggregate':aggregate(rows),
            'scenario_successes':sum(r['scenario_passed'] for r in rows),'complete_runs':len(rows),
            'unexpected_resets':sum(r['unexpected_resets'] for r in rows),
            'source_profile_sha256':sha(ROOT/'assets/cars/physics.json')}
    (out/'results.json').write_text(json.dumps(result,indent=2)+'\n')
    scalar=[{k:v for k,v in r.items() if not isinstance(v,(list,dict))} for r in result['aggregate']]
    with (out/'results.csv').open('w') as stream:
        fields=sorted({k for r in scalar for k in r});writer=csv.DictWriter(stream,fields)
        writer.writeheader();writer.writerows(scalar)
    print(json.dumps({k:result[k] for k in ['complete_runs','scenario_successes','unexpected_resets']}))


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('study',type=Path);parser.add_argument('--output',type=Path,required=True)
    parser.add_argument('--index',type=Path,help='Explicit selected-attempt index; defaults to the raw run index')
    args=parser.parse_args();config=read(args.study/'study.json')
    index=args.index or args.study/'runs.jsonl'
    tasks=list(lines(index)); roster=read(ROOT/'assets/cars/roster.json')
    config['run_index']=str(index);config['run_index_sha256']=sha(index)
    expected={(c['id'],case,r) for c in roster for case in ALL_CASES for r in range(1,config['repeats']+1)}
    assert set(config['cars'])=={c['id'] for c in roster}
    assert len(tasks)==len(expected)
    assert {(r['car'],r['case'],r['repeat']) for r in tasks}==expected
    assert config['profile_sha256']==sha(ROOT/'assets/cars/physics.json')
    assert config['roster_sha256']==sha(ROOT/'assets/cars/roster.json')
    assert config['courses_sha256']==sha(ROOT/'docs/verification/fleet-40/courses.json')
    assert config['cases']==ALL_CASES and config['repeats']==2
    profiles={p['id']:p for p in read(ROOT/'assets/cars/physics.json')['cars']}
    rows=[analyze(task,config,profiles) for task in tasks]
    save(args.output,config,rows)


if __name__=='__main__':main()
