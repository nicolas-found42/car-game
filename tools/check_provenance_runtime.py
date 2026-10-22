#!/usr/bin/env python3
"""Exercise compiler-bound sidecars through the actual production smoke CLI."""
import argparse
import json
import os
import shutil
import subprocess
from pathlib import Path

from provenance_bundle import sha256
from provenance_binding import FORMAT, recompute


def launch(executable, output):
    output.mkdir(parents=True)
    with (output / 'process.log').open('w') as log:
        result = subprocess.run([str(executable), '--smoke', '--offscreen'], cwd=output,
                                env=dict(os.environ, CAR_GAME_MUTE='1'), stdout=log,
                                stderr=subprocess.STDOUT, timeout=90)
    latest = Path((output / 'artifacts/latest').read_text().strip())
    run = latest if latest.is_absolute() else output / latest
    capture = json.loads((run / 'provenance/capture.json').read_text())
    return result.returncode, capture


def exercise(executable, output, tamper):
    output.mkdir(parents=True)
    copied = output / 'car-game'
    shutil.copyfile(executable, copied)
    copied.chmod(0o755)
    sidecar = output / 'car-game.provenance'
    shutil.copytree(executable.with_name(executable.name + '.provenance'), sidecar)
    code, control = launch(copied, output / 'valid')
    expected = tamper == 'remove-source'
    assert code == 0 and control['binary_build']['rebuild_complete'] == expected, 'valid build identity/status rejected'
    assert control['binary_build']['availability'] == 'included', 'source binding was not validated'
    metadata_path = sidecar / 'build.json'
    metadata = json.loads(metadata_path.read_text())
    asserted = metadata['source_fingerprint']
    if tamper == 'remove-source':
        metadata['inventory'] = [entry for entry in metadata['inventory']
                                 if entry['logical_path'] != 'build_source/src/main.rs']
    else:
        metadata['rebuild_complete'] = True
    metadata_path.write_text(json.dumps(metadata))
    assert metadata['binary_sha256'] == sha256(copied)
    assert metadata['source_fingerprint'] == asserted
    code, tampered = launch(copied, output / 'tampered')
    accepted = tampered['binary_build']['rebuild_complete']
    calculated = recompute(metadata) if metadata.get('binding_format') == FORMAT else None
    result = {'ok': code == 0 and not accepted, 'binary_sha256_unchanged': sha256(copied),
              'asserted_fingerprint_unchanged': asserted, 'recomputed_after_tamper': calculated,
              'bound_omissions': metadata.get('binding_declarations', {}).get('omissions'),
              'valid_control': control['binary_build']['rebuild_complete'],
              'tampered_sidecar_accepted': accepted,
              'tampered_status': {key: tampered['binary_build'].get(key) for key in
                                   ('availability', 'rebuild_complete', 'reason', 'omissions')}}
    (output / 'result.json').write_text(json.dumps(result, indent=2))
    print(json.dumps(result))
    return 0 if result['ok'] else 1


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--executable', required=True, type=Path)
    parser.add_argument('--output', required=True, type=Path)
    parser.add_argument('--tamper', choices=['remove-source', 'complete-flag'], default='remove-source')
    args = parser.parse_args()
    return exercise(args.executable.resolve(), args.output.resolve(), args.tamper)


if __name__ == '__main__':
    raise SystemExit(main())
