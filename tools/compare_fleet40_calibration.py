#!/usr/bin/env python3
"""Compare frozen before/after production measurements with qualified reference envelopes."""
import argparse
import csv
import json
from pathlib import Path
import statistics

ROOT = Path(__file__).resolve().parents[1]


def read(path):
    return json.loads(path.read_text())


def performance(out, measurements):
    observed = {r['car']: r for r in measurements}
    result = []
    for row in read(out/'before-comparison.json')['rows']:
        actual = observed[row['car']][row['metric']]
        result.append({**row, 'before': row['simulation'], 'after': actual,
                       'before_delta_pct': row['delta_pct'],
                       'after_delta_pct': None if row['upper_bound'] else 100*(actual/row['reference']-1),
                       'after_within_bound': actual <= row['reference'] if row['upper_bound'] else None})
    return result


def handling(out, artifacts):
    before = {(r['car'],r['diameter_ft']):r for r in read(artifacts/'before-skidpad.json')}
    after = {(r['car'],r['diameter_ft']):r for r in read(artifacts/'after-skidpad-r1.json')}
    result = []
    for ref in read(out/'handling-targets.json'):
        key = ref['car'],ref['diameter_ft']
        first,last = before[key]['peak_sustained_g'],after[key]['peak_sustained_g']
        result.append({**ref,'before_g':first,'after_g':last,
                       'before_delta_pct':100*(first/ref['reference_g']-1),
                       'after_delta_pct':100*(last/ref['reference_g']-1)})
    return result


def changes(out):
    before = {r['id']:r for r in read(out/'before-profiles.json')['cars']}
    result = []
    for row in read(out/'after-profiles.json')['cars']:
        edits = {k:{'before':before[row['id']]['parameters'].get(k),'after':v,'evidence':row['evidence'][k]}
                 for k,v in row['parameters'].items()
                 if k != 'lateral_mu' and before[row['id']]['parameters'].get(k) != v}
        if row['parameters']['lateral_mu'] != row['parameters']['tire_mu']:
            edits['lateral_mu'] = {'before':before[row['id']]['parameters']['tire_mu'],
                                   'after':row['parameters']['lateral_mu'],'evidence':row['evidence']['lateral_mu']}
        if edits: result.append({'car':row['id'],'changes':edits})
    return result


def envelope_metric(fact):
    if 'top_speed' in fact['field'] and isinstance(fact['value'],(int,float)):
        return 'top_speed_kmh'
    return {'acceleration_0_200_kmh':'zero_200_kmh_s',
            'acceleration_0_125_mph':'zero_125_mph_s'}.get(fact['field'])


def envelope_row(profile, car, fact, before, after, metric):
    field=fact['field'];source=next(s for s in car['sources'] if s['id']==fact['source_id'])
    value=fact['value']*(1.609344 if metric=='top_speed_kmh' and fact['unit']=='mph' else 1.)
    first,last=before[profile['id']][metric],after[profile['id']][metric]
    bound='lower' if 'lower_bound' in field else None
    eligible=field!='reference_2004US_top_speed'
    return {'car':profile['id'],'metric':metric,'reference':value,'before':first,'after':last,
            'source_url':source['url'],'source_id':source['id'],'source_field':field,
            'note':fact.get('note',''),'identity':car['identity_status'],'bound':bound,'eligible':eligible,
            'eligibility_note':'Source-qualified comparison; read identity and test conditions' if eligible else 'Alternate US market speed limit does not apply to selected European GT4 reference',
            'before_delta_pct':100*(first/value-1) if first is not None else None,
            'after_delta_pct':100*(last/value-1) if last is not None else None,
            'after_within_bound':last>=value if bound and last is not None else None}


