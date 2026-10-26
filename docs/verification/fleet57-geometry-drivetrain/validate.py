"""Final independent identity, source, repeat and evidence checks for this rerun."""
import hashlib
import json
from pathlib import Path
import sys
sys.path.insert(0, 'tools')
from provenance_bundle import verify
from fleet40_protocol import ALL_CASES, ROUTES, scenario
from input_equivalence import verify_pair

OUT = Path('docs/verification/fleet57-geometry-drivetrain')
ART = Path('artifacts/vehicle-reference/fleet57-geometry-drivetrain')
STUDY = Path('artifacts/fleet57-geometry-drivetrain-20261025')
NATIVE = Path('artifacts/runs/20261025T031729012509Z-2342545-fleet-80285-1-1792898249043075000')


def read(path):
    return json.loads(Path(path).read_text())


def lines(path):
    return [json.loads(s) for s in Path(path).read_text().splitlines()]


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def check_matrix():
    config = read(STUDY/'study.json'); runs = lines(STUDY/'runs.jsonl'); data = read(OUT/'results.json')
    roster = read('assets/cars/roster.json'); expected = {(r['id'], c, n) for r in roster for c in ALL_CASES for n in [1, 2]}
    assert len(roster) == 57 and len(expected) == len(runs) == len(data['runs']) == 2736
    assert {(r['car'], r['case'], r['repeat']) for r in runs} == expected
    assert all(r['exit_code'] == 0 for r in runs)
    assert config['repeats'] == 2 and config['seed'] == 17 and config['fixed_hz'] == 120
    assert config['cases'] == ALL_CASES
    assert data['scenario_successes'] == 2736 and data['unexpected_resets'] == 0
    assert len(data['aggregate']) == 1368 and all(r['identical_trajectories'] for r in data['aggregate'])
    assert config['binary_sha256'] == sha(ART/'car-game.recorded')
    assert config['roster_sha256'] == sha('assets/cars/roster.json')
    assert config['profile_sha256'] == sha('assets/cars/physics.json')
    assert config['courses_sha256'] == sha('docs/verification/fleet-40/courses.json')
    for car in roster:
        for case in ALL_CASES:
            assert read(STUDY/'scenarios'/f'{case}-{car["id"]}.json') == scenario(car['id'], case)
    check_timed_inputs(data)
    return config, runs


def check_timed_inputs(data):
    baseline = {(r['car'], r['case'], r['repeat']): r for r in read('docs/verification/fleet-57/results.json')['runs']}
    inputs = read(OUT/'input-comparison.json')
    proof = {(r['car'], r['case'], r['repeat']): r for r in inputs['rows']}
    baseline_hashes = read('docs/verification/fleet-57/validation.json')['file_hashes']
    export_hashes = {row['run']: e['raw_sha256'] for row in lines(OUT/'export-binding.jsonl')
                     for e in row['exports'] if e['logical_path'] == 'input.jsonl'}
    assert inputs['all_ready_relative_controls_equal'] and len(proof) == 1824
    for r in data['runs']:
        old = baseline[r['car'], r['case'], r['repeat']]
        assert old['scenario_sha256'] == r['scenario_sha256']
        if r['case'] not in ROUTES:
            p = proof[r['car'], r['case'], r['repeat']]
            assert p['ready_relative_controls_equal']
            assert p['baseline_raw_input_sha256'] == baseline_hashes[str(Path(old['run'])/'input.jsonl')]
            assert p['after_raw_input_sha256'] == export_hashes[r['run']]
            if not p['whole_raw_input_hash_equal']:
                assert verify_pair(old, r) == p


def check_native(config):
    assert 'verification log:' in (ART/'verification.log').read_text()
    assert read(NATIVE/'manifest.json')['binary_sha256'] == config['binary_sha256']
    rows = read(NATIVE/'report.json')
    assert len(rows) == 57 and all(r['passed'] for r in rows)
    assert all(r['nonblank_captures'] == 4 and r['animated_nodes'] == r['expected_nodes'] for r in rows)
    check = verify(NATIVE)
    assert check['ok'] and check['completeness']['runtime_material_complete'] and check['completeness']['rebuild_complete']
    return {'run': str(NATIVE), 'cars': 57, 'views': 228, 'evidence': check}


