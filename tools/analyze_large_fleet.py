"""Validate full common-matrix coverage and rank observed stability/performance risks."""
import argparse
from collections import Counter, defaultdict
import csv
import gzip
import hashlib
import json
from pathlib import Path
import statistics


def read(path):
    return json.loads(Path(path).read_text())


def sha(path):
    with Path(path).open('rb') as stream:return hashlib.file_digest(stream,'sha256').hexdigest()


def load_stage(root, car, stage, bounds=None):
    receipt=read(root/car/(stage+'.receipt.json'));path=Path(receipt['result'])
    assert sha(path)==receipt['sha256'] and receipt['exit_code']==0
    with gzip.open(path,'rt') as stream:rows=[json.loads(line) for line in stream]
    expected=set(range(*(bounds or (0,5000)))) if stage=='stress' else {r['case'] for r in read(root/'city-definitions.json')}
    assert len(rows)==len(expected) and {r['case'] for r in rows}==expected
    assert all(r['car']==car and r['repeats']==2 and len(r['repeat_hashes'])==2 for r in rows)
    assert receipt['pairs']==len(rows) and receipt['executions']==len(rows)*2
    return rows,receipt


def family_summary(car, family, rows):
    m=[r['metrics'] for r in rows]
    stops=[r['first_stop_distance_m'] for r in m if r['first_stop_distance_m'] is not None]
    failures=Counter(reason for row in rows for reason in set(row['failures'][0]+row['failures'][1]))
    return {'car':car,'family':family,'pairs':len(rows),'executions':2*len(rows),
            'failed_pairs':sum(any(r['failures']) for r in rows),
            'different_repeat_pairs':sum(r['repeat_hashes'][0]!=r['repeat_hashes'][1] for r in rows),
            'maximum_tilt_deg':max(r['peak_tilt_deg'] for r in m),
            'median_peak_tilt_deg':statistics.median(r['peak_tilt_deg'] for r in m),
            'maximum_slip_mps':max(r['peak_slip_mps'] for r in m),
            'maximum_speed_mps':max(r['maximum_speed_mps'] for r in m),
            'minimum_four_contact_fraction':min(r['four_contact_ticks']/r['ticks'] for r in m),
            'stops_observed':len(stops),'median_observed_stop_distance_m':statistics.median(stops) if stops else None,
            'failure_reasons':dict(failures)}


def audit_traces(root, car, rows):
    index=read(root/car/'failure-traces.json');mapping={r['original_path']:r for r in index}
    expected={path for r in rows for path in r.get('traces',[])}
    assert mapping.keys()==expected
    for row in index:
        assert sha(row['path'])==row['sha256']
        with gzip.open(row['path'],'rb') as stream:
            assert hashlib.file_digest(stream,'sha256').hexdigest()==row['raw_sha256']
    for row in rows:
        unequal=row['repeat_hashes'][0]!=row['repeat_hashes'][1]
        failed=any(row['failures']) if 'failures' in row else not row['summary']['passed'] or not row['repeat_summary_equal']
        if failed or unequal:
            assert len(row['traces'])==1+int(unequal),(car,row['case'])
            assert mapping[row['traces'][0]]['raw_sha256']==row['repeat_hashes'][0]
            if unequal:assert mapping[row['traces'][1]]['raw_sha256']==row['repeat_hashes'][1]
    return len(index)


def failed_pair(row):
    unequal=row['repeat_hashes'][0]!=row['repeat_hashes'][1]
    if 'failures' in row:
        return any(row['failures']) or unequal
    return not row['summary']['passed'] or not row['repeat_summary_equal'] or unequal


def flagged(rows):
    return [r for r in rows if failed_pair(r)]


def repaired_rows(root, car, city, stress, revised):
    if root is None or not (root/car/'complete.json').exists():
        return city,stress,0,None
    before={'failed_city_pairs':len(flagged(city)), 'failed_stress_pairs':len(flagged(stress))}
    fixed_city,_=load_stage(root,car,'city');fixed_stress,_=load_stage(root,car,'stress')
    if revised is not None:
        fixed={r['case']:r['repeat_hashes'] for r in fixed_stress if r['family']=='slope'}
        assert fixed=={r['case']:r['repeat_hashes'] for r in revised}
    return fixed_city,fixed_stress,audit_traces(root,car,fixed_city+fixed_stress),before


