#!/usr/bin/env python3
"""Recompute the original 13-car scenarios within the expanded run and match their previous inputs."""
import argparse
import json
from pathlib import Path

from analyze_handling_study import analyze_one, aggregate
from compare_handling_studies import compare
from handling_protocol import CASES
from run_handling_study import ROOT, sha


def read(path):
    return json.loads(path.read_text())


def asset_hash(run, logical):
    entries=read(ROOT/run/'provenance/capture.json')['inventory']
    hashes={e['sha256'] for e in entries if e['logical_path']==logical}
    assert len(hashes)==1
    return hashes.pop()


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('study',type=Path);parser.add_argument('--output',type=Path,required=True)
    args=parser.parse_args();out=args.output
    original=read(ROOT/'docs/verification/fleet-40/original-roster.json');ids={c['id'] for c in original}
    roster=read(ROOT/'assets/cars/roster.json');assert roster[:13]==original
    profiles=read(ROOT/'assets/cars/physics.json')['cars']
    assert profiles[:13]==read(ROOT/'docs/verification/fleet-40/original-profiles.json')['cars']
    before=read(ROOT/'artifacts/handling-study-profiles-audited-20261023/analysis.json')
    old={(r['car'],r['case'],r['repeat']):r for r in before['runs']}
    config=read(args.study/'study.json');cars={c['id']:c for c in roster}
    tasks=[json.loads(s) for s in (args.study/'runs.jsonl').read_text().splitlines()]
    tasks=[t for t in tasks if t['car'] in ids and t['case'] in CASES];assert len(tasks)==130
    rows=[]
    for task in tasks:
        row=analyze_one(task,config,cars[task['car']]);first=old[row['car'],row['case'],row['repeat']]
        assert row['scenario_sha256']==first['scenario_sha256']
        assert row['matched_input_sha256']==first['matched_input_sha256']
        logical='assets/'+cars[row['car']]['path']
        assert asset_hash(row['run'],logical)==asset_hash(first['run'],logical)
        rows.append(row)
    after={'aggregate':aggregate(rows)}
    result={'schema':1,'before':'artifacts/handling-study-profiles-audited-20261023','after':str(args.study),
            'matched_runs':130,'same_original_car_assets_profiles_and_effective_inputs':True,
            'city_changed':'Added northeast connector; original scenario footprints are retained and checked',
            'rows':compare(before,after),'profile_sha256':sha(ROOT/'assets/cars/physics.json')}
    out.mkdir(parents=True,exist_ok=True)
    (out/'original-13-comparison.json').write_text(json.dumps(result,indent=2)+'\n')
    print('130 original-fleet runs matched by scenario, inputs, source models and original profiles')


if __name__=='__main__':main()