def check_fixtures():
    method = read(ART/'benchmark-method.json')
    for path, digest in method['production_sources'].items():
        assert sha(path) == digest, path
    for entry in method['binaries'].values():
        assert sha(entry['path']) == entry['sha256']
    for kind, size in [('benchmark', 57), ('skidpad', 114), ('gt500-trace', None)]:
        assert (ART/f'{kind}-r1.json').read_bytes() == (ART/f'{kind}-r2.json').read_bytes()
        if size:
            assert len(read(ART/f'{kind}-r1.json')) == size
    for path, digest in read(ART/'fix-inputs.json')['files'].items():
        assert sha(path) == digest, path
    assert (ART/'physics.json').read_bytes() == Path('assets/cars/physics.json').read_bytes()
    assert read(OUT/'comparison.json')['gt500_existing_guards_pass']
    assert read(OUT/'reproduction.json')['source_to_glb_byte_exact']
    assert read(OUT/'reproduction.json')['receipt_semantically_equal']
    trace_method = read(ART/'gt500-trace-method.json')
    assert sha(ART/'gt500_trace') == trace_method['binary_sha256']
    assert sha('examples/gt500_trace.rs') == trace_method['source_sha256']
    trace = read(ART/'gt500-trace-r1.json')
    assert trace['parameters'] == next(r for r in read('assets/cars/physics.json')['cars'] if r['id'] == 'redline-gt500')
    assert trace['parameters']['parameters']['driveline_efficiency'] == .82


def check_evidence(runs):
    expected = {r['run'] for r in runs}
    audits = lines(OUT/'evidence-audit.jsonl'); exports = lines(OUT/'export-binding.jsonl')
    assert len(audits) == len(exports) == len(expected) == 2736
    assert {r['run'] for r in audits} == {r['run'] for r in exports} == expected
    assert all(r['ok'] and r['completeness']['runtime_material_complete'] and r['completeness']['rebuild_complete'] for r in audits)
    hashes = {}
    for n, row in enumerate(exports, 1):
        assert len(row['exports']) == 6
        for e in row['exports']:
            path = Path(row['run'])/e['logical_path']
            assert sha(path) == e['raw_sha256'], str(path)
            assert e['semantic_equality_after_policy_redaction']
            hashes[str(path)] = e['raw_sha256']
        if n % 300 == 0:
            print('Final raw export hashes checked', n, '/2736', flush=True)
    return hashes


def check_archives():
    hashes = {}
    for name in ['fleet40-calibrated-v2-20261024', 'fleet40-gt500-midrange-20261024']:
        receipt = Path('artifacts/archives')/(name+'.json')
        data = read(receipt); archive = Path(data['archive'])
        assert data['verified'] and sha(archive) == data['sha256']
        assert not (Path('artifacts')/name).exists()
        hashes[str(receipt)] = sha(receipt); hashes[str(archive)] = data['sha256']
    return hashes


def main():
    config, runs = check_matrix(); native = check_native(config); check_fixtures()
    assert read(ART/'matrix-exit.json')['exit_code'] == 0
    hashes = check_evidence(runs)
    hashes.update(check_archives())
    hashes['docs/verification/fleet-57/validation.json'] = sha('docs/verification/fleet-57/validation.json')
    for directory in [OUT, ART]:
        for p in directory.rglob('*'):
            if p.is_file() and p != OUT/'validation.json' and p.name != 'validation.log':
                hashes[str(p)] = sha(p)
    for p in [Path('assets/cars/ref-a08.glb'), Path('assets/cars/ref-a08.json'), Path('assets/cars/roster.json'), Path('assets/cars/physics.json')]:
        hashes[str(p)] = sha(p)
    result = {'status': 'passed', 'executions': 2736, 'repeat_pairs': 1368,
              'bundle_audits': 2736, 'raw_exports_rehashed': 16416, 'same_scenarios_and_ready_relative_timed_inputs': True,
              'route_inputs': 'Closed-loop feedback may differ; identical route/controller definitions and courses required.',
              'native': native, 'file_hashes': hashes,
              'limits': 'Same dry map and seed17; runtime replay completeness remains explicitly unclaimed.'}
    (OUT/'validation.json').write_text(json.dumps(result, indent=2)+'\n')
    print('Validated', len(hashes), 'file hashes and all2736 runs', flush=True)


if __name__ == '__main__':
    main()
