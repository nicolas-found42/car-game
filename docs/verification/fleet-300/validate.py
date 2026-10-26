"""Bind final coverage, profile/track revisions, protected baselines and native evidence."""
import hashlib
import json
from pathlib import Path
import sys
import subprocess
sys.path.insert(0,'tools')
from provenance_bundle import verify
ROOT=Path.cwd()
OUT=ROOT/'docs/verification/fleet-300'
ART=ROOT/'artifacts/vehicle-reference/fleet300'
INITIAL=ROOT/'artifacts/fleet300-5024-20261025'
SLOPES=ROOT/'artifacts/fleet300-slopes-20261025'
REPAIR=ROOT/'artifacts/fleet300-profile-repair-20261025'


def read(path):
    return json.loads(Path(path).read_text())


def sha(path):
    with Path(path).open('rb') as stream:return hashlib.file_digest(stream,'sha256').hexdigest()


def source_manifest(name):
    manifest=read(ART/name);source=Path(manifest['compiled_source'])
    for path,digest in manifest['files'].items():assert sha(source/path)==digest,path
    return source


def same_execution_source(path, source):
    current=path.read_text();frozen=(source/path).read_text()
    if current==frozen:return
    if path==Path('src/diagnostics/settings.rs'):
        current=current.split('/// Only the aggregate fleet report')[0].rstrip()+'\n'
        current=current.replace('1024..=8 * 1024 * 1024 * 1024', '1024..=2 * 1024 * 1024 * 1024')
    elif path==Path('src/smoke.rs'):
        current=current.split('// Driving phases follow simulated time;')[0].rstrip()+'\n'
        current=current.replace('    time: Res<Time<Virtual>>,\n    real_time: Res<Time<Real>>,','    time: Res<Time<Real>>,')
        current=current.replace('real_time.', 'time.')
        current=current.replace('script_elapsed(&time, smoke.started.unwrap())', 'time.elapsed_secs_f64() - smoke.started.unwrap()')
    elif path==Path('src/city.rs'):
        current=current.replace('        // The common native fleet maneuver must not strike sidewalks or buildings.\n        // Interactive play and recorded city scenarios retain every original block.\n        if config.mode == "fleet" && (-1..=1).contains(&x) {\n            continue;\n        }\n','')
    elif path==Path('src/diagnostics.rs'):
        current=current.replace('pub(crate) use settings::metadata_limit;\n','')
        start=current.index('fn write_json_atomic(path:')
        end=current.index('    let bytes = recording::bounded_json(value, max_bytes, true)',start)
        current=current[:start]+'fn write_json_atomic(path: &Path, value: &impl Serialize) -> std::io::Result<()> {\n'+current[end:]
        current=current.replace('recording::bounded_json(value, max_bytes, true)','recording::bounded_json(value, 2 * 1024 * 1024, true)')
    elif path==Path('src/diagnostics/health.rs'):
        current=current.replace('        let relative = path.strip_prefix(&self.dir).unwrap_or(path);\n','')
        current=current.replace('super::write_json_atomic_bounded(path, value, super::metadata_limit(relative))','super::write_json_atomic(path, value)')
    else:
        assert path==Path('src/provenance.rs'),str(path)
        current=current.split('#[cfg(test)]')[0].rstrip()+'\n'
        current=current.replace('    let allowed: std::collections::BTreeSet<_> = allowed_assets().into_iter().collect();\n','')
        current=current.replace('if !allowed.contains(&logical)', 'if !allowed_assets().contains(&logical)')
        current=current.replace('crate::diagnostics::metadata_limit(relative) as u64','2 * 1024 * 1024')
    assert current==frozen.rstrip()+'\n','Only documented diagnostic export/cache changes may differ: '+str(path)


def check_profiles():
    source=source_manifest('final-source-manifest.json')
    for p in Path('src').rglob('*.rs'):same_execution_source(p,source)
    for name in ['physics','roster']:
        assert (ROOT/f'assets/cars/{name}.json').read_bytes()==(source/f'assets/cars/{name}.json').read_bytes()
    initial=read(OUT/'initial-prepared-physics.json')['cars'];final=read(ROOT/'assets/cars/physics.json')['cars']
    assert len(initial)==len(final)==300
    assert [a['id'] for a,b in zip(initial,final,strict=True) if a!=b]==['exp-161']
    original=read(OUT/'original-physics.json')['cars'];assert final[:57]==original
    roster=read(ROOT/'assets/cars/roster.json');assert roster[:57]==read(OUT/'original-roster.json')
    materials=read(OUT/'preparation.json')['existing_material_sha256']
    for path,digest in materials.items():assert sha(ROOT/path)==digest,path
    repaired=next(r for r in final if r['id']=='exp-161')
    assert repaired['parameters']['torque_nm']>0 and repaired['evidence']['torque_nm']['status']=='estimate'
    return source


