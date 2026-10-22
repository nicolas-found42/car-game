"""Public evidence CLI contracts: portable immutable inventory and safe failures."""
import hashlib
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

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
                      'completeness': {'runtime_replay_complete': False},
                      'replay_prerequisites': [{'id': 'gpu', 'availability': 'external_required'}]}
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
