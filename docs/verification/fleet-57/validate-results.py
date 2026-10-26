"""Independently validate the completed fleet, joins, evidence coverage and final file hashes."""
from collections import Counter
import hashlib
import json
import math
from pathlib import Path
import sys
sys.path.insert(0,'tools')
from fleet40_protocol import ALL_CASES, scenario
from provenance_bundle import verify

ROOT=Path.cwd()
OUT=Path('docs/verification/fleet-57')
ART=Path('artifacts/vehicle-reference/fleet-57')
STUDY=Path('artifacts/fleet57-reference-20261025')


def read(path):
    return json.loads(Path(path).read_text())


def lines(path):
    return [json.loads(s) for s in Path(path).read_text().splitlines()]


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream,'sha256').hexdigest()


def check_roster():
    roster=read('assets/cars/roster.json');profiles=read('assets/cars/physics.json')['cars']
    assert len(roster)==len(profiles)==57
    assert len({r['source_code'] for r in roster})==57
    assert roster[:40]==read(OUT/'original-roster.json')
    assert profiles[:40]==read(OUT/'original-physics.json')['cars']
    assert {r['id'] for r in roster}=={r['id'] for r in profiles}
    refs={r['id'] for r in read('docs/vehicle-reference/real-profiles.json')['cars']}
    assert len(refs)==50 and refs<={r['corpus_reference'] for r in profiles}
    for path,digest in read(OUT/'preparation.json')['existing_material_sha256'].items():
        assert sha(path)==digest,path
    for profile in profiles:
        assert profile['parameters'].keys()==profile['evidence'].keys()
        for key,value in profile['parameters'].items():
            values=value if isinstance(value,list) else [value]
            assert (key=='speed_governor_mps' and value is None) or all(isinstance(v,(int,float)) and math.isfinite(v) for v in values)
    return roster


def check_protocol_files(roster,config):
    assert config['cases']==ALL_CASES and config['repeats']==2
    assert config['fixed_hz']==120 and config['seed']==17
    for car in roster:
        for case in ALL_CASES:
            path=STUDY/'scenarios'/f'{case}-{car["id"]}.json'
            assert read(path)==scenario(car['id'],case)


def check_matrix(roster):
    config=read(STUDY/'study.json');runs=lines(STUDY/'runs.jsonl');data=read(OUT/'results.json')
    expected={(r['id'],case,repeat) for r in roster for case in ALL_CASES for repeat in [1,2]}
    assert len(runs)==len(data['runs'])==len(expected)==2736
    assert {(r['car'],r['case'],r['repeat']) for r in runs}==expected
    assert sha('assets/cars/physics.json')==config['profile_sha256']
    assert sha('assets/cars/roster.json')==config['roster_sha256']
    assert sha('docs/verification/fleet-40/courses.json')==config['courses_sha256']
    assert all(r['exit_code']==0 for r in runs)
    assert all(r['scenario_passed'] for r in data['runs'])
    assert len(data['aggregate'])==1368 and all(r['identical_trajectories'] for r in data['aggregate'])
    check_protocol_files(roster,config)
    assert sum(r['ticks'] for r in data['runs'])==6798960
    assert sum(r['assertions_executed'] for r in data['runs'])==7638
    assert sum('route_passed' in r for r in data['runs'])==912
    assert data['unexpected_resets']==0
    return config,runs,data


def check_prior(data):
    old={(r['car'],r['case'],r['repeat']):r for r in read('docs/verification/gt500-midrange/results.json')['runs']}
    matched=[r for r in data['runs'] if (r['car'],r['case'],r['repeat']) in old]
    assert len(matched)==1920
    for row in matched:
        before=old[row['car'],row['case'],row['repeat']]
        assert row['trajectory_sha256']==before['trajectory_sha256']
        assert row['scenario_sha256']==before['scenario_sha256']
    for kind,count in [('benchmark',57),('skidpad',114)]:
        assert (ART/f'{kind}-r1.json').read_bytes()==(ART/f'{kind}-r2.json').read_bytes()
        assert len(read(ART/f'{kind}-r1.json'))==count
    assert read(ART/'benchmark-r1.json')[:40]==read('artifacts/vehicle-reference/gt500-midrange/after-benchmark-r1.json')
    assert read(ART/'skidpad-r1.json')[:80]==read('artifacts/vehicle-reference/gt500-midrange/after-skidpad-r1.json')


