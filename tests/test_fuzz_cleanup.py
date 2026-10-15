# SPDX-License-Identifier: MPL-2.0
"""Smoke scratch ownership, completion and failure evidence at the process boundary."""
import os
import json
import signal
import time
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
ROOT = Path(__file__).resolve().parents[1]


class FuzzCleanupTest(unittest.TestCase):
    def exercise(self, failure=False, explicit=False, runs=10000, incomplete=False, require_mutations=False, seed_only=False, slow_first=False, stale_reap=False):
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
import time
from pathlib import Path
if os.environ.get('STALE_REAP') and sys.argv[0].endswith('extract'):
    time.sleep(2)
    sys.exit(42)
if os.environ.get('SLOW_FIRST'):
    if sys.argv[0].endswith('contract_bytes'):
        Path(os.environ['CHILD_READY']).write_text(str(os.getpid()))
        time.sleep(30)
    else:
        while not Path(os.environ['CHILD_READY']).exists(): time.sleep(.01)
        print('intentional second-target failure', flush=True)
        sys.exit(42)
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
            shell_environment=root/'shell-environment'
            shell_environment.write_text('''kill() {
    if [[ ${1:-} == -TERM ]]; then printf '%s\\n' "$*" >> "$CANCELLED_PIDS"; fi
    builtin kill "$@"
}
''')
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
            started=time.monotonic()
            result = subprocess.run(args, cwd=ROOT,
                env={**os.environ, "PATH": str(tools) + os.pathsep + os.environ["PATH"],
                     "TMPDIR": str(scratch), 'FAIL_RUN': 'yes' if failure else '',
                     'INCOMPLETE': 'yes' if incomplete else '',
                     'SEED_ONLY': 'yes' if seed_only else '',
                     'STALE_REAP': 'yes' if stale_reap else '',
                     **({'BASH_ENV':str(shell_environment),'CANCELLED_PIDS':str(root/'cancelled-pids')} if stale_reap else {}),
                     'SLOW_FIRST': 'yes' if slow_first else '', 'CHILD_READY': str(root/'child-ready')},
                capture_output=True, text=True, timeout=15)
            expected = 42 if failure or slow_first or stale_reap else 1 if incomplete or (seed_only and require_mutations) else 0
            self.assertEqual(result.returncode, expected, result.stderr)
            self.assertFalse(list(scratch.glob('htmlcut-fuzz-build.*')), 'build scratch must always be removed')
            if explicit:
                self.assertEqual((destination / "caller-marker").read_text(), "preserve")
                self.assertEqual((destination / "contract_bytes/corpus/caller-seed").read_text(), "preserve seed")
                remaining = list(destination.glob('run.*'))
            else:
                remaining = list(scratch.iterdir())
            if failure or slow_first or stale_reap or incomplete or (seed_only and require_mutations) or explicit:
                self.assertEqual(len(remaining), 1)
                self.assertIn(str(remaining[0]), result.stderr)
                self.assertTrue((remaining[0] / "contract_bytes/corpus").is_dir())
                self.assertTrue((remaining[0] / "build.resources.json").is_file())
                inventory=json.loads((remaining[0]/'seeds.json').read_text())
                for target,seeds in inventory.items():
                    corpus=remaining[0]/target/'corpus'
                    self.assertEqual({seed['name'] for seed in seeds}, {path.name for path in corpus.iterdir()})
                    self.assertTrue(all((corpus/seed['name']).stat().st_size == seed['bytes'] <= 4096
                                        for seed in seeds))
                if slow_first:
                    self.assertLess(time.monotonic()-started, 8, 'failed second target must cancel slow first target')
                    with self.assertRaises(ProcessLookupError):
                        os.kill(int((root/'child-ready').read_text()), 0)
                    self.assertIn('intentional second-target failure', (remaining[0]/'extract/run.log').read_text())
                    self.assertTrue((remaining[0]/'contract_bytes/run.resources.json').is_file())
                if stale_reap:
                    self.assertFalse((root/'cancelled-pids').exists(),
                                     'completed and reaped watchdogs must not receive cancellation signals')
            else:
                self.assertEqual(remaining, [])
            if runs != 10000 and expected == 0:
                self.assertIn('Short test only:', result.stdout)

    def test_default_success_removes_output(self):
        self.exercise()

    def test_default_failure_retains_output_and_prints_path(self):
        self.exercise(failure=True)

    @unittest.skipUnless(os.name == 'posix', 'POSIX shell/process boundary')
    def test_second_target_failure_cancels_slow_first_target(self):
        self.exercise(runs=3, slow_first=True)

    def test_reaped_workers_are_removed_from_cancellation_ownership(self):
        self.exercise(runs=3, stale_reap=True)

    def test_oversized_combined_seed_refuses_before_build(self):
        with tempfile.TemporaryDirectory() as directory:
            import shutil
            root=Path(directory)
            checkout=root/'checkout'
            (checkout/'scripts').mkdir(parents=True)
            for script in ['focused-smoke.sh','contributor-rust-tools.sh','run-resource-bounded.py']:
                shutil.copy2(ROOT/'scripts'/script, checkout/'scripts'/script)
            shutil.copy2(ROOT/'rust-toolchain.toml', checkout/'rust-toolchain.toml')
            case=checkout/'conformance/contract/seed'
            case.mkdir(parents=True)
            (case/'contract.json').write_bytes(b'{"select":"h1"}')
            # Each source separately fits4096; the framed combined input does not.
            (case/'page.html').write_bytes(b'x'*4096)
            (checkout/'fuzz/seeds/documents').mkdir(parents=True)
            tools=root/'bin';tools.mkdir()
            (tools/'python3').symlink_to(sys.executable)
            cargo=tools/'cargo'
            cargo.write_text('#!/usr/bin/env bash\ntouch "$BUILD_MARKER"\nexit 23\n');cargo.chmod(0o755)
            scratch=root/'scratch';scratch.mkdir()
            result=subprocess.run([str(checkout/'scripts/focused-smoke.sh'),'--runs','1'],
                env={**os.environ,'PATH':str(tools)+os.pathsep+os.environ['PATH'],
                     'TMPDIR':str(scratch),'BUILD_MARKER':str(root/'build-started')},
                capture_output=True,text=True,timeout=5)
            self.assertEqual(result.returncode,1,result.stderr)
            self.assertIn('exceeds the smoke input bound',result.stderr)
            self.assertFalse((root/'build-started').exists())
            self.assertFalse(list(scratch.glob('htmlcut-fuzz-build.*')))
            # Metadata-only refusal: no10,000-input fuzz campaign is launched.
            (case/'page.html').write_bytes(b'x')
            for index in range(10000):
                (checkout/'fuzz/seeds/documents'/str(index)).touch()
            result=subprocess.run([str(checkout/'scripts/focused-smoke.sh'),'--runs','1'],
                env={**os.environ,'PATH':str(tools)+os.pathsep+os.environ['PATH'],
                     'TMPDIR':str(scratch),'BUILD_MARKER':str(root/'build-started')},
                capture_output=True,text=True,timeout=5)
            self.assertEqual(result.returncode,1,result.stderr)
            self.assertIn('exceeds the bounded initial corpus of 10000 seeds',result.stderr)
            self.assertFalse((root/'build-started').exists())
            # The second refusal discovers the total before copying any seed.
            self.assertEqual(len(list(scratch.glob('htmlcut-fuzz.*/extract/corpus/*'))),0)

    def test_unconfirmed_cleanup_preserves_only_affected_roots(self):
        import shutil
        for mode in ['build-error','build-report-missing','worker-error']:
            with self.subTest(mode=mode), tempfile.TemporaryDirectory() as directory:
                root=Path(directory); checkout=root/'checkout'
                (checkout/'scripts').mkdir(parents=True)
                for script in ['focused-smoke.sh','contributor-rust-tools.sh']:
                    shutil.copy2(ROOT/'scripts'/script,checkout/'scripts'/script)
                shutil.copy2(ROOT/'rust-toolchain.toml',checkout/'rust-toolchain.toml')
                case=checkout/'conformance/contract/seed';case.mkdir(parents=True)
                (case/'contract.json').write_text('{"select":"h1"}')
                (case/'page.html').write_text('<h1>x</h1>')
                (checkout/'fuzz/seeds/documents').mkdir(parents=True)
                # Inject the watchdog's report contract; no writer survives this fixture.
                (checkout/'scripts/run-resource-bounded.py').write_text('''import json,os,sys
from pathlib import Path
args=sys.argv;root=Path(args[args.index('--root')+1]);label=args[args.index('--label')+1]
mode=os.environ['REPORT_MODE']
(root/(label+'.log')).write_text('#1 INITED cov: 1\\nDone 3 runs\\n')
errors=['injected unconfirmed child cleanup'] if (mode=='worker-error' and label=='run') or mode=='build-error' else []
if mode!='build-report-missing':
    (root/(label+'.resources.json')).write_text(json.dumps({'cleanup_errors':errors}))
if label=='build': (root/'owned-build-marker').write_text('preserve only if unconfirmed')
sys.exit(125 if mode.startswith('build-') else 0)
''')
                tools=root/'bin';tools.mkdir();(tools/'python3').symlink_to(sys.executable)
                rustc=tools/'rustc';rustc.write_text('#!/usr/bin/env bash\nprintf "host: test-host\\n"\n');rustc.chmod(0o755)
                scratch=root/'scratch';scratch.mkdir()
                result=subprocess.run([str(checkout/'scripts/focused-smoke.sh'),'--runs','3'],
                    env={**os.environ,'PATH':str(tools)+os.pathsep+os.environ['PATH'],
                         'TMPDIR':str(scratch),'REPORT_MODE':mode},capture_output=True,text=True,timeout=5)
                self.assertEqual(result.returncode,125,result.stderr)
                self.assertIn('process cleanup is unconfirmed',result.stderr)
                builds=list(scratch.glob('htmlcut-fuzz-build.*'))
                if mode.startswith('build-'):
                    self.assertEqual(len(builds),1)
                    self.assertTrue((builds[0]/'owned-build-marker').exists())
                    self.assertIn(str(builds[0]),result.stderr)
                else:
                    self.assertEqual(builds,[],'confirmed build must still be removed')
                evidence,=scratch.glob('htmlcut-fuzz.*')
                self.assertTrue(evidence.is_dir())
                self.assertIn(str(evidence),result.stderr)

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