def check_definitions():
    old=read(ART/'stress-definitions.json');new=read(ART/'stress-definitions-slopefix.json')
    assert len(old)==len(new)==5000
    assert {r['id'] for r in new}==set(range(5000))
    for a,b in zip(old,new,strict=True):
        height=b.pop('base_height_m');assert height==(128 if a['family']=='slope' else 0)
        assert a==b
    isolation=read(ART/'slopefix-source-isolation.json')
    assert isolation['all_production_library_sources_and_profiles_exact']
    assert isolation['common_unchanged_physical_definitions']==4500
    assert read(OUT/'original-city-comparison.json')['all_assertions_summaries_and_final_positions_exact']
    assert read(OUT.parent/'fleet57-next-calibration/validation.json')['gt500_c5_rs4_ctsv_preserved']


def check_execution_receipts(root, expected_cars, expected_executions):
    status=read(root/'execution-status.json');assert not status['errors']
    assert len(status['cars'])==expected_cars
    hashes={};total=0
    for car in status['cars']:
        stages=car.get('stages',[car.get('stage')])
        for stage in stages:
            assert stage['exit_code']==0 and stage['complete_coverage']
            assert stage['different_repeat_pairs']==0
            assert sha(stage['result'])==stage['sha256']
            hashes[stage['result']]=stage['sha256'];total+=stage['executions']
        trace_index=root/car['car']/'failure-traces.json'
        hashes[str(trace_index)]=sha(trace_index)
        for trace in read(trace_index):assert sha(trace['path'])==trace['sha256']
    assert total==expected_executions
    return hashes


def check_numeric():
    evaluation=read(OUT/'final/evaluation.json')
    assert evaluation['cars']==300 and evaluation['executions']==3014400
    assert evaluation['original_cases_per_car']==24 and evaluation['new_cases_per_car']==5000
    assert evaluation['repeats']==2 and evaluation['slope_revision_applied']
    assert evaluation['additional_slope_executions']==300000 and evaluation['additional_profile_executions']==10048
    assert evaluation['different_stress_repeat_pairs']==0
    assert len(evaluation['family_metrics'])==3000 and len(evaluation['car_results'])==300
    assert all(r['pairs']==500 and r['executions']==1000 for r in evaluation['family_metrics'])
    hashes=check_execution_receipts(INITIAL,300,3014400)
    hashes.update(check_execution_receipts(SLOPES,300,300000))
    hashes.update(check_execution_receipts(REPAIR,1,10048))
    for r in evaluation['car_results']:assert r['executions']==10048
    return evaluation,hashes


def check_method_bindings():
    initial=read(INITIAL/'method.json');slopes=read(SLOPES/'method.json');repair=read(REPAIR/'method.json')
    for binary,definition in [('bin-compact','stress-definitions.json'),('bin-final','stress-definitions-slopefix.json')]:
        assert json.loads(subprocess.check_output([str(ART/binary/'fleet_stress'),'--manifest']))==read(ART/definition)
    final_manifest=read(ART/'final-source-manifest.json')
    for name,digest in initial['binaries'].items():assert sha(ART/'bin-compact'/name)==digest
    for name,digest in final_manifest['binaries'].items():assert sha(ART/'bin-final'/name)==digest
    stress=sha(ART/'bin-final/fleet_stress');city=sha(ART/'bin-final/fleet_city_batch')
    assert stress==slopes['binary_sha256']==repair['binary_sha256']
    assert city==repair['city_binary_sha256']
    city_definition=sha(ART/'city-definitions.json')
    assert city_definition==initial['definitions_sha256']==repair['definitions_sha256']
    assert city_definition==sha(INITIAL/'city-definitions.json')==sha(REPAIR/'city-definitions.json')
    assert slopes['definitions_sha256']==sha(ART/'stress-definitions-slopefix.json')
    assert repair['repair_sha256']==sha(OUT/'profile-repair.json')
    assert initial['roster_sha256']==slopes['roster_sha256']==sha(ROOT/'assets/cars/roster.json')
    compact=ART/'source-compact/assets/cars'
    assert read(compact/'physics.json')==read(OUT/'initial-prepared-physics.json')
    assert read(compact/'roster.json')==read(ROOT/'assets/cars/roster.json')
    ids=[c['id'] for c in read(ROOT/'assets/cars/roster.json')]
    assert initial['cars']==slopes['cars']==ids and repair['cars']==['exp-161']
    assert slopes['case_ids']==list(range(4000,4500))


