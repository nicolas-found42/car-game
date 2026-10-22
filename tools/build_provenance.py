"""Sanitized build inputs and dependency identities for record_build.py."""
import json
import os
import re
import shlex
import subprocess
from pathlib import Path

from provenance_bundle import sha256
from provenance_legacy import store_bytes


def command(root, arguments):
    result = subprocess.run(arguments, cwd=root, capture_output=True, text=True)
    if result.returncode:
        raise RuntimeError(f'{arguments[0]} failed')
    return result.stdout.strip()


def source_paths(root):
    paths = list((root / 'src').rglob('*.rs'))
    paths += [root / name for name in ('Cargo.toml', 'Cargo.lock', 'build.rs',
              'rust-toolchain', 'rust-toolchain.toml', 'assets/cars/roster.json')
              if (root / name).is_file()]
    for path in paths:
        if path.is_symlink() or any(parent.is_symlink() for parent in path.parents if parent != root):
            raise ValueError('source_symlink_disallowed')
    return sorted(paths)


def snapshot(root, destination):
    inventory = [store_bytes(destination, path.read_bytes(), 'binary_build_snapshot',
                             'build_source/' + path.relative_to(root).as_posix())
                 for path in source_paths(root)]
    patch = command(root, ['git', 'diff', '--binary', 'HEAD', '--', 'src', 'Cargo.toml',
                           'Cargo.lock', 'build.rs', 'assets/cars/roster.json']).encode()
    inventory.append(store_bytes(destination, patch, 'binary_build_snapshot', 'build_source/dirty.patch'))
    return inventory


def source_hashes(root):
    return {path.relative_to(root).as_posix(): sha256(path) for path in source_paths(root)}


def dependency_graph(root, options):
    args = ['cargo', 'metadata', '--format-version=1', '--locked', '--offline']
    if options.features:
        args += ['--features', options.features]
    if options.no_default_features:
        args.append('--no-default-features')
    target = options.target or command(root, ['rustc', '-vV']).split('host: ', 1)[1].splitlines()[0]
    args += ['--filter-platform', target]
    metadata = json.loads(command(root, args))
    ids = {package['id']: f"{package['name']}@{package['version']}" for package in metadata['packages']}
    packages = [{'id': ids[package['id']], 'checksum': package.get('checksum'),
                 'source': safe_dependency_source(package.get('source'))}
                for package in metadata['packages']]
    nodes = [{'id': ids[node['id']], 'features': node['features'],
              'dependencies': [ids[key] for key in node['dependencies']]}
             for node in metadata['resolve']['nodes']]
    return {'packages': packages, 'resolved_features': nodes,
            'material_availability': 'external_immutable_registry_versions_and_lock_checksums'}


def safe_dependency_source(source):
    if source is None:
        return 'captured_workspace'
    if source == 'registry+https://github.com/rust-lang/crates.io-index':
        return source
    return 'unavailable_non_registry_source_redacted'


def build_flags():
    raw = os.environ.get('CARGO_ENCODED_RUSTFLAGS', os.environ.get('RUSTFLAGS', ''))
    parts = raw.split('\x1f') if '\x1f' in raw else shlex.split(raw)
    allowed = re.compile(r'(opt-level=[0-3sz]|debuginfo=[0-2]|codegen-units=[0-9]+|lto=(off|thin|fat)|target-cpu=[A-Za-z0-9_-]+)')
    flags, omitted = [], []
    index = 0
    while index < len(parts):
        if parts[index] == '-C' and index + 1 < len(parts) and allowed.fullmatch(parts[index + 1]):
            flags += parts[index:index + 2]
            index += 2
        else:
            omitted.append('unsupported_rustflag_redacted')
            index += 1
    return flags, omitted


def unavailable_inputs(root):
    reasons = []
    for name in ('.cargo/config', '.cargo/config.toml'):
        if (root / name).exists():
            reasons.append('cargo_config_not_allowlisted')
    if any((parent / '.cargo/config.toml').is_file() or (parent / '.cargo/config').is_file()
           for parent in root.parents):
        reasons.append('ancestor_cargo_config_unavailable')
    cargo_home = Path(os.environ.get('CARGO_HOME', Path.home() / '.cargo'))
    if any((cargo_home / name).is_file() for name in ('config', 'config.toml')):
        reasons.append('user_cargo_config_unavailable')
    allowed_cargo_env = {'CARGO_TARGET_DIR', 'CARGO_HOME', 'CARGO_ENCODED_RUSTFLAGS', 'CARGO_INCREMENTAL', 'CARGO_BUILD_JOBS'}
    for name in os.environ:
        if name.startswith(('RUSTC', 'CARGO_')) and name not in allowed_cargo_env:
            reasons.append('unsupported_build_environment_redacted')
    return sorted(set(reasons))
