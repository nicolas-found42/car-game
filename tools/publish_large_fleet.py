"""Publish the prepared 300-car roster after verified 57-car calibration."""
import hashlib
import json
import os
from pathlib import Path
import shutil

ROOT=Path(__file__).resolve().parents[1]
OUT=ROOT/'docs/verification/fleet-300'


def read(path):
    return json.loads(path.read_text())


def sha(path):
    with path.open('rb') as stream:return hashlib.file_digest(stream,'sha256').hexdigest()


def main():
    validation=ROOT/'docs/verification/fleet57-next-calibration/validation.json'
    assert read(validation)['status']=='passed'
    roster=read(OUT/'prepared-roster.json');profiles=read(OUT/'prepared-physics.json')
    current=ROOT/'assets/cars';old_roster=read(current/'roster.json');old_profiles=read(current/'physics.json')
    assert len(roster)==len(profiles['cars'])==300 and len(old_roster)==57
    assert roster[:57]==old_roster and profiles['cars'][:57]==old_profiles['cars']
    source=ROOT/'.scratch/fleet-300-build/cars';installed={}
    for car in roster[57:]:
        for suffix in ['.glb','.json']:
            name=car['id']+suffix;destination=current/name
            assert not destination.exists(),destination
            os.link(source/name,destination);installed[str(destination.relative_to(ROOT))]=sha(destination)
    for name in ['roster','physics']:
        shutil.copy2(OUT/f'prepared-{name}.json',current/f'{name}.json')
    receipt={'cars':300,'original_profiles_and_geometry_rows_exact':57,
             'preceding_57car_validation_sha256':sha(validation),'installed':installed,
             'roster_sha256':sha(current/'roster.json'),'physics_sha256':sha(current/'physics.json')}
    (OUT/'publication.json').write_text(json.dumps(receipt,indent=2)+'\n')
    print('Published300 cars; all57 accepted rows unchanged',flush=True)


if __name__=='__main__':main()