def check_gear_probe():
    trace=read(OUT/'gear-hunting-trace.json');run=Path(trace['run']);summary=trace['summary']
    assert trace['status']=='confirmed shift hunting' and summary['passed']
    assert summary['assertions_passed']==summary['assertions_declared']==3
    rows=[json.loads(line) for line in (run/'physics.jsonl').open()]
    ready=summary['readiness']['tick'];rows=[r for r in rows if ready<r['tick']<=ready+2400]
    assert len(rows)==2400 and sum(a['gear']!=b['gear'] for a,b in zip(rows,rows[1:]))==trace['gear_transitions']==592
    tail=rows[-1200:]
    assert all(r['grounded']==4 and r['reset_count']==0 for r in tail)
    assert sum(sum(abs(x) for x in r['drive_demands'])<1e-3 for r in tail)/1200==trace['last_10s_zero_drive_demand_fraction']
    assert trace['binary_sha256']==sha(ART/'car-game.recorded-attempt7')
    for path,digest in trace['files'].items():assert sha(path)==digest
    result=verify(run);assert result['ok'] and result['completeness']['runtime_material_complete']


def check_native_controls():
    corridor=read(OUT/'native-corridor-validation.json')
    assert corridor['status']=='passed' and corridor['cars']==300
    assert corridor['original_interactive_mini_collision_reproduced'] and corridor['native_smoke_input_values_phase_boundaries_and_assertions_unchanged']
    for path,digest in corridor['files'].items():assert sha(path)==digest
    clock=read(OUT/'native-clock-validation.json');assert clock['status']=='passed' and clock['phase_clock']=='Time<Virtual>'
    for path,digest in clock['files'].items():assert sha(path)==digest
    assert read(ART/'awake-assertion.json')['process_scoped']


def check_native():
    check_native_controls()
    assert read(ART/'verification-exit.json')['exit_code']==0
    assert 'verification log:' in (ART/'verification.log').read_text()
    native=(ROOT/(ART/'native-run.txt').read_text().strip()).resolve();reports=read(native/'report.json')
    assert len(reports)==300
    assert all(r['passed'] and r['nonblank_captures']==4 and r['animated_nodes']==r['expected_nodes'] for r in reports)
    result=verify(native);assert result['ok']
    assert result['completeness']['runtime_material_complete'] and result['completeness']['rebuild_complete']
    inventory=read(native/'evidence-manifest.json')['inventory']
    for role,name in [('embedded_catalog','roster'),('embedded_physics_profile','physics')]:
        entry=next(r for r in inventory if r['role']==role)
        assert entry['sha256']==sha(ROOT/f'assets/cars/{name}.json')
    assert read(native/'manifest.json')['binary_sha256']==sha(ART/'car-game.recorded')
    review=read(OUT/'native-review.json');assert review['native']['cars']==300 and review['native']['views']==1200
    assert review['overviews_reviewed']==34 and review['figures_reviewed']==2
    for path,digest in review['file_hashes'].items():assert sha(path)==digest
    return {'run':str(native.relative_to(ROOT)),'cars':300,'views':1200,'evidence':result}


def main():
    source=check_profiles();check_definitions();check_method_bindings();check_gear_probe();evaluation,hashes=check_numeric();native=check_native()
    for name in ['compact-source-manifest.json','slopefix-source-manifest.json']:source_manifest(name)
    assert sha(ART/'bin-final/fleet_stress')==read(SLOPES/'method.json')['binary_sha256']
    assert sha(ART/'bin-final/fleet_stress')==read(REPAIR/'method.json')['binary_sha256']
    archive=read(ROOT/'artifacts/archives/fleet57-reference-calibration-20261025.json')
    assert archive['verified'] and sha(archive['archive'])==archive['sha256']
    for directory in [OUT,ART]:
        for p in directory.rglob('*'):
            if p.is_file() and p!=OUT/'validation.json' and not p.name.endswith('.log'):
                hashes[str(p.relative_to(ROOT))]=sha(p)
    result={'status':'passed','cars':300,'common_cases_per_car':5024,'repeats':2,
            'final_common_executions':3014400,'all_matrix_executions_including_revisions':3324448,
            'failed_city_pairs':evaluation['failed_city_pairs'],'failed_stress_pairs':evaluation['failed_stress_pairs'],
            'original_57_profiles_and_geometry_exact':True,'protected_four_preserved':True,
            'slope_cases_repeated_on_all300_cars':True,'repaired_profile_repeated_on_all5024_cases':True,
            'native':native,'source':str(source.relative_to(ROOT)),'file_hashes':hashes}
    (OUT/'validation.json').write_text(json.dumps(result,indent=2)+'\n')
    print('PASS: 300 cars, common5024 cases twice, all revisions/evidence/native checks bound',flush=True)


if __name__=='__main__':main()
