"""Bind compact original-case results to the completed frozen 57-car recorder matrix."""
import gzip
import hashlib
import json
from pathlib import Path
OUT=Path('docs/verification/fleet-300')
ART=Path('artifacts/vehicle-reference/fleet57-next-calibration')
STUDY=Path('artifacts/fleet57-next-calibration-20261025')
BATCH=Path('artifacts/fleet300-5024-20261025')


def comparable(value):
    return {k:v for k,v in value.items() if k not in ['elapsed_ms','artifacts','name']}


def completed_state(run):
    last=None;digest=hashlib.sha256()
    with (run/'events.jsonl').open('rb') as stream:
        for line in stream:
            digest.update(line);row=json.loads(line)
            if row['kind']=='simulation.completed':last=row
    assert last is not None
    return last,digest.hexdigest()


def bind_definition(car, case, common):
    current=json.loads(json.dumps(common[case]))
    current['initial']['car']=car
    for assertion in current['assertions']:
        if assertion['kind']=='selected_car':assertion['car']=car
    recorded=json.loads((STUDY/'scenarios'/f'{case}-{car}.json').read_text())
    current.pop('name');recorded.pop('name')
    assert current==recorded,(car,case,'execution definition')


def main():
    old=[json.loads(s) for s in (STUDY/'runs.jsonl').read_text().splitlines()]
    assert len(old)==2736
    roster=json.loads((ART/'accepted-roster.json').read_text());new={}
    for car in roster:
        with gzip.open(BATCH/car['id']/'city.jsonl.gz','rt') as stream:
            rows=[json.loads(s) for s in stream]
        assert len(rows)==24
        new.update({(r['car'],r['case']):r for r in rows})
    common={r['case']:r['definition'] for r in json.loads((BATCH/'city-definitions.json').read_text())}
    proof=[]
    for row in old:
        bind_definition(row['car'],row['case'],common)
        run=Path(row['run']);summary=json.loads((run/'scenario-summary.json').read_text())
        compact=new[row['car'],row['case']];last,digest=completed_state(run)
        assert comparable(summary)==comparable(compact['summary']),(row['car'],row['case'],'summary')
        assert last['fields']['position']==compact['metrics']['end'],(row['car'],row['case'],'position')
        assert last['fields']['reset_count']==compact['metrics']['reset_count']
        assert last['tick']==compact['summary']['completed_tick']
        assert compact['repeat_hashes'][0]==compact['repeat_hashes'][1]
        proof.append({'car':row['car'],'case':row['case'],'repeat':row['repeat'],'recorded_run':str(run),
                      'events_sha256':digest,'summary_equal_without_recording_metadata_and_label':True,
                      'execution_definition_equal_except_label_and_vehicle_binding':True,'final_position_exact':True,'final_reset_count_exact':True,'completed_tick_exact':True,
                      'compact_repeat_hashes':compact['repeat_hashes']})
    (OUT/'original-city-comparison.json').write_text(json.dumps({'recorded_executions':2736,
        'compact_case_pairs':1368,'all_assertions_summaries_and_final_positions_exact':True,
        'limits':'Recording artifact lists, elapsed wall time and handling-/fleet40- display-name prefixes intentionally differ; case IDs and all execution definitions are bound separately. Full trajectory equivalence is separately tested at every tick on five cars and in1000-case/24-city scheduling differential trials.',
        'proof':proof},indent=2)+'\n')
    print('PASS: all2736 recorded city outcomes and final states match compact pairs',flush=True)


if __name__=='__main__':main()