def envelopes(out, artifacts):
    corpus={c['id']:c for c in read(ROOT/'docs/vehicle-reference/real-profiles.json')['cars']}
    profiles=read(out/'after-profiles.json')['cars']
    before={r['car']:r for r in read(artifacts/'before-envelope.json')}
    after={r['car']:r for r in read(artifacts/'after-envelope-r1.json')}
    rows=[]
    for profile in profiles:
        car=corpus.get(profile['corpus_reference'])
        if not car:continue
        for fact in car['facts']:
            metric=envelope_metric(fact)
            if metric:rows.append(envelope_row(profile,car,fact,before,after,metric))
    selected=next(p for p in profiles if p['id']=='redline-gt500')['evidence']['speed_governor_mps']
    rows.append({'car':'redline-gt500','metric':'top_speed_kmh','reference':155*1.609344,
                 'before':before['redline-gt500']['top_speed_kmh'],'after':after['redline-gt500']['top_speed_kmh'],
                 'before_delta_pct':100*(before['redline-gt500']['top_speed_kmh']/(155*1.609344)-1),
                 'after_delta_pct':100*(after['redline-gt500']['top_speed_kmh']/(155*1.609344)-1),
                 'source_url':selected['source'],'note':selected['basis'],'eligible':True,'bound':None})
    rows.append({'car':'redline-gt500','metric':'zero_100_mph_s','reference':9.7,
                 'before':before['redline-gt500']['zero_100_mph_s'],'after':after['redline-gt500']['zero_100_mph_s'],
                 'before_delta_pct':100*(before['redline-gt500']['zero_100_mph_s']/9.7-1),
                 'after_delta_pct':100*(after['redline-gt500']['zero_100_mph_s']/9.7-1),
                 'source_url':'https://www.motortrend.com/reviews/2008-dodge-challenger-srt8-vs-ford-shelby-gt500-comparison-test-archive',
                 'note':'Same2008 coupe period test, additional point not used as a fit target; rollout unspecified.',
                 'eligible':True,'bound':None,'fit_target':False})
    return rows


def coverage_counts(result, car):
    performance=sum(r['car']==car and r.get('eligible',True) for r in result['performance']+result['envelopes'])
    handling=sum(r['car']==car for r in result['handling'])
    return {'performance_reference_rows':performance,'handling_reference_rows':handling}


def comparison_table(out, artifacts, result):
    roster=read(out/'roster.json');profiles={p['id']:p for p in read(out/'after-profiles.json')['cars']}
    phases={phase:{r['car']:r for r in read(artifacts/file)}
            for phase,file in [('before','before-envelope.json'),('after','after-envelope-r1.json')]}
    skidpads={phase:{(r['car'],r['diameter_ft']):r for r in read(artifacts/file)}
              for phase,file in [('before','before-skidpad.json'),('after','after-skidpad-r1.json')]}
    changed={r['car'] for r in result['profile_changes']};rows=[]
    for car in roster:
        key=car['id'];profile=profiles[key]
        row={'car':key,'name':car['name'],'model_game':car['game'],
             'corpus_reference':profile['corpus_reference'],'physical_profile_changed':key in changed,
             'governor_mps':profile['parameters']['speed_governor_mps']}
        for phase,values in phases.items():
            row.update({phase+'_'+k:v for k,v in values[key].items() if k!='car'})
        for phase,values in skidpads.items():
            row.update({f'{phase}_skidpad_{diameter}ft_g':values[key,diameter]['peak_sustained_g'] for diameter in [200,300]})
        row.update(coverage_counts(result,key));rows.append(row)
    with (out/'all-40-comparison.csv').open('w') as stream:
        writer=csv.DictWriter(stream,list(rows[0]));writer.writeheader();writer.writerows(rows)


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output',type=Path,default=ROOT/'docs/verification/fleet-40-calibration')
    parser.add_argument('--artifacts',type=Path,default=ROOT/'artifacts/vehicle-reference/fleet-40-calibration')
    args=parser.parse_args();out,artifacts=args.output,args.artifacts
    measurements=read(artifacts/'after-benchmark-r1.json')
    assert measurements==read(artifacts/'after-benchmark-r2.json') and len(measurements)==40
    assert read(artifacts/'after-skidpad-r1.json')==read(artifacts/'after-skidpad-r2.json')
    assert read(artifacts/'after-envelope-r1.json')==read(artifacts/'after-envelope-r2.json')
    perf=performance(out,measurements);corners=handling(out,artifacts)
    result={'performance':perf,'handling':corners,'envelopes':envelopes(out,artifacts),'profile_changes':changes(out),
            'all_40_measured':True,'skidpad_sweeps':80,'benchmark_repeats_identical':True,
            'skidpad_repeats_identical':True,'calibration_is_not_independent_validation':True,
            'performance_mean_absolute_delta_pct':{phase:statistics.mean(abs(r[phase+'_delta_pct']) for r in perf if r[phase+'_delta_pct'] is not None) for phase in ['before','after']}}
    (out/'reference-deltas.json').write_text(json.dumps(result,indent=2)+'\n')
    comparison_table(out,artifacts,result)
    print(json.dumps(result['performance_mean_absolute_delta_pct']))


if __name__=='__main__':main()
