"""Black-box checks for compiler profile selection and failure propagation."""
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest


SCRIPT = Path(__file__).resolve().parents[1] / 'scripts/check_codelimit.sh'


class ComplexityGateTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        (self.root / 'scripts').mkdir()
        shutil.copyfile(SCRIPT, self.root / 'scripts/check_codelimit.sh')
        (self.root / 'bin').mkdir()
        for tool in ('codelimit', 'lizard'):
            self.executable(tool, '#!/bin/sh\nexit 0\n')
        self.executable('cargo', '#!/bin/sh\nprintf "%s\\n" "$@" > args\n'
                        'echo "${GATE_DIAGNOSTIC:-}" >&2\nexit "${GATE_EXIT:-0}"\n')
        subprocess.run(['git', 'init', '-q'], cwd=self.root, check=True)
        (self.root / 'changed.rs').write_text('fn original() {}\n')
        subprocess.run(['git', 'add', '.'], cwd=self.root, check=True)
        subprocess.run(['git', '-c', 'user.name=Test', '-c', 'user.email=test@example.com',
                        'commit', '-qm', 'fixture'], cwd=self.root, check=True)
        (self.root / 'changed.rs').write_text('fn modified() {}\n')

    def executable(self, name, content):
        path = self.root / 'bin' / name
        path.write_text(content)
        path.chmod(0o755)

    def gate(self, **settings):
        env = dict(os.environ, PATH=str(self.root / 'bin') + os.pathsep + os.environ['PATH'])
        env.update(settings)
        return subprocess.run(['bash', 'scripts/check_codelimit.sh', 'HEAD'],
                              cwd=self.root, env=env, capture_output=True, text=True)

    def test_ci_profile_is_forwarded_to_clippy(self):
        result = self.gate(CODELIMIT_CARGO_PROFILE='ci')
        self.assertEqual(result.returncode, 0, result.stderr)
        args = (self.root / 'args').read_text().splitlines()
        self.assertIn('--profile', args)
        self.assertEqual(args[args.index('--profile') + 1], 'ci')

    def test_compiler_failure_cannot_be_reported_as_a_pass(self):
        result = self.gate(GATE_EXIT='101', GATE_DIAGNOSTIC='error: compilation failed')
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('compilation failed', result.stderr)

    def test_changed_function_length_violation_fails(self):
        result = self.gate(GATE_DIAGNOSTIC='changed.rs:1:1: warning: this function has too many lines')
        self.assertNotEqual(result.returncode, 0)

    def test_unchanged_function_length_warning_is_not_new_debt(self):
        result = self.gate(GATE_DIAGNOSTIC='untouched.rs:1:1: warning: this function has too many lines')
        self.assertEqual(result.returncode, 0, result.stderr)


if __name__ == '__main__':
    unittest.main()
