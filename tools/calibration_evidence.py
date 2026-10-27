"""Audit frozen production evidence before exposing any study measurements."""
import subprocess
import sys
from calibration_io import ROOT, digest, load, local, require, sha
from provenance_bundle import captured_entry, verify
from diagnosis_io import records


def binding(run, role, expected):
    _, path = captured_entry(run, role=role)
    require(sha(path) == expected, f'captured {role} differs from study')
    return path


def audit(study, plan, task):
    require(task['exit_code'] == 0 and task['run'], 'failed or unavailable producer run')
    run = local(study, task['run'])
    bundle = verify(run)
    require(bundle['ok'], f'immutable bundle failed: {bundle["errors"][:3]}')
    require(all(bundle['completeness'].get(key) for key in
                ['runtime_material_complete', 'rebuild_complete']), 'incomplete frozen material')
    public = subprocess.run([sys.executable, str(ROOT / 'tools/verify_run.py'), '--json', str(run)],
                            capture_output=True, text=True, check=False)
    require(public.returncode == 0, f'production trace validation failed: {public.stdout[-500:]}')
    manifest = load(captured_entry(run, logical='manifest.json')[1])
    require(manifest['binary_sha256'] == plan['binary_sha256'], 'producer binary mismatch')
    binding(run, 'scenario', task['scenario_sha256'])
    _, config_path = captured_entry(run, role='effective_config')
    config = load(config_path)['settings']
    expected = {**plan['policy'], 'car': task['car'], 'render_disabled': True, 'mode': 'scenario'}
    require(all(config.get(k) == v for k, v in expected.items()), 'effective execution policy mismatch')
    for name in ['assets/cars/roster.json', 'assets/cars/physics.json']:
        _, path = captured_entry(run, logical=name, role='receipt_or_catalog')
        require(sha(path) == plan['bindings'][name]['sha256'], f'frozen {name} mismatch')
    audit_sources(run, plan, task)
    scenario = load(local(study, task['scenario']))
    summary = load(captured_entry(run, logical='scenario-summary.json')[1])
    end = max(item['tick'] for item in scenario['assertions'])
    require(summary['passed'] and summary['ticks'] == end, 'scenario did not complete declared window')
    samples = physics_window(captured_entry(run, logical='physics.jsonl')[1],
                             summary['readiness']['tick'], end, task['car'])
    inputs = input_digest(samples, scenario)
    audit_profile(run, task, plan, study, samples)
    return samples, inputs, bundle['completeness']


def physics_window(path, ready, end, car):
    errors = []
    selected = [row for row in records(path, errors)
                if row.get('actor_id') == 'player-vehicle' and ready < row['tick'] <= ready + end]
    require(not errors, 'malformed physics stream')
    require([row['tick'] - ready for row in selected] == list(range(1, end + 1)),
            'missing, duplicate or unordered completed player tick')
    require(all(row['car'] == car for row in selected), 'wrong player car')
    return selected


def input_digest(samples, scenario):
    timeline = {item['tick']: item['command'] for item in scenario['commands']}
    held = {'throttle': 0., 'brake': 0., 'steer': 0., 'handbrake': False}
    canonical = []
    for tick, row in enumerate(samples, 1):
        command = timeline.get(tick, {})
        if command.get('action') == 'drive':
            held = {key: command.get(key, False if key == 'handbrake' else 0.) for key in held}
        actual = row['input']
        if 'route' not in scenario:
            expected = {**held, 'reset': command.get('action') == 'reset'}
            require(actual['controller'] == 'scenario', 'unexpected controller')
            require(all(abs(actual[k] - v) < 1e-6 for k, v in expected.items()),
                    f'input mismatch at relative tick {tick}')
        canonical.append({key: actual[key] for key in [*held, 'reset', 'controller']})
    return digest(canonical)


def audit_profile(run, task, plan, study, samples):
    profile_path = local(study, plan['bindings']['assets/cars/physics.json']['path'])
    profile = next(p for p in load(profile_path)['cars'] if p['id'] == task['car'])
    errors = []
    applied = [row['fields'] for row in records(captured_entry(run, logical='events.jsonl')[1], errors)
               if row['kind'] == 'vehicle.physics_profile' and row['fields'].get('id') == task['car']]
    require(not errors and applied and all(p == profile for p in applied), 'applied profile mismatch')
    mass = profile['parameters']['mass_kg']
    require(all(abs(row['evidence']['mass'] - mass) < .01 for row in samples), 'applied mass mismatch')
    share = profile['parameters']['front_drive']
    undriven = [0, 1] if share == 0 else [2, 3] if share == 1 else []
    require(all(abs(row['drive_demands'][i]) < .01 for row in samples for i in undriven),
            'drive demand on an undriven axle')


def audit_sources(run, plan, task):
    for name, item in plan['bindings'].items():
        if name.startswith('src/'):
            _, path = captured_entry(run, role='binary_build_snapshot', logical='build_source/' + name)
            require(sha(path) == item['sha256'], f'compiled driver/course differs: {name}')
    _, asset = captured_entry(run, role='prepared_asset', logical=f"assets/cars/{task['car']}.glb")
    require(sha(asset) == plan['assets'][task['car']], 'prepared player asset differs')
