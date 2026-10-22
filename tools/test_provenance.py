"""Public evidence CLI contracts: portable immutable inventory and safe failures."""
import hashlib
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

from test_provenance_fixtures import complete_runtime_index
from provenance_legacy import store_bytes

CLI = Path(__file__).with_name('evidence_manifest.py').resolve()


class PortableEvidenceTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.run = self.root / 'run'
        self.run.mkdir()
        self.entries = []
        for role, data in [('executable', b'bin'), ('prepared_asset', b'glb'),
                           ('effective_config', b'{"car":"350z"}')]:
            digest = hashlib.sha256(data).hexdigest()
            logical = f'objects/sha256/{digest}'
            path = self.run / logical
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(data)
            self.entries.append({'role': role, 'logical_path': role,
                                 'sha256': digest, 'size_bytes': len(data),
                                 'object': logical, 'availability': 'included'})
        self.index = {'schema': 2, 'provenance_version': 1, 'run_id': 'fixture',
                      'inventory': self.entries, 'cars': [],
                      'policy': 'grid-portable-allowlist-v1',
                      'completeness': {'inventory_integrity': True, 'runtime_material_complete': False,
                                       'runtime_replay_complete': False, 'rebuild_complete': False},
                      'omissions': [{'role': 'fixture_partial_inventory', 'reason': 'explicit_partial_fixture',
                                     'replay_required': True}],
                      'replay_prerequisites': [{'id': 'gpu', 'availability': 'external_required', 'replay_required': True}]}
        self.write_index()

    def write_index(self):
        path = self.run / 'evidence-manifest.json'
        path.write_text(json.dumps(self.index))
        path.with_suffix('.sha256').write_text(hashlib.sha256(path.read_bytes()).hexdigest())

    def cli(self, action, run=None, *extra):
        result = subprocess.run([sys.executable, str(CLI), '--action', action,
                                 '--run', str(run or self.run), *extra],
                                cwd='/', capture_output=True, text=True)
        return result.returncode, json.loads(result.stdout)

    def test_export_verifies_without_original_run_or_working_directory(self):
        exported = self.root / 'portable'
        code, result = self.cli('export', None, '--output', str(exported))
        self.assertEqual((code, result['ok']), (0, True))
        self.run.rename(self.root / 'unavailable-original')
        code, result = self.cli('verify', exported)
        self.assertEqual((code, result['ok']), (0, True))
        self.assertFalse(result['completeness']['runtime_replay_complete'])

    def test_empty_inventory_cannot_claim_complete_material_or_replay(self):
        self.index['inventory'] = []
        self.index['completeness'] = dict.fromkeys(self.index['completeness'], True)
        self.index['omissions'] = []
        self.write_index()
        code, result = self.cli('verify')
        self.assertEqual(code, 1)
        self.assertIn('invalid_completeness_claim', [error['code'] for error in result['errors']])

    def test_complete_material_requires_every_catalog_asset_and_receipt(self):
        self.index = complete_runtime_index(self.run)
        self.write_index()
        code, result = self.cli('verify')
        self.assertEqual((code, result['ok']), (0, True))
        self.index['inventory'] = [entry for entry in self.index['inventory']
                                   if entry['role'] != 'prepared_asset']
        self.write_index()
        code, result = self.cli('verify')
        self.assertEqual(code, 1)
        self.assertEqual(result['errors'][0]['code'], 'invalid_completeness_claim')

    def test_duplicate_and_invalid_object_bindings_are_rejected(self):
        self.entries.append(dict(self.entries[0]))
        self.write_index()
        code, result = self.cli('verify')
        self.assertEqual((code, result['errors'][0]['code']), (1, 'duplicate_inventory_binding'))
        self.entries.pop()
        self.entries[0]['object'] = self.entries[1]['object']
        self.write_index()
        code, result = self.cli('verify')
        self.assertEqual((code, result['errors'][0]['code']), (1, 'object_binding_mismatch'))

    def test_config_substitution_cannot_keep_an_unrelated_capture_binding(self):
        self.index = complete_runtime_index(self.run)
        config = next(entry for entry in self.index['inventory'] if entry['role'] == 'effective_config')
        binary = next(entry for entry in self.index['inventory'] if entry['role'] == 'executable')
        binary.update({key: config[key] for key in ('sha256', 'object', 'size_bytes')})
        self.write_index()
        code, result = self.cli('verify')
        self.assertEqual((code, result['errors'][0]['code']), (1, 'capture_inventory_mismatch'))

    def test_explicit_incomplete_inventory_can_be_inspected_without_claiming_replay(self):
        self.entries.pop(0)
        self.write_index()
        code, result = self.cli('verify')
        self.assertEqual((code, result['ok']), (0, True))
        self.assertEqual(result['material_status'], 'explicitly_incomplete_runtime_material')

    def test_complete_replay_cannot_hide_external_prerequisites(self):
        self.index = complete_runtime_index(self.run)
        self.index['completeness']['runtime_replay_complete'] = True
        self.write_index()
        code, result = self.cli('verify')
        self.assertEqual((code, result['errors'][0]['code']), (1, 'invalid_completeness_claim'))

    def test_wrong_status_and_inventory_types_return_machine_readable_errors(self):
        self.index['completeness']['rebuild_complete'] = 'true'
        self.write_index()
        code, result = self.cli('verify')
        self.assertEqual((code, result['errors'][0]['code']), (1, 'invalid_completeness_claim'))
        self.index['completeness']['rebuild_complete'] = False
        self.index['inventory'] = {}
        self.write_index()
        code, result = self.cli('verify')
        self.assertEqual((code, result['errors'][0]['code']), (1, 'invalid_bundle_schema'))

    def test_null_capture_document_cannot_claim_complete_runtime_material(self):
        self.index = complete_runtime_index(self.run)
        replacement = store_bytes(self.run, b'null', 'capture_index', 'provenance/capture.json')
        self.index['inventory'] = [replacement if entry['role'] == 'capture_index' else entry
                                   for entry in self.index['inventory']]
        self.index['capture_sha256'] = replacement['sha256']
        self.write_index()
        code, result = self.cli('verify')
        self.assertEqual((code, result['errors'][0]['code']), (1, 'invalid_material_schema'))

    def assert_replay_claim_rejected(self, mutation):
        self.index = complete_runtime_index(self.run)
        self.index['completeness']['runtime_replay_complete'] = True
        binary = next(entry for entry in self.index['inventory'] if entry['role'] == 'executable')
        for prerequisite in self.index['replay_prerequisites']:
            prerequisite.update(mutation(binary))
        frozen = next(entry for entry in self.index['inventory'] if entry['role'] == 'capture_index')
        capture = json.loads((self.run / frozen['object']).read_text())
        capture['completeness'] = self.index['completeness']
        capture['replay_prerequisites'] = self.index['replay_prerequisites']
        replacement = store_bytes(self.run, json.dumps(capture).encode(), 'capture_index', 'provenance/capture.json')
        self.index['inventory'] = [replacement if entry['role'] == 'capture_index' else entry
                                   for entry in self.index['inventory']]
        self.index['capture_sha256'] = replacement['sha256']
        self.write_index()
        code, result = self.cli('verify', None, '--require-replay')
        self.assertEqual(code, 1)
        self.assertEqual(result['errors'][0]['code'], 'invalid_completeness_claim')

    def test_system_prerequisites_cannot_be_relabelled_optional(self):
        self.assert_replay_claim_rejected(lambda _: {'replay_required': False})

    def test_system_prerequisites_cannot_bind_an_arbitrary_bundled_object(self):
        self.assert_replay_claim_rejected(lambda binary: {'availability': 'included',
                                         'logical_path': binary['logical_path'], 'sha256': binary['sha256']})

    def test_legacy_export_redacts_existing_metadata_and_never_claims_original_assets(self):
        private = 'PRIVATE_CREDENTIAL_4a82'
        for path in (self.run / 'evidence-manifest.json', self.run / 'evidence-manifest.sha256'):
            path.unlink()
        (self.run / 'manifest.json').write_text(json.dumps({
            'run_id': 'legacy', 'args': ['--unknown', private],
            'env': {'cwd': '/Users/' + private, 'token': private},
            'exit': {'code': 0}}))
        original = {'schema': 1, 'run_id': 'legacy', 'args': [private], 'cars': []}
        (self.run / 'evidence-manifest.json').write_text(json.dumps(original))
        exported = self.root / 'legacy-portable'
        code, result = self.cli('export', None, '--output', str(exported))
        self.assertEqual((code, result['ok']), (0, True))
        self.assertFalse(result['completeness']['runtime_material_complete'])
        for path in exported.rglob('*'):
            if path.is_file():
                self.assertNotIn(private.encode(), path.read_bytes())
        self.assertEqual(json.loads((self.run / 'evidence-manifest.json').read_text()), original)

    def test_mismatched_objects_report_actionable_role_specific_errors(self):
        expected = ['binary_hash_mismatch', 'content_hash_mismatch', 'config_hash_mismatch']
        for entry, error in zip(self.entries, expected, strict=True):
            with self.subTest(role=entry['role']):
                path = self.run / entry['object']
                original = path.read_bytes()
                path.write_bytes(b'intentional-corruption')
                code, result = self.cli('verify')
                self.assertEqual(code, 1)
                self.assertEqual(result['errors'][0]['code'], error)
                path.write_bytes(original)
        (self.run / self.entries[0]['object']).unlink()
        code, result = self.cli('verify')
        self.assertEqual((code, result['errors'][0]['code']), (1, 'missing_required_object'))

    def test_unsafe_object_paths_and_symlinks_are_rejected_without_private_paths(self):
        for logical in ('/private/SECRET', '../SECRET', 'objects/sha256/../../SECRET'):
            self.entries[0]['object'] = logical
            self.write_index()
            code, result = self.cli('verify')
            self.assertEqual((code, result['errors'][0]['code']), (1, 'unsafe_bundle_path'))
            self.assertNotIn('SECRET', json.dumps(result))
        entry = self.entries[1]
        path = self.run / entry['object']
        path.unlink()
        path.symlink_to('/etc/passwd')
        self.entries.pop(0)
        self.write_index()
        code, result = self.cli('verify')
        self.assertEqual((code, result['errors'][0]['code']), (1, 'unsafe_bundle_path'))

    def test_missing_replay_prerequisites_are_separate_from_valid_integrity(self):
        code, result = self.cli('verify', None, '--require-replay')
        self.assertEqual(code, 1)
        self.assertIn('replay_prerequisite_unavailable', [error['code'] for error in result['errors']])
        code, result = self.cli('verify')
        self.assertEqual((code, result['ok']), (0, True))

    def test_index_mutation_and_unsupported_versions_are_distinct(self):
        (self.run / 'evidence-manifest.json').write_text('{}')
        code, result = self.cli('verify')
        self.assertEqual((code, result['errors'][0]['code']), (1, 'unsupported_bundle_version'))
        self.write_index()
        with (self.run / 'evidence-manifest.json').open('a') as stream:
            stream.write(' ')
        code, result = self.cli('verify')
        self.assertEqual((code, result['errors'][0]['code']), (1, 'index_hash_mismatch'))


class BuildBoundaryTests(unittest.TestCase):
    def test_recorded_build_rejects_a_shared_external_output_directory(self):
        import os
        with tempfile.TemporaryDirectory() as temporary:
            environment = dict(os.environ, CARGO_TARGET_DIR=temporary)
            result = subprocess.run([sys.executable, str(CLI.with_name('record_build.py'))],
                                    cwd='/', env=environment, capture_output=True, text=True)
        self.assertEqual(result.returncode, 1)
        self.assertEqual(json.loads(result.stdout)['errors'][0]['code'],
                         'shared_target_not_allowed_use_checkout_local_target')


if __name__ == '__main__':
    unittest.main()
