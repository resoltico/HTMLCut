# SPDX-License-Identifier: MPL-2.0
"""Smoke scratch ownership, completion and failure evidence at the process boundary."""
import os
import signal
import time
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
ROOT = Path(__file__).resolve().parents[1]


class FuzzCleanupTest(unittest.TestCase):
    def exercise(self, failure=False, explicit=False, runs=10000, incomplete=False, require_mutations=False, seed_only=False):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            scratch = root / "scratch"
            scratch.mkdir()
            tools = root / "bin"
            tools.mkdir()
            (tools / "python3").symlink_to(sys.executable)
            cargo = tools / "cargo"
            cargo.write_text("""#!/usr/bin/env python3
import os
from pathlib import Path
import sys
args=sys.argv[1:]
assert 'build' in args and '--codegen-units' in args
output=Path(args[args.index('--target-dir')+1])/'test-host'/'release'
output.mkdir(parents=True)
for target in ['contract_bytes','extract']:
    executable=output/target
    executable.write_text('''#!/usr/bin/env python3
import os
import sys
runs=next(arg.split('=')[1] for arg in sys.argv if arg.startswith('-runs='))
if os.environ.get('INCOMPLETE'): print('Done '+str(int(runs)-1)+' runs'); sys.exit(0)
print('#'+(runs if os.environ.get('SEED_ONLY') else '1')+'\tINITED cov: 1')
print('intentional smoke failure' if os.environ.get('FAIL_RUN') else 'Done '+runs+' runs')
sys.exit(42 if os.environ.get('FAIL_RUN') else 0)
''')
    executable.chmod(0o755)
""")
            cargo.chmod(0o755)
            rustc = tools / "rustc"
            rustc.write_text("#!/usr/bin/env bash\nprintf 'host: test-host\\n'\n")
            rustc.chmod(0o755)
            destination = root / "caller-output"
            args = [str(ROOT / "scripts/focused-smoke.sh"), '--runs', str(runs)]
            if require_mutations:
                args.append('--require-mutations')
            if explicit:
                destination.mkdir()
                (destination / "caller-marker").write_text("preserve")
                old_corpus = destination / "contract_bytes/corpus"
                old_corpus.mkdir(parents=True)
                (old_corpus / "caller-seed").write_text("preserve seed")
                args.append(str(destination))
            result = subprocess.run(args, cwd=ROOT,
                env={**os.environ, "PATH": str(tools) + os.pathsep + os.environ["PATH"],
                     "TMPDIR": str(scratch), 'FAIL_RUN': 'yes' if failure else '',
                     'INCOMPLETE': 'yes' if incomplete else '',
                     'SEED_ONLY': 'yes' if seed_only else ''},
                capture_output=True, text=True, timeout=15)
            expected = 42 if failure else 1 if incomplete or (seed_only and require_mutations) else 0
            self.assertEqual(result.returncode, expected, result.stderr)
            self.assertFalse(list(scratch.glob('htmlcut-fuzz-build.*')), 'build scratch must always be removed')
            if explicit:
                self.assertEqual((destination / "caller-marker").read_text(), "preserve")
                self.assertEqual((destination / "contract_bytes/corpus/caller-seed").read_text(), "preserve seed")
                remaining = list(destination.glob('run.*'))
            else:
                remaining = list(scratch.iterdir())
            if failure or incomplete or (seed_only and require_mutations) or explicit:
                self.assertEqual(len(remaining), 1)
                self.assertIn(str(remaining[0]), result.stderr)
                self.assertTrue((remaining[0] / "contract_bytes/corpus").is_dir())
                self.assertTrue((remaining[0] / "build.resources.json").is_file())
            else:
                self.assertEqual(remaining, [])
            if runs != 10000 and expected == 0:
                self.assertIn('Short test only:', result.stdout)

    def test_default_success_removes_output(self):
        self.exercise()

    def test_default_failure_retains_output_and_prints_path(self):
        self.exercise(failure=True)

    def test_explicit_output_survives_success_and_failure(self):
        self.exercise(explicit=True)
        self.exercise(failure=True, explicit=True)

    def test_short_run_is_identified_and_incomplete_run_refused(self):
        self.exercise(runs=3)
        self.exercise(incomplete=True)

    def test_mutation_preflight_rejects_completed_seed_replay_only(self):
        self.exercise(runs=250, require_mutations=True)
        self.exercise(runs=250, require_mutations=True, seed_only=True)

    def test_rejects_unbounded_or_invalid_run_counts(self):
        with tempfile.TemporaryDirectory() as directory:
            tools = Path(directory)
            (tools / 'python3').symlink_to(sys.executable)
            env = {**os.environ, 'PATH': str(tools) + os.pathsep + os.environ['PATH']}
            for runs in ['0', '-1', '10001', '99999999999999999999', 'oops']:
                result = subprocess.run([str(ROOT/'scripts/focused-smoke.sh'), '--runs', runs],
                                        cwd=ROOT, env=env, capture_output=True, text=True)
                self.assertEqual(result.returncode, 2, result.stderr)
                self.assertIn('--runs must be an integer', result.stderr)

    def test_refuses_scratch_inside_checkout(self):
        with tempfile.TemporaryDirectory() as directory:
            tools = Path(directory)
            (tools / 'python3').symlink_to(sys.executable)
            result = subprocess.run([str(ROOT/'scripts/focused-smoke.sh'), '--runs', '1'],
                cwd=ROOT, env={**os.environ, 'PATH': str(tools) + os.pathsep + os.environ['PATH'],
                               'TMPDIR': str(ROOT)}, capture_output=True, text=True)
            self.assertEqual(result.returncode, 2, result.stderr)
            self.assertIn('Fuzz scratch must be outside', result.stderr)

    @unittest.skipUnless(os.name == 'posix', 'POSIX shell interruption')
    def test_build_interruption_stops_child_and_retains_evidence(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            tools = root / 'tools'
            tools.mkdir()
            (tools / 'python3').symlink_to(sys.executable)
            ready = root / 'ready'
            cargo = tools / 'cargo'
            cargo.write_text("#!/usr/bin/env python3\nfrom pathlib import Path\nimport os, time\nPath(os.environ['READY']).write_text(str(os.getpid()))\ntime.sleep(30)\n")
            cargo.chmod(0o755)
            scratch = root / 'scratch'
            scratch.mkdir()
            process = subprocess.Popen([str(ROOT/'scripts/focused-smoke.sh'), '--runs', '1'],
                cwd=ROOT, env={**os.environ, 'PATH': str(tools) + os.pathsep + os.environ['PATH'],
                               'TMPDIR': str(scratch), 'READY': str(ready)},
                stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
            try:
                deadline = time.monotonic() + 5
                while not ready.exists() and time.monotonic() < deadline:
                    time.sleep(.02)
                self.assertTrue(ready.exists(), 'build child did not start')
                process.send_signal(signal.SIGTERM)
                _, stderr = process.communicate(timeout=5)
                self.assertEqual(process.returncode, 143, stderr)
                self.assertFalse(list(scratch.glob('htmlcut-fuzz-build.*')))
                evidence, = scratch.glob('htmlcut-fuzz.*')
                self.assertTrue((evidence/'build.resources.json').is_file())
                with self.assertRaises(ProcessLookupError):
                    os.kill(int(ready.read_text()), 0)
            finally:
                if process.poll() is None:
                    process.kill()
                    process.wait()