def analyze_car(root, car, slopes=None, repairs=None):
    city,city_receipt=load_stage(root,car,'city');stress,stress_receipt=load_stage(root,car,'stress')
    traces=audit_traces(root,car,city+stress)
    original_slope=[r for r in stress if r['family']=='slope'];revised=None
    if slopes:
        revised,_=load_stage(slopes,car,'stress',(4000,4500))
        traces+=audit_traces(slopes,car,revised)
        stress=[r for r in stress if r['family']!='slope']+revised
    city,stress,extra_traces,repair_before=repaired_rows(repairs,car,city,stress,revised)
    traces+=extra_traces
    groups=defaultdict(list)
    for row in stress:groups[row['family']].append(row)
    assert len(groups)==10 and all(len(rows)==500 for rows in groups.values())
    families=[family_summary(car,family,rows) for family,rows in groups.items()]
    failed=flagged(stress)
    routes=flagged(city)
    selected={r['case']:r['metrics'] for r in stress if r['case'] in [49,545,1549,2499,2999,3499,3999,4499,4999]}
    return {'car':car,'families':families,'failed_stress_pairs':failed,'failed_city_pairs':routes,
            'selected_common_cases':selected,'executions':city_receipt['executions']+stress_receipt['executions'],
            'failure_trace_files':traces,
            'original_slope_before':family_summary(car,'slope',original_slope),'profile_repair_before':repair_before}


def write_csv(path, rows):
    with path.open('w') as sink:
        writer=csv.DictWriter(sink,sorted({k for r in rows for k in r}));writer.writeheader();writer.writerows(rows)


def summarize(results,families):
    summary={'cars':300,'original_cases_per_car':24,'new_cases_per_car':5000,'repeats':2,
             'executions':sum(r['executions'] for r in results),
             'failed_stress_pairs':sum(len(r['failed_stress_pairs']) for r in results),
             'failed_city_pairs':sum(len(r['failed_city_pairs']) for r in results),
             'different_stress_repeat_pairs':sum(r['different_repeat_pairs'] for r in families),
             'failure_reasons':dict(sum((Counter(r['failure_reasons']) for r in families),Counter())),
             'family_metrics':families,'car_results':results}
    return summary


def selected_csv(results):
    return [{'car':c['car'],'case':case,**metrics} for c in results
            for case,metrics in c['selected_common_cases'].items()]


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--study',type=Path,required=True);parser.add_argument('--output',type=Path,required=True)
    parser.add_argument('--source',type=Path,required=True)
    parser.add_argument('--slope-study',type=Path)
    parser.add_argument('--profile-study',type=Path);args=parser.parse_args()
    root=args.study.resolve();out=args.output;out.mkdir(parents=True,exist_ok=True)
    method=read(root/'method.json');roster=read(args.source/'assets/cars/roster.json')
    assert len(roster)==len(method['cars'])==300 and [r['id'] for r in roster]==method['cars']
    assert sha(args.source/'assets/cars/roster.json')==method['roster_sha256']
    results=[]
    for i,car in enumerate(method['cars'],1):
        results.append(analyze_car(root,car,args.slope_study,args.profile_study))
        if i%10==0:print('Validated',i,'/300',flush=True)
    families=[r for c in results for r in c['families']]
    summary=summarize(results,families)
    summary['slope_revision_applied']=bool(args.slope_study)
    summary['additional_slope_executions']=300000 if args.slope_study else 0
    summary['additional_profile_executions']=10048 if args.profile_study else 0
    assert summary['executions']==3014400
    (out/'evaluation.json').write_text(json.dumps(summary,indent=2)+'\n')
    write_csv(out/'car-family-metrics.csv',[{k:v for k,v in r.items() if k!='failure_reasons'} for r in families])
    selected=selected_csv(results)
    write_csv(out/'selected-common-case-metrics.csv',selected)
    print(json.dumps({k:v for k,v in summary.items() if k not in ['family_metrics','car_results']},indent=2))


if __name__=='__main__':main()