def check_evidence(runs):
    expected={r['run'] for r in runs};audit=lines(OUT/'evidence-audit.jsonl');exports=lines(OUT/'export-binding.jsonl')
    assert len(audit)==len(exports)==len(expected)==2736
    assert {r['run'] for r in audit}=={r['run'] for r in exports}==expected
    assert all(r['ok'] and r['completeness']['runtime_material_complete'] and r['completeness']['rebuild_complete'] for r in audit)
    assert all(len(r['exports'])==6 for r in exports)
    hashes={}
    for n,row in enumerate(exports,1):
        for entry in row['exports']:
            path=Path(row['run'])/entry['logical_path']
            assert sha(path)==entry['raw_sha256'],str(path)
            hashes[str(path)]=entry['raw_sha256']
        if n%300==0:print('Final raw-export hashes checked',n,'/2736',flush=True)
    return hashes


def check_bridge_and_native(config):
    bridge=read(OUT/'recorder-build-bridge.json')
    assert bridge['matrix_binary_sha256']==config['binary_sha256']==sha(ART/'car-game.recorded')
    assert bridge['native_and_recorder_bridge_binary_sha256']==sha(ART/'car-game-budget.recorded')
    assert bridge['changed_compiled_source_files']==['build_source/src/diagnostics/settings.rs']
    assert [r['executions'] for r in bridge['bridges']]==[114,48]
    assert all(row['identical_trajectory'] and row['evidence_verified'] for group in bridge['bridges'] for row in group['rows'])
    native=Path(Path('artifacts/latest').read_text().strip())
    assert read(native/'manifest.json')['binary_sha256']==bridge['native_and_recorder_bridge_binary_sha256']
    reports=read(native/'report.json');assert len(reports)==57 and all(r['passed'] for r in reports)
    check=verify(native);assert check['ok'] and check['completeness']['runtime_material_complete']
    assert 'verification log:' in (ART/'verification.log').read_text()
    return str(native)


def check_diagnostic():
    proof=read(OUT/'contact-investigation.json');assert proof['lost_ray_contact_ticks']==8
    assert proof['probe_repeats_byte_identical'] and proof['repeated_primary_observation']
    assert proof['maximum_pose_error_after_declared_epoch_alignment_m']<.001
    assert (ART/'contact-probe-r1.json').read_bytes()==(ART/'contact-probe-r2.json').read_bytes()
    method=read(OUT/'contact-probe-method.json');assert sha(ART/'contact-probe')==method['binary_sha256']
    assert sha(ART/'contact-probe-source/examples/contact.rs')==method['example_source_sha256']
    for path,digest in method['production_source_sha256'].items():
        assert sha(path)==digest,path


def main():
    assert (ART/'augmentation.done').exists()
    roster=check_roster();config,runs,data=check_matrix(roster);check_prior(data)
    native=check_bridge_and_native(config);check_diagnostic();hashes=check_evidence(runs)
    archive_counts={}
    for receipt in Path('artifacts/archives').glob('*.json'):
        value=read(receipt)
        assert value['verified'] and sha(value['archive'])==value['sha256']
        archive_counts[value['archive']]=len(value['files'])
        hashes[str(receipt)]=sha(receipt);hashes[value['archive']]=value['sha256']
    source=[*Path('src').rglob('*.rs'),*Path('assets/cars').glob('*'),Path('scripts/verify.sh'),Path('docs/agents/verification.md'),Path('tests/recorder_budget.rs'),Path('tests/scenarios.rs')]
    related=[*OUT.glob('*'),*ART.rglob('*'),*STUDY.glob('*.json'),STUDY/'runs.jsonl',*list((STUDY/'scenarios').glob('*.json'))]
    for path in source+related:
        if path.is_file() and path.name not in ['validation.json','validation.log']:
            hashes[str(path)]=sha(path)
    review=read(OUT/'visual-review.json');assert review['native_overview_pages']==7 and review['scenario_matrix_reviewed']
    summary=read(OUT/'completion-summary.json')
    assert summary['executions']==summary['scenario_successes']==2736
    result={'status':'passed','edition_count':57,'real_references_covered':50,'scenario_executions':2736,'repeat_pairs_identical':1368,'prior_40_matching_trajectories':1920,'recorder_bridge_matching_trajectories':162,'native_run':native,'verified_raw_export_files':2736*6,'verified_archive_file_counts':archive_counts,'file_hashes':hashes,'limitations':'Local fixed-seed dry-road study. Runtime material and rebuild inventories verified; full portable replay and complete real-world fidelity are not claimed.'}
    (OUT/'validation.json').write_text(json.dumps(result,indent=2)+'\n')
    print(json.dumps({k:v for k,v in result.items() if k!='file_hashes'},indent=2),flush=True)
    print('Final bound files',len(hashes),flush=True)


if __name__=='__main__':
    main()
