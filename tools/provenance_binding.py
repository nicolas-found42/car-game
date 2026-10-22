"""Canonical source/configuration binding used by compiler and bundle verification."""
import hashlib
import json

FORMAT = 'grid-build-inputs-v1'


def identity(inventory, invocation, flags, dependencies, compiler_sha256, declarations):
    return {'inventory': inventory, 'invocation': invocation, 'flags': flags,
            'dependencies': dependencies, 'compiler_sha256': compiler_sha256, 'declarations': declarations}


def fingerprint(inputs):
    canonical = json.dumps(inputs, sort_keys=True, separators=(',', ':'), ensure_ascii=False)
    return hashlib.sha256(canonical.encode('utf-8')).hexdigest()


def recompute(metadata):
    if metadata.get('binding_format') != FORMAT:
        raise ValueError('unsupported_build_binding_format')
    inputs = identity(metadata['inventory'], metadata['invocation'], metadata['rust_flags'],
                      metadata['dependency_graph'], metadata['compiler_sha256'], metadata['binding_declarations'])
    return fingerprint(inputs)


def derived_rebuild_complete(metadata):
    declarations = metadata['binding_declarations']
    if not isinstance(declarations, dict):
        raise ValueError('invalid_build_declarations')
    fields = ('compiler_version', 'cargo_version', 'git_commit', 'dirty', 'source_binding',
              'omissions', 'rebuild_prerequisites')
    if any(metadata.get(key) != declarations.get(key) for key in fields):
        raise ValueError('unbound_build_declaration')
    if any(not isinstance(declarations.get(key), str) or not declarations[key] for key in
           ('compiler_version', 'cargo_version', 'git_commit', 'source_binding')):
        raise ValueError('required_build_declaration_missing')
    if type(declarations.get('dirty')) is not bool or not isinstance(declarations.get('rebuild_prerequisites'), list) or not declarations['rebuild_prerequisites']:
        raise ValueError('required_build_declaration_missing')
    if not isinstance(declarations.get('omissions'), list):
        raise ValueError('invalid_build_omissions')
    return not declarations['omissions']
