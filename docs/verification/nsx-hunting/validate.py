"""Bind the gearbox correction to complete common-matrix and recorded evidence."""
import hashlib
import json
from pathlib import Path
import sys

ROOT=Path(__file__).resolve().parents[3]
sys.path.insert(0,str(ROOT/'tools'))
from provenance_bundle import verify
ART=ROOT/'artifacts/nsx-gear-hunting-fix'
OUT=Path(__file__).resolve().parent


def read(path):
    return json.loads(Path(path).read_text())


def sha(path):
    with Path(path).open('rb') as stream:return hashlib.file_digest(stream,'sha256').hexdigest()


def preservation():
    before=read(ART/'before/hashes.json')
    for path in ['assets/cars/physics.json','assets/cars/roster.json']:assert sha(ROOT/path)==before[path]
    old=read(ROOT/'artifacts/diagnosis-upgrade/recorded-source-match.json')['files']
    changed=sorted(path for path,digest in old.items() if sha(ROOT/path)!=digest)
    assert changed==['src/diagnostics/decision.rs','src/vehicle/profile.rs'],changed
    binding=read(ART/'candidate-source.json')
    for path,digest in binding['files'].items():assert sha(ROOT/path)==digest
    for name,digest in binding['binaries'].items():assert sha(ART/'bin'/name)==digest
    original=read(ROOT/'artifacts/vehicle-reference/fleet300/stress-definitions-slopefix.json')
    assert original==read(ART/'stress-definitions.json')
    assert sha(ART/'city-definitions.json')==sha(ROOT/'artifacts/vehicle-reference/fleet300/city-definitions.json')
    comparison=read(ART/'comparison.json');assert comparison['status']=='passed'
    assert comparison['pairs']==1507200 and comparison['cars']==300
    assert comparison['unaffected_cars_all_case_pairs_exact']==289
    assert comparison['protected_all_5024_cases_exact']==['redline-gt500','ref-a08','ref-e12','ref-a14']
    return {'changed_production_sources':changed,'all_profiles_and_roster_unchanged':True,
            'protected_cars_all_5024_case_pairs_exact':comparison['protected_all_5024_cases_exact']}


def verified_replay(path):
    run=ROOT/path
    result=verify(run);assert result['ok']
    entry=next(r for r in read(run/'evidence-manifest.json')['inventory'] if r['logical_path']=='physics.jsonl')
    rows=0
    with (run/'physics.jsonl').open() as raw,(run/entry['object']).open() as captured:
        for left,right in zip(raw,captured,strict=True):
            assert json.dumps(json.loads(left),sort_keys=True)==json.dumps(json.loads(right),sort_keys=True)
            rows+=1
    return {'run':path,'raw_sha256':sha(run/'physics.jsonl'),'captured_object_sha256':entry['sha256'],
            'canonical_rows_verified':rows,'comparison':'key ordering normalized; numeric types and signed zero preserved'}


def reproduction():
    assert read(ART/'minimized-red-exit.json')['exit_code']!=0
    assert 'gear hunting: 595 changes' in (ART/'minimized-red-test.log').read_text()
    assert read(ART/'green-tests-exit.json')['exit_code']==0
    assert read(ART/'supplemental-window-tests-exit.json')['exit_code']==0
    assert read(ART/'supplemental-all-targets-exit.json')['exit_code']==0
    assert read(ART/'supplemental-gates-exit.json')['exit_code']==0
    assert read(ART/'portable-decision-proof.json')['status']=='passed' 
    controls=read(ART/'ten-second-comparison.json');assert controls['status']=='passed'
    assert controls['controls']['before']['gear_hunting'] and not controls['controls']['after']['gear_hunting']
    bindings=[verified_replay(entry['run']) for entry in controls['controls'].values()]
    divergence=read(ART/'ten-second-first-divergence.json')
    assert divergence['matched_samples']==409 and divergence['different_fields']==['gear','rpm']
    decision=read(ART/'first-shift-decision-difference.json')
    assert decision['tick']==410 and decision['before']['gear']==1 and decision['after']['gear']==2
    original=read(ART/'probe-result.json')
    assert original['gear_transitions']<=10 and original['exit_code']==1
    bindings.append(verified_replay((ART/'probe-run.txt').read_text().strip()))
    return {'matched_ten_second_reproduction':controls,'unchanged_city_boundary_result':original,'raw_to_captured_physics':bindings}


