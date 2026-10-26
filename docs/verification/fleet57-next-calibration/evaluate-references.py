"""Evaluate qualified period observations without erasing configuration uncertainty."""
import csv
import json
from pathlib import Path
OUT=Path('docs/verification/fleet57-next-calibration')
ART=Path('artifacts/vehicle-reference/fleet57-next-calibration')
BASE=Path('artifacts/vehicle-reference/fleet57-reference-calibration')


def read(path):
    return json.loads(path.read_text())


def main():
    before={r['car']:r for r in read(BASE/'benchmark-r1.json')}
    after={r['car']:r for r in read(ART/'benchmark-r1.json')}
    circles={(r['car'],r['diameter_ft']):r for r in read(ART/'skidpad-r1.json')}
    rows=[]
    for metric,target in [('zero_60_mph_s',5.4),('zero_100_mph_s',13.9),('sixty_to_hundred_mph_s',8.5)]:
        values=[]
        for mapping in [before,after]:
            r=mapping['350z'];values.append(r[metric] if metric in r else r['zero_100_mph_s']-r['zero_60_mph_s'])
        rows.append({'car':'350z','metric':metric,'before':values[0],'after':values[1],
                     'reference':target,'source':'z-cd-2003','unit':'s',
                     'before_error_pct':(values[0]/target-1)*100,'after_error_pct':(values[1]/target-1)*100})
    decision=read(OUT/'remaining-targets.json')
    result={'primary_rows':rows,'qualification':'Selected related Track6MT equipment; rollout, driver load and exact source game grade unresolved. Calibration observations are not held-out validation.',
            'nsx':{'measured_period_range_g':[.86,.93],'range_is_matched_circle':False,
                   'simulation_200ft_g':circles['gt4-nsx',200]['peak_sustained_g'],
                   'simulation_300ft_g':circles['gt4-nsx',300]['peak_sustained_g'],
                   'policy':'Conservative model envelope 0.84–0.95 g includes protocol/build uncertainty. Do not claim the .86–.93 range is an exact matched-circle target.'},
            'decisions':decision['cars'],'nominal_engine_outputs_unchanged':True,
            'mean_absolute_error_selected_350z_observations_before_pct':sum(abs(r['before_error_pct']) for r in rows)/3,
            'mean_absolute_error_selected_350z_observations_after_pct':sum(abs(r['after_error_pct']) for r in rows)/3}
    assert all(abs(r['after_error_pct'])<5 for r in rows)
    assert .84<=result['nsx']['simulation_300ft_g']<=.95
    (OUT/'reference-evaluation.json').write_text(json.dumps(result,indent=2)+'\n')
    with (OUT/'primary-comparison.csv').open('w') as sink:
        writer=csv.DictWriter(sink,list(rows[0]));writer.writeheader();writer.writerows(rows)


if __name__=='__main__':main()
