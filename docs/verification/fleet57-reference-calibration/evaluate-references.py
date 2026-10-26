"""Rank qualified profile gaps and preserve reasons for deferred calibration."""
import csv
from collections import Counter
import json
from pathlib import Path
OUT = Path('docs/verification/fleet57-reference-calibration')
ART = Path('artifacts/vehicle-reference/fleet57-reference-calibration')
BASE = Path('artifacts/vehicle-reference/fleet57-geometry-drivetrain')
FIELDS = {'zero_60_mph':'zero_60_mph_s','zero_100_mph':'zero_100_mph_s',
          'brake_70_mph_0':'brake_70_mph_0_m','brake_60_mph_0':'brake_60_mph_0_m'}
DEFER = {
    'ref-a16':'GNX instrumented reference is a prototype; no verified production target.',
    'gt4-nsx':'Early US test production status/tire configuration and weather correction unresolved.',
    'redline-911':'Asset market inferred; unladen simulation and reference performance-load basis not established comparable.',
    'ref-e17':'Manufacturer acceleration claim; dry1280kg versus estimated operating1370kg and unknown launch/test basis.',
    'gt4-m3':'No matched CSL tire/circle test; manufacturer acceleration claims remain a qualified envelope.',
}


def read(path):
    return json.loads(Path(path).read_text())


def observed(bench, circles, car, metric, diameter=None):
    if diameter:
        return next(r['peak_sustained_g'] for r in circles if r['car']==car and r['diameter_ft']==diameter)
    return next(r[FIELDS[metric]] for r in bench if r['car']==car)


def compare_primary():
    targets = read(OUT/'handling-targets.json'); rows=[]
    before = read(BASE/'benchmark-r1.json'); after = read(ART/'benchmark-r1.json')
    old_circle = read(BASE/'skidpad-r1.json'); new_circle = read(ART/'skidpad-r1.json')
    for car in targets['cars']:
        for target in car.get('targets',[]):
            metric=target['metric']; diameter=target.get('diameter_ft')
            if metric not in FIELDS and not diameter:
                continue
            prior=observed(before,old_circle,car['car'],metric,diameter)
            actual=observed(after,new_circle,car['car'],metric,diameter)
            reference=target['reference_value']*(.3048 if target['unit']=='ft' else 1)
            use=target.get('use_for_calibration',True) and car['car'] in ['ref-e12','ref-a14']
            rows.append({'car':car['car'],'metric':metric,'diameter_ft':diameter,'before':prior,
                         'after':actual,'reference':reference,'unit':'g' if diameter else 'm' if metric.startswith('brake') else 's',
                         'before_error_pct':100*(prior/reference-1),'after_error_pct':100*(actual/reference-1),
                         'use_for_calibration':use,'source_id':target['source_id'], 'decision':car['decision']})
    return rows


def original_comparisons():
    original=read('docs/verification/fleet57-geometry-drivetrain/reference-comparison.json')
    after={r['car']:r for r in read(ART/'benchmark-r1.json')}; rows=[]
    for old in original['rows']:
        row=old.copy(); actual=after[row['car']][row['metric']]
        row.update(before=row['simulation'],simulation=actual,delta=actual-row['reference'],
                   delta_pct=None if row['upper_bound'] else 100*(actual/row['reference']-1),
                   within_bound=actual<=row['reference'] if row['upper_bound'] else None)
        if row['car'] in DEFER:
            row['calibration_decision']='deferred';row['decision_basis']=DEFER[row['car']]
        else:
            row['calibration_decision']='qualified_comparison';row['decision_basis']=row['reference_status']
        rows.append(row)
    return rows


def write_coverage(original, primary):
    roster={r['id']:r for r in read('assets/cars/roster.json')}
    profiles=read('assets/cars/physics.json')['cars'];rows=[]
    assert len(profiles)==57 and len({r['corpus_reference'] for r in profiles if r['corpus_reference']})==50
    for profile in profiles:
        car=profile['id'];reference=profile['corpus_reference']
        path=Path('docs/vehicle-reference/profiles')/(reference+'.md') if reference else None
        assert path is None or path.is_file()
        evidence=profile['evidence'];mass=evidence['mass_kg']
        statuses=Counter(v['status'] for v in evidence.values())
        rows.append({'car':car,'name':roster[car]['name'],'corpus_reference':reference,
                     'real_profile':str(path) if path else '',
                     'mapping_status':'corpus_mapped' if reference else 'legacy_separate_sources','simulation_mass_kg':profile['parameters']['mass_kg'],
                     'mass_evidence_status':mass['status'],'mass_source':mass.get('source',''),
                     'mass_basis':mass.get('basis',''),
                     'calibration_supplement':'docs/vehicle-reference/calibration-supplement-20261025.json' if car in ['ref-e12','ref-a14'] else '',
                     'estimated_parameter_count':statuses['estimate'],
                     'game_proxy_parameter_count':statuses['game_proxy'],
                     'related_reference_parameter_count':statuses['related_reference'],
                     'original_performance_comparisons':sum(r['car']==car for r in original),
                     'selected_calibration_observations':sum(r['car']==car and r['use_for_calibration'] for r in primary)})
    with (OUT/'profile-coverage.csv').open('w') as stream:
        writer=csv.DictWriter(stream,list(rows[0]));writer.writeheader();writer.writerows(rows)


def main():
    rows=compare_primary(); all_rows=original_comparisons()
    write_coverage(all_rows,rows)
    selected=[r for r in rows if r['use_for_calibration']]
    assert len(selected)==8
    assert all(abs(r['after_error_pct']) < 5 for r in selected)
    score={'targets':len(selected),'before_mean_absolute_error_pct':sum(abs(r['before_error_pct']) for r in selected)/len(selected),
           'after_mean_absolute_error_pct':sum(abs(r['after_error_pct']) for r in selected)/len(selected)}
    result={'primary_rows':rows,'original_qualified_rows':all_rows,'selected_target_score':score,
            'deferred':DEFER,'coverage':{'editions':57,'real_reference_cars':50,'corpus_mapped_editions':51,'unmapped_legacy_editions':6,
                                       'original_performance_comparisons':40,
                                       'original_editions_with_performance_targets':24},
            'limits':'Selected target error is a calibration score, not held-out validation. Exact same independent city scenarios and repetitions remain separately evaluated.'}
    (OUT/'reference-evaluation.json').write_text(json.dumps(result,indent=2)+'\n')
    for name,values in [('primary-comparison',rows),('reference-comparison',all_rows)]:
        with (OUT/(name+'.csv')).open('w') as stream:
            writer=csv.DictWriter(stream,sorted({k for r in values for k in r}));writer.writeheader();writer.writerows(values)
    print(json.dumps(score,indent=2))


if __name__=='__main__':
    main()
