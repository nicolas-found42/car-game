"""Verify frozen source, identical protocol, complete evidence and protected improvements."""
import hashlib
import json
from pathlib import Path
import sys
sys.path.insert(0,'tools')
from fleet40_protocol import ALL_CASES,scenario
from provenance_bundle import verify
OUT=Path('docs/verification/fleet57-next-calibration')
ART=Path('artifacts/vehicle-reference/fleet57-next-calibration')
STUDY=Path('artifacts/fleet57-next-calibration-20261025')
BASE=Path('artifacts/vehicle-reference/fleet57-reference-calibration')


def read(path):
    return json.loads(Path(path).read_text())


def lines(path):
    return [json.loads(s) for s in Path(path).read_text().splitlines()]


def sha(path):
    with Path(path).open('rb') as stream:return hashlib.file_digest(stream,'sha256').hexdigest()


def frozen(path):
    if path.startswith('src/'):return ART/'production-source'/path[4:]
    if path in ['assets/cars/roster.json','assets/cars/physics.json']:
        return ART/('accepted-'+Path(path).name)
    return Path(path)


def check_sources():
    for filename,field in [('fix-inputs.json','files'),('benchmark-method.json','production_sources'),('gt500-trace-method.json','production_sources')]:
        for path,digest in read(ART/filename)[field].items():assert sha(frozen(path))==digest,path
    for entry in read(ART/'benchmark-method.json')['binaries'].values():assert sha(entry['path'])==entry['sha256']
    assert read(OUT/'compiled-source-diff.json')['changed_compiled_inputs']==['build_source/assets/cars/physics.json']
    for kind,size in [('benchmark',57),('skidpad',114),('gt500-trace',None)]:
        assert (ART/f'{kind}-r1.json').read_bytes()==(ART/f'{kind}-r2.json').read_bytes()
        if size:assert len(read(ART/f'{kind}-r1.json'))==size
    assert (ART/'gt500-trace-r1.json').read_bytes()==(BASE/'gt500-trace-r1.json').read_bytes()
    for kind,keys in [('benchmark',['car']),('skidpad',['car','diameter_ft'])]:
        old={tuple(r[k] for k in keys):r for r in read(BASE/f'{kind}-r1.json')}
        new=read(ART/f'{kind}-r1.json')
        assert all(old[tuple(r[k] for k in keys)]==r for r in new if r['car'] not in ['350z','gt4-nsx'])
    old=read(ART/'before-physics.json')['cars'];new=read(ART/'accepted-physics.json')['cars']
    assert {a['id'] for a,b in zip(old,new,strict=True) if a!=b}=={'350z','gt4-nsx'}
    assert (ART/'before-roster.json').read_bytes()==(ART/'accepted-roster.json').read_bytes()


def check_definitions(config,roster):
    assert config['cases']==ALL_CASES and config['fixed_hz']==120 and config['seed']==17
    assert config['courses_sha256']==sha('docs/verification/fleet-40/courses.json')
    for car in roster:
        for case in ALL_CASES:assert read(STUDY/'scenarios'/f'{case}-{car["id"]}.json')==scenario(car['id'],case)


def check_matrix():
    config=read(STUDY/'study.json');runs=lines(STUDY/'runs.jsonl');roster=read(ART/'accepted-roster.json')
    expected={(r['id'],c,n) for r in roster for c in ALL_CASES for n in [1,2]}
    assert len(runs)==len(expected)==2736
    assert {(r['car'],r['case'],r['repeat']) for r in runs}==expected
    assert all(r['exit_code']==0 for r in runs)
    assert config['binary_sha256']==sha(ART/'car-game.recorded')
    assert config['roster_sha256']==sha(ART/'accepted-roster.json') and config['profile_sha256']==sha(ART/'accepted-physics.json')
    check_definitions(config,roster)
    data=read(OUT/'results.json');assert data['scenario_successes']==2736 and data['unexpected_resets']==0
    assert len(data['aggregate'])==1368 and all(r['identical_trajectories'] for r in data['aggregate'])
    unchanged=read(OUT/'comparison.json')['matrix']['unchanged_baseline_trajectories_by_car']
    assert all(unchanged[c['id']]==48 for c in roster if c['id'] not in ['350z','gt4-nsx'])
    return config,runs


def check_evidence(runs):
    audits=lines(OUT/'evidence-audit.jsonl');exports=lines(OUT/'export-binding.jsonl');expected={r['run'] for r in runs}
    assert len(audits)==len(exports)==2736
    assert {r['run'] for r in audits}=={r['run'] for r in exports}==expected
    assert all(r['ok'] and r['completeness']['runtime_material_complete'] and r['completeness']['rebuild_complete'] for r in audits)
    hashes={}
    for row in exports:
        assert len(row['exports'])==6
        for entry in row['exports']:
            path=Path(row['run'])/entry['logical_path'];assert sha(path)==entry['raw_sha256']
            assert entry['semantic_equality_after_policy_redaction'];hashes[str(path)]=entry['raw_sha256']
    return hashes


def check_native(config):
    assert 'verification log:' in (ART/'verification.log').read_text()
    native=Path((ART/'native-run.txt').read_text().strip());rows=read(native/'report.json')
    assert len(rows)==57 and all(r['passed'] and r['nonblank_captures']==4 and r['animated_nodes']==r['expected_nodes'] for r in rows)
    assert read(native/'manifest.json')['binary_sha256']==config['binary_sha256']
    result=verify(native);assert result['ok'] and result['completeness']['runtime_material_complete'] and result['completeness']['rebuild_complete']
    review=read(OUT/'visual-review.json');assert review['native_overviews_reviewed']==7 and review['cars_visible']==57
    assert review['comparison_chart_reviewed']
    for path,digest in review['file_hashes'].items():assert sha(path)==digest
    return {'run':str(native),'cars':57,'views':228,'evidence':result}


def main():
    config,runs=check_matrix();check_sources();native=check_native(config);hashes=check_evidence(runs)
    assert read(ART/'matrix-exit.json')['exit_code']==0
    for directory in [OUT,ART]:
        for p in directory.rglob('*'):
            if p.is_file() and p!=OUT/'validation.json' and p.name not in ['validation.log','finalize.log']:hashes[str(p)]=sha(p)
    result={'status':'passed','executions':2736,'repeat_pairs':1368,'bundle_audits':2736,
            'raw_exports_rehashed':16416,'all_original_scenarios_preserved':True,
            'untouched_editions_with_48_exact_baseline_trajectories':55,
            'gt500_c5_rs4_ctsv_preserved':True,'native':native,
            'baseline_validation_sha256':sha('docs/verification/fleet57-reference-calibration/validation.json'),'file_hashes':hashes}
    (OUT/'validation.json').write_text(json.dumps(result,indent=2)+'\n');print('PASS: 2736 paired executions, all evidence, exact protected trajectories and fixtures',flush=True)


if __name__=='__main__':main()
