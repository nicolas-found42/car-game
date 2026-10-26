"""Final independent identity, source, repeat and evidence checks for this rerun."""
import csv
import hashlib
import json
from pathlib import Path
import sys
sys.path.insert(0, 'tools')
from provenance_bundle import verify
from fleet40_protocol import ALL_CASES, ROUTES, scenario
from input_equivalence import verify_pair

OUT = Path('docs/verification/fleet57-reference-calibration')
ART = Path('artifacts/vehicle-reference/fleet57-reference-calibration')
STUDY = Path('artifacts/fleet57-reference-calibration-20261025')
NATIVE = Path((ART/'native-run.txt').read_text().strip())


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
    baseline = {(r['car'], r['case'], r['repeat']): r for r in read('docs/verification/fleet57-geometry-drivetrain/results.json')['runs']}
    inputs = read(OUT/'input-comparison.json')
    proof = {(r['car'], r['case'], r['repeat']): r for r in inputs['rows']}
    baseline_hashes = read('docs/verification/fleet57-geometry-drivetrain/validation.json')['file_hashes']
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
    check_visual_review()
    return {'run': str(NATIVE), 'cars': 57, 'views': 228, 'evidence': check}


def check_visual_review():
    review = read(OUT/'visual-review.json')
    assert review['native_overviews_reviewed'] == 7 and review['cars_visible'] == 57
    assert review['comparison_chart_reviewed']
    for path, digest in review['file_hashes'].items():
        assert sha(path) == digest, path
    assert str(OUT/'comparison.png') in review['file_hashes']


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
    check_preservation()
    check_sources_and_probes()
    assert read(OUT/'comparison.json')['gt500_existing_guards_pass']
    evaluation = read(OUT/'reference-evaluation.json')
    assert evaluation['selected_target_score']['targets'] == 8
    assert all(abs(r['after_error_pct']) < 5 for r in evaluation['primary_rows'] if r['use_for_calibration'])
    trace_method = read(ART/'gt500-trace-method.json')
    assert sha(ART/'gt500_trace') == trace_method['binary_sha256']
    assert sha('examples/gt500_trace.rs') == trace_method['source_sha256']
    trace = read(ART/'gt500-trace-r1.json')
    assert trace['parameters'] == next(r for r in read('assets/cars/physics.json')['cars'] if r['id'] == 'redline-gt500')
    assert trace['parameters']['parameters']['driveline_efficiency'] == .82


def check_sources_and_probes():
    supplement = Path('docs/vehicle-reference/calibration-supplement-20261025.json')
    assert read(supplement) == read(OUT/'handling-targets.json')
    coverage = list(csv.DictReader((OUT/'profile-coverage.csv').open()))
    assert len(coverage) == 57 and len({r['corpus_reference'] for r in coverage if r['corpus_reference']}) == 50
    assert {r['car'] for r in coverage} == {r['id'] for r in read('assets/cars/roster.json')}
    method = read(ART/'probe-method.json')
    for path, digest in method['production_source_sha256'].items():
        assert sha(path) == digest, path
    for path, digest in method['shadow_source_sha256'].items():
        assert sha(ART/'probe-source'/path) == digest, path
    for entry in method['binaries'].values():
        assert sha(entry['path']) == entry['sha256']
    diff = read(OUT/'compiled-source-diff.json')
    assert diff['changed_compiled_inputs'] == ['build_source/assets/cars/physics.json']
    assert diff['after_binary_sha256'] == sha(ART/'car-game.recorded')


def check_preservation():
    old = {r['id']:r for r in read(ART/'before-physics.json')['cars']}
    new = {r['id']:r for r in read('assets/cars/physics.json')['cars']}
    assert {k for k in old if old[k] != new[k]} == {'ref-e12','ref-a14'}
    assert (ART/'before-roster.json').read_bytes() == Path('assets/cars/roster.json').read_bytes()
    baseline = read('docs/verification/fleet57-geometry-drivetrain/results.json')
    before = {(r['car'],r['case'],r['repeat']):r for r in baseline['runs']}
    matched = [r for r in read(OUT/'results.json')['runs'] if r['car'] not in ['ref-e12','ref-a14']]
    assert len(matched) == 2640
    assert all(r['trajectory_sha256'] == before[r['car'],r['case'],r['repeat']]['trajectory_sha256'] for r in matched)
    check_preserved_fixtures()


def check_preserved_fixtures():
    base_art = Path('artifacts/vehicle-reference/fleet57-geometry-drivetrain')
    for kind, keys in [('benchmark', ['car']), ('skidpad', ['car','diameter_ft'])]:
        a = {tuple(r[k] for k in keys):r for r in read(base_art/f'{kind}-r1.json')}
        b = {tuple(r[k] for k in keys):r for r in read(ART/f'{kind}-r1.json')}
        assert all(a[k] == r for k,r in b.items() if r['car'] not in ['ref-e12','ref-a14'])
    assert (ART/'gt500-trace-r1.json').read_bytes() == (base_art/'gt500-trace-r1.json').read_bytes()
    baseline_hashes = read('docs/verification/fleet57-geometry-drivetrain/validation.json')['file_hashes']
    for name in ['assets/cars/ref-a08.glb','assets/cars/ref-a08.json']:
        assert sha(name) == baseline_hashes[name]


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
    for name in ['fleet57-reference-20261025']:
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
    hashes['docs/verification/fleet57-geometry-drivetrain/validation.json'] = sha('docs/verification/fleet57-geometry-drivetrain/validation.json')
    for directory in [OUT, ART]:
        for p in directory.rglob('*'):
            if p.is_file() and p != OUT/'validation.json' and p.name != 'validation.log':
                hashes[str(p)] = sha(p)
    for p in [Path('assets/cars/ref-a08.glb'), Path('assets/cars/ref-a08.json'), Path('assets/cars/roster.json'), Path('assets/cars/physics.json'), Path('docs/vehicle-reference/calibration-supplement-20261025.json')]:
        hashes[str(p)] = sha(p)
    for p in Path('docs/vehicle-reference/profiles').glob('*.md'):
        hashes[str(p)] = sha(p)
    for name in ['real-profiles.json', 'real_fact.csv', 'source.csv', 'validation.json']:
        p = Path('docs/vehicle-reference')/name
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