def native():
    run=ROOT/(ART/'native-run.txt').read_text().strip()
    report=read(run/'report.json');assert len(report)==300
    assert all(r['passed'] and r['nonblank_captures']==4 and r['animated_nodes']==r['expected_nodes'] for r in report)
    old_run=ROOT/(ROOT/'artifacts/diagnosis-upgrade/native-run.txt').read_text().strip()
    original_assets={r['id']:r['asset_sha256'] for r in read(old_run/'report.json')}
    assert original_assets=={r['id']:r['asset_sha256'] for r in report}
    result=verify(run);assert result['ok'] and result['completeness']['rebuild_complete']
    capture=read(run/'provenance/capture.json');assert capture['source_comparison']['matches']
    objects={r['logical_path']:r['sha256'] for r in capture['inventory'] if r.get('availability')=='included'}
    for path,digest in read(ART/'candidate-source.json')['files'].items():assert objects['build_source/'+path]==digest
    assert result['completeness']['runtime_material_complete']
    digest=sha(ART/'bin/car-game.recorded');assert read(run/'manifest.json')['binary_sha256']==digest
    assert digest==read(ART/'probe-result.json')['binary_sha256']
    health=read(run/'recorder-health.json');assert not health['failed'] and not health['losses']
    assert not health['automatic_window_pending'] and health['completed_tick']==health['last_durable_tick']
    windows=read(run/'diagnostic-windows.json');assert windows['triggered']==windows['exported'] and not windows['active']
    return {'run':str(run.relative_to(ROOT)),'cars':300,'captures':1200,'provenance':result,'windows':windows}


def matrix():
    assert read(ART/'matrix-exit.json')['exit_code']==0
    method=read(ART/'matrix/method.json');status=read(ART/'matrix/execution-status.json')
    assert not status['errors'] and len(status['cars'])==300
    executions=0;hashes={}
    for car in status['cars']:
        for stage in car['stages']:
            assert stage['complete_coverage'] and stage['exit_code']==0 and stage['different_repeat_pairs']==0
            assert sha(stage['result'])==stage['sha256'];hashes[stage['result']]=stage['sha256']
            executions+=stage['executions']
    assert executions==method['expected_executions']==3014400
    return {'executions':executions,'repeats':2,'cars':300,'scenarios_per_car':5024,'result_hashes':hashes,
            'evaluation':{k:v for k,v in read(OUT/'evaluation.json').items() if k not in ['car_results','family_metrics']}}


def figure():
    proof=read(ART/'plot-proof.json')
    assert sha(OUT/'shift-diagnosis.png')==proof['image_sha256']
    for path,digest in proof['input_hashes'].items():assert sha(ROOT/path)==digest
    return proof


def main():
    assert read(ART/'verification-exit.json')['exit_code']==0
    result={'status':'passed','preservation':preservation(),'reproduction':reproduction(),'matrix':matrix(),'native':native(),'figure':figure()}
    gate_logs=['verification.log','supplemental-gates.log','supplemental-all-targets.log','minimized-red-test.log',
               'green-tests.log','supplemental-window-tests.log','matrix.log','final-analysis-0.log','final-analysis-1.log',
               'report-complexity.log']
    result['gate_log_hashes']={name:sha(ART/name) for name in gate_logs}
    result['evidence_hashes']={str(p.relative_to(ROOT)):sha(p) for folder in [ART,OUT]
        for p in folder.rglob('*') if p.is_file() and p.suffix in ['.json','.py','.jsonl'] and p!=OUT/'validation.json'}
    (OUT/'validation.json').write_text(json.dumps(result,indent=2)+'\n')
    print('PASS: transmission correction, full common matrix, protected cars and recorded native evidence',flush=True)


if __name__=='__main__':main()
