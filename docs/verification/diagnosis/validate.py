"""Bind the diagnostic upgrade to historical evidence and unchanged trajectories."""
import hashlib
import json
from pathlib import Path
import sys

ROOT=Path(__file__).resolve().parents[3]
sys.path.insert(0,str(ROOT/'tools'))
from provenance_bundle import verify
ART=ROOT/'artifacts/diagnosis-upgrade'
OUT=Path(__file__).resolve().parent


def read(path):
    return json.loads(path.read_text())


def sha(path):
    digest=hashlib.sha256()
    with Path(path).open('rb') as stream:
        for chunk in iter(lambda:stream.read(1024*1024),b''):digest.update(chunk)
    return digest.hexdigest()


def historical():
    nsx=read(ART/'historical-nsx.json');mini=read(ART/'historical-mini.json');clock=read(ART/'historical-clock.json')
    assert nsx['gear_transitions']==592 and not nsx['errors'] and not mini['errors'] and not clock['errors']
    assert any(f['reason']=='gear_hunting' for f in nsx['findings'])
    assert any(f['reason']=='contact_loss' and f['car']=='exp-037' for f in mini['findings'])
    assert any(f['reason']=='clock_gap' and f['evidence']['elapsed_s']>20 and f['evidence']['tick_advance']==1 for f in clock['findings'])
    control=read(ART/'historical-gt500-control.json');assert control['physics_samples_examined']>2400
    assert not any(f['reason'] in ['gear_hunting','contact_loss','zero_drive_demand'] for f in control['findings'])
    for name,digest in read(ART/'historical-evidence-bindings.json')['files'].items():assert sha(ROOT/name)==digest
    return {'nsx_gear_transitions':592,'mini_contact_loss_detected':True,'historical_clock_gap_detected':True}


def unchanged():
    baseline=read(ART/'baseline/hashes.json')
    for name in ['src/vehicle/profile.rs','src/vehicle/forces.rs','assets/cars/physics.json','assets/cars/roster.json']:
        assert sha(ROOT/name)==baseline[name],name
    comparison=read(ART/'probe-comparison.json')
    assert comparison['equal'] and comparison['matched_samples']>2400 and not comparison['errors']
    stress=read(ART/'stress-comparison.json')
    assert stress['status']=='passed' and stress['cars']==8 and stress['case_pairs']==104
    assert len(stress['comparisons'])==104 and all(r['equal'] for r in stress['comparisons'])
    mini=read(ART/'mini-probe-comparison.json');assert mini['equal'] and mini['matched_samples']>1800
    analysis=read(ART/'probe-analysis.json')
    assert any(f.get('explanation',{}).get('mechanism')=='repeated_shifts_interrupt_engine_demand' for f in analysis['findings'])
    portable=read(ART/'portable-window-proof.json');assert portable['status']=='passed' and portable['included_window_entries']==8
    return {'profile_force_equations_and_roster_exact':True,'historical_probe_samples_exact':comparison['matched_samples'],
            'stress_case_pairs_exact':104,'protected_four_included':True,'mini_probe_samples_exact':mini['matched_samples'],
            'portable_decisions_and_windows_verified':True}


def recorded_source():
    binding=read(ART/'recorded-source-match.json')
    assert binding['binary_sha256']==sha(ART/'bin/car-game.recorded')
    for name,digest in binding['files'].items():assert sha(ROOT/name)==digest


def native():
    recorded_source()
    assert read(ART/'verification-exit.json')['exit_code']==0
    log=(ART/'verification.log').read_text()
    assert 'verification log:' in log and 'RUN EVIDENCE PASS:' in log
    run=(ROOT/(ART/'native-run.txt').read_text().strip()).resolve()
    rows=read(run/'report.json');assert len(rows)==300
    assert all(r['passed'] and r['nonblank_captures']==4 and r['animated_nodes']==r['expected_nodes'] for r in rows)
    result=verify(run);assert result['ok'] and result['completeness']['runtime_material_complete']
    assert result['completeness']['rebuild_complete']
    assert read(run/'manifest.json')['binary_sha256']==sha(ART/'bin/car-game.recorded')
    health=read(run/'recorder-health.json');assert not health['failed'] and health['last_durable_tick']==health['completed_tick']
    assert not health['losses'] and not health['automatic_window_pending']
    windows=read(run/'diagnostic-windows.json');assert windows['triggered']==windows['exported'] and not windows['active']
    return {'run':str(run.relative_to(ROOT)),'cars':300,'captured_views':1200,'provenance':result,
            'automatic_windows':windows,'producer_overhead':health['overhead'].get('producer')}


def main():
    assert read(ART/'supplemental-headless-exit.json')['exit_code']==0
    result={'status':'passed','historical':historical(),'physics_preservation':unchanged(),'native':native()}
    hashes={}
    for folder in [ART,OUT]:
        for path in folder.rglob('*'):
            if path.is_file() and path!=OUT/'validation.json' and path.suffix not in ['.log','.pyc']:
                hashes[str(path.relative_to(ROOT))]=sha(path)
    for path in [ROOT/'tools/diagnose.py',ROOT/'tools/diagnosis_io.py',ROOT/'tools/diagnosis_rules.py']:
        hashes[str(path.relative_to(ROOT))]=sha(path)
    result['file_hashes']=hashes
    (OUT/'validation.json').write_text(json.dumps(result,indent=2)+'\n')
    print('PASS: historical failures detected; physics preserved; native evidence verified',flush=True)


if __name__=='__main__':main()
