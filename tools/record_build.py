#!/usr/bin/env python3
"""Build GRID and bind allowlisted source/toolchain/features to exact executable bytes."""
import argparse
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

from build_provenance import (build_flags, command, dependency_graph, snapshot,
                              source_hashes, unavailable_inputs)
from provenance_bundle import sha256
from provenance_binding import FORMAT, fingerprint, identity

ROOT = Path(__file__).resolve().parents[1]


def build_arguments(options):
    args = ['cargo', 'build', '--locked', '--offline', '--message-format=json',
            '--bin', 'car-game', '--profile', options.profile]
    for name in ('features', 'target'):
        value = getattr(options, name)
        if value:
            args += ['--' + name, value]
    if options.no_default_features:
        args.append('--no-default-features')
    return args


def compile_binary(root, args, fingerprint):
    build_env = dict(os.environ, CAR_GAME_BUILD_SOURCE_SHA256=fingerprint)
    process = subprocess.Popen(args, cwd=root, env=build_env, stdout=subprocess.PIPE, text=True)
    artifact = None
    units = []
    for line in process.stdout:
        try:
            message = json.loads(line)
        except ValueError:
            continue
        if message.get('reason') == 'compiler-artifact':
            units.append({'target': message['target']['name'], 'kind': message['target']['kind'],
                          'features': message['features'], 'profile': message['profile']})
        if message.get('reason') == 'compiler-artifact' and message.get('executable') and message.get('target', {}).get('name') == 'car-game':
            artifact = {'executable': message['executable'], 'features': message['features'],
                        'target': message['target']['name'], 'profile': message['profile']}
    if process.wait() or artifact is None:
        raise RuntimeError('build_failed')
    artifact['compiler_artifact_units'] = units
    return artifact


def record(options):
    target = Path(os.environ.get('CARGO_TARGET_DIR', ROOT / 'target')).resolve()
    if not target.is_relative_to(ROOT):
        raise RuntimeError('shared_target_not_allowed_use_checkout_local_target')
    with tempfile.TemporaryDirectory(prefix='grid-build-') as temporary:
        stage = Path(temporary)
        before = source_hashes(ROOT)
        inventory = snapshot(ROOT, stage)
        flags, omissions = build_flags()
        omissions += unavailable_inputs(ROOT)
        graph = dependency_graph(ROOT, options)
        if any(package['source'].startswith('unavailable') for package in graph['packages']):
            omissions.append('non_registry_dependency_material_unavailable')
        compiler = command(ROOT, ['rustc', '-vV'])
        cargo = command(ROOT, ['cargo', '-V'])
        sysroot = Path(command(ROOT, ['rustc', '--print', 'sysroot']))
        compiler_sha = sha256(sysroot / 'bin/rustc')
        args = build_arguments(options)
        declarations = {'compiler_version': compiler, 'cargo_version': cargo,
                        'git_commit': command(ROOT, ['git', 'rev-parse', 'HEAD']),
                        'dirty': bool(command(ROOT, ['git', 'status', '--porcelain'])),
                        'source_binding': 'compiler_embedded_source_fingerprint_and_stable_inputs',
                        'omissions': list(omissions),
                        'rebuild_prerequisites': ['compatible_toolchain_and_target_sdk',
                                                 'registry_dependencies_identified_by_Cargo_lock']}
        source_fingerprint = fingerprint(identity(inventory, args, flags, graph, compiler_sha, declarations))
        artifact = compile_binary(ROOT, args, source_fingerprint)
        if source_hashes(ROOT) != before:
            omissions.append('build_inputs_changed_during_compilation')
        compiled = Path(artifact.pop('executable'))
        executable = compiled.with_name(compiled.name + '.recorded')
        with tempfile.NamedTemporaryFile(dir=compiled.parent, prefix='.recorded-', delete=False) as output:
            staged_binary = Path(output.name)
        shutil.copyfile(compiled, staged_binary)
        if source_fingerprint.encode() not in staged_binary.read_bytes():
            raise RuntimeError('compiler_output_source_binding_mismatch')
        staged_binary.chmod(0o755)
        os.replace(staged_binary, executable)
        metadata = {'schema': 1, 'binary_sha256': sha256(executable),
                    'availability': 'included', 'rebuild_complete': not omissions,
                    **declarations, 'omissions': omissions,
                    'source_fingerprint': source_fingerprint, 'binding_format': FORMAT,
                    'binding_declarations': declarations, 'compiler_sha256': compiler_sha,
                    'invocation': args, 'rust_flags': flags,
                    'dependency_graph': graph, 'artifact': artifact, 'inventory': inventory}
        (stage / 'build.json').write_text(json.dumps(metadata, indent=2) + '\n')
        destination = executable.with_name(executable.name + '.provenance')
        if destination.exists():
            shutil.rmtree(destination)
        shutil.copytree(stage, destination)
        return {'ok': True, 'binary_sha256': metadata['binary_sha256'],
                'rebuild_complete': metadata['rebuild_complete'], 'omissions': omissions,
                'recorded_executable': executable.relative_to(ROOT).as_posix()}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--profile', default='dev')
    parser.add_argument('--features')
    parser.add_argument('--target')
    parser.add_argument('--no-default-features', action='store_true')
    options = parser.parse_args()
    values = [value for value in (options.profile, options.features, options.target) if value]
    if not all(re.fullmatch(r'[a-zA-Z0-9_,.-]+', value) for value in values):
        parser.error('profile/features/target must be portable identifiers')
    try:
        result = record(options)
    except RuntimeError as error:
        code = str(error) if str(error) in {'shared_target_not_allowed_use_checkout_local_target', 'compiler_output_source_binding_mismatch'} else 'build_provenance_unavailable'
        result = {'ok': False, 'errors': [{'code': code}]}
    except (OSError, ValueError):
        result = {'ok': False, 'errors': [{'code': 'build_provenance_unavailable'}]}
    print(json.dumps(result))
    return 0 if result['ok'] else 1


if __name__ == '__main__':
    sys.exit(main())
