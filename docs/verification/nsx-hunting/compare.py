"""Compare every common case with the accepted slope/profile-corrected baseline."""
import gzip
import hashlib
import json
from pathlib import Path
import sys

ROOT=Path(__file__).resolve().parents[3]
sys.path.insert(0,str(ROOT/'tools'))
from analyze_large_fleet import load_stage, audit_traces, flagged
ART=ROOT/'artifacts/nsx-gear-hunting-fix'
INITIAL=ROOT/'artifacts/fleet300-5024-20261025'
SLOPES=ROOT/'artifacts/fleet300-slopes-20261025'
REPAIR=ROOT/'artifacts/fleet300-profile-repair-20261025'
PROTECTED=['redline-gt500','ref-a08','ref-e12','ref-a14']


def baseline(car):
    if car=='exp-161':
        return {s:load_stage(REPAIR,car,s)[0] for s in ['city','stress']}
    stages={s:load_stage(INITIAL,car,s)[0] for s in ['city','stress']}
    revised=load_stage(SLOPES,car,'stress',(4000,4500))[0]
    stages['stress']=[r for r in stages['stress'] if r['family']!='slope']+revised
    return stages


def comparable(row):
    result={k:v for k,v in row.items() if k!='traces'}
    if 'summary' in result:
        result['summary']={k:v for k,v in result['summary'].items() if k not in ['elapsed_ms','artifacts']}
    return result


def launch_metrics(rows):
    return next(r for r in rows if r['case']==49)['metrics']


def compare_car(car):
    old=baseline(car);new={s:load_stage(ART/'matrix',car,s)[0] for s in old}
    audit_traces(ART/'matrix',car,new['city']+new['stress'])
    result={'car':car,'stages':{}}
    for stage in old:
        before={r['case']:r for r in old[stage]};after={r['case']:r for r in new[stage]}
        assert before.keys()==after.keys()
        changed=[c for c in before if comparable(before[c])!=comparable(after[c])]
        was_failed={r['case'] for r in flagged(old[stage])};now_failed={r['case'] for r in flagged(new[stage])}
        assert all(r['repeat_hashes'][0]==r['repeat_hashes'][1] for r in new[stage])
        result['stages'][stage]={'pairs':len(before),'changed_cases':changed,
            'newly_failed_cases':sorted(now_failed-was_failed),'resolved_cases':sorted(was_failed-now_failed),
            'failed_before':len(was_failed),'failed_after':len(now_failed)}
        if car in PROTECTED:assert not changed,(car,stage,changed[:10])
    if car=='exp-231':
        result['launch_case_49']={name:launch_metrics(rows['stress']) for name,rows in [('before',old),('after',new)]}
    return result


def changed_cars(results):
    return sorted(r['car'] for r in results if any(s['changed_cases'] for s in r['stages'].values()))


def main():
    roster=json.loads((ROOT/'assets/cars/roster.json').read_text())
    results=[]
    for i,row in enumerate(roster,1):
        results.append(compare_car(row['id']))
        if i%25==0:print(f'Compared {i}/300 cars',flush=True)
    output={'status':'passed','cars':300,'pairs':sum(s['pairs'] for r in results for s in r['stages'].values()),
            'protected_all_5024_cases_exact':PROTECTED,'results':results}
    affected={r['car'] for r in json.loads((ART/'threshold-audit.json').read_text())['affected']}
    output['changed_cars']=changed_cars(results)
    assert set(output['changed_cars'])<=affected,output['changed_cars']
    output['unaffected_cars_all_case_pairs_exact']=300-len(affected)
    assert output['pairs']==1507200
    (ART/'comparison.json').write_text(json.dumps(output,indent=2)+'\n')
    print('PASS: every common case compared; protected four exact',flush=True)


if __name__=='__main__':main()
