# SPDX-License-Identifier: MPL-2.0
"""Smoke scratch ownership, completion and failure evidence at the process boundary."""

import os
import json
import runpy
import shutil
from unittest import mock
import signal
import time
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[3]


def copy_runner(checkout):
    shutil.copytree(
        ROOT / "tools",
        checkout / "tools",
        ignore=shutil.ignore_patterns("__pycache__", "tests"),
    )
    shutil.copy2(ROOT / "rust-toolchain.toml", checkout / "rust-toolchain.toml")
    (checkout / "fuzz").mkdir()
    shutil.copy2(ROOT / "fuzz/Cargo.toml", checkout / "fuzz/Cargo.toml")


class FuzzCleanupTest(unittest.TestCase):
    def exercise(
        self,
        failure=False,
        explicit=False,
        runs=10000,
        incomplete=False,
        require_mutations=False,
        seed_only=False,
        slow_first=False,
        later_failure=False,
    ):
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
if os.environ.get('LATER_FAILURE') and sys.argv[0].endswith('extract'):
    time.sleep(2)
    sys.exit(42)
if os.environ.get('SLOW_FIRST'):
    Path(os.environ['CHILD_READY']).write_text(str(os.getpid()))
    print('intentional first-target failure',flush=True)
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
            destination = root / "caller-output"
            args = [
                sys.executable,
                "-m",
                "tools.htmlcut_maintenance",
                "qa",
                "fuzz",
                "--runs",
                str(runs),
            ]
            if require_mutations:
                args.append("--require-mutations")
            if explicit:
                destination.mkdir()
                (destination / "caller-marker").write_text("preserve")
                old_corpus = destination / "contract_bytes/corpus"
                old_corpus.mkdir(parents=True)
                (old_corpus / "caller-seed").write_text("preserve seed")
                args.append(str(destination))
            started = time.monotonic()
            result = subprocess.run(
                args,
                cwd=ROOT,
                env={
                    **os.environ,
                    "PATH": str(tools) + os.pathsep + os.environ["PATH"],
                    "TMPDIR": str(scratch),
                    "FAIL_RUN": "yes" if failure else "",
                    "INCOMPLETE": "yes" if incomplete else "",
                    "SEED_ONLY": "yes" if seed_only else "",
                    "LATER_FAILURE": "yes" if later_failure else "",
                    "SLOW_FIRST": "yes" if slow_first else "",
                    "CHILD_READY": str(root / "child-ready"),
                },
                capture_output=True,
                text=True,
                timeout=15,
            )
            expected = (
                42
                if failure or slow_first or later_failure
                else 1
                if incomplete or (seed_only and (require_mutations or runs == 10000))
                else 0
            )
            self.assertEqual(result.returncode, expected, result.stderr)
            self.assertFalse(
                list(scratch.glob("htmlcut-fuzz-build.*")),
                "build scratch must always be removed",
            )
            if explicit:
                self.assertEqual(
                    (destination / "caller-marker").read_text(), "preserve"
                )
                self.assertEqual(
                    (destination / "contract_bytes/corpus/caller-seed").read_text(),
                    "preserve seed",
                )
                remaining = list(destination.glob("run.*"))
            else:
                remaining = list(scratch.iterdir())
            if (
                failure
                or slow_first
                or later_failure
                or incomplete
                or (seed_only and (require_mutations or runs == 10000))
                or explicit
            ):
                self.assertEqual(len(remaining), 1)
                self.assertIn(str(remaining[0]), result.stderr)
                self.assertTrue((remaining[0] / "contract_bytes/corpus").is_dir())
                self.assertTrue((remaining[0] / "build.resources.json").is_file())
                inventory = json.loads((remaining[0] / "seeds.json").read_text())
                for target, seeds in inventory.items():
                    corpus = remaining[0] / target / "corpus"
                    self.assertEqual(
                        {seed["name"] for seed in seeds},
                        {path.name for path in corpus.iterdir()},
                    )
                    self.assertTrue(
                        all(
                            (corpus / seed["name"]).stat().st_size
                            == seed["bytes"]
                            <= 65536
                            for seed in seeds
                        )
                    )
                if slow_first:
                    self.assertLess(
                        time.monotonic() - started,
                        8,
                        "first target failure must finish promptly",
                    )
                    with self.assertRaises(ProcessLookupError):
                        os.kill(int((root / "child-ready").read_text()), 0)
                    self.assertFalse(
                        (remaining[0] / "extract/run.log").exists(),
                        "a failed first target must stop the second launch",
                    )
                    self.assertTrue(
                        (remaining[0] / "contract_bytes/run.resources.json").is_file()
                    )
            else:
                self.assertEqual(remaining, [])
            if runs != 10000 and expected == 0:
                self.assertIn("Short test only:", result.stdout)

    def test_default_success_removes_output(self):
        self.exercise()

    def test_default_failure_retains_output_and_prints_path(self):
        self.exercise(failure=True)

    @unittest.skipUnless(os.name == "posix", "POSIX shell/process boundary")
    def test_first_target_failure_stops_second_target_launch(self):
        self.exercise(runs=3, slow_first=True)

    def test_completed_first_target_does_not_obscure_later_failure(self):
        self.exercise(runs=3, later_failure=True)

    @unittest.skipUnless(os.name == "posix", "POSIX SIGCHLD ownership")
    def test_ignored_sigchld_refuses_before_creating_artifacts(self):
        code = """import runpy,signal,sys
namespace=runpy.run_path(sys.argv[1])
signal.signal(signal.SIGCHLD,signal.SIG_IGN)
sys.argv=[sys.argv[1],'--runs','1',sys.argv[2]]
sys.exit(namespace['main']())
"""
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "must-not-create"
            result = subprocess.run(
                [
                    sys.executable,
                    "-c",
                    code,
                    str(ROOT / "tools/htmlcut_maintenance/fuzz.py"),
                    str(output),
                ],
                capture_output=True,
                text=True,
                timeout=3,
            )
            self.assertEqual(result.returncode, 2, result.stderr)
            self.assertIn("SIGCHLD", result.stderr)
            self.assertFalse(output.exists())

    def test_harness_discovery_requires_one_complete_owned_executable_set(self):
        namespace = runpy.run_path(str(ROOT / "tools/htmlcut_maintenance/fuzz.py"))
        discover = namespace["harness_directory"]
        refusal = namespace["Refusal"]
        targets = namespace["fuzz_targets"]()
        binary = namespace["harness_path"]
        with tempfile.TemporaryDirectory() as directory:
            build = Path(directory) / "build"
            release = build / "target/first-host/release"
            release.mkdir(parents=True)
            for target in targets:
                binary(release, target).write_text("binary control")
                binary(release, target).chmod(0o755)
            self.assertEqual(discover(build, targets), release)
            second = build / "target/second-host/release"
            shutil.copytree(release, second)
            with self.assertRaisesRegex(refusal, "exactly one"):
                discover(build, targets)
            shutil.rmtree(second)
            binary(release, targets[0]).unlink()
            with self.assertRaisesRegex(refusal, "exactly one"):
                discover(build, targets)
            external = Path(directory) / "external-binary"
            external.write_text("outside build")
            binary(release, targets[0]).symlink_to(external)
            with self.assertRaisesRegex(refusal, "owned build"):
                discover(build, targets)
            linked_build = Path(directory) / "linked-build"
            linked_build.mkdir()
            cache = Path(directory) / "outside-target"
            external_release = cache / "host/release"
            external_release.mkdir(parents=True)
            for target in targets:
                binary(external_release, target).write_text("external binary control")
                binary(external_release, target).chmod(0o755)
            (linked_build / "target").symlink_to(cache, target_is_directory=True)
            with self.assertRaisesRegex(refusal, "owned build"):
                discover(linked_build, targets)

    def test_windows_harness_suffix_preserves_logical_target_names(self):
        from types import SimpleNamespace

        namespace = runpy.run_path(str(ROOT / "tools/htmlcut_maintenance/fuzz.py"))
        binary = namespace["harness_path"]
        discover = namespace["harness_directory"]
        native_os = binary.__globals__["os"]
        windows = SimpleNamespace(
            name="nt", access=native_os.access, X_OK=native_os.X_OK
        )
        with tempfile.TemporaryDirectory() as directory:
            build = Path(directory)
            release = build / "target/host/release"
            release.mkdir(parents=True)
            for name in ["contract_bytes.exe", "extract.exe"]:
                (release / name).write_bytes(b"filename control")
                (release / name).chmod(0o755)
            with mock.patch.dict(binary.__globals__, os=windows):
                self.assertEqual(binary(release, "extract"), release / "extract.exe")
                self.assertEqual(
                    discover(build, ["contract_bytes", "extract"]), release
                )
                (release / "extract.exe").rename(release / "extract")
                with self.assertRaisesRegex(namespace["Refusal"], "exactly one"):
                    discover(build, ["contract_bytes", "extract"])

    @unittest.skipUnless(hasattr(os, "mkfifo"), "POSIX named pipe seed refusal")
    def test_nonregular_seed_sources_refuse_before_open_or_build(self):
        for source_kind in ["contract", "document"]:
            with (
                self.subTest(source_kind=source_kind),
                tempfile.TemporaryDirectory() as directory,
            ):
                root = Path(directory)
                checkout = root / "checkout"
                copy_runner(checkout)
                case = checkout / "conformance/contract/seed"
                case.mkdir(parents=True)
                (case / "page.html").write_text("<h1>x</h1>")
                documents = checkout / "fuzz/seeds/documents"
                documents.mkdir(parents=True)
                if source_kind == "contract":
                    os.mkfifo(case / "contract.json")
                else:
                    (case / "contract.json").write_text('{"select":"h1"}')
                    os.mkfifo(documents / "must-not-open")
                tools = root / "bin"
                tools.mkdir()
                (tools / "python3").symlink_to(sys.executable)
                cargo = tools / "cargo"
                cargo.write_text(
                    '#!/usr/bin/env bash\ntouch "$BUILD_MARKER"\nexit 23\n'
                )
                cargo.chmod(0o755)
                scratch = root / "scratch"
                scratch.mkdir()
                started = time.monotonic()
                result = subprocess.run(
                    [
                        sys.executable,
                        "-m",
                        "tools.htmlcut_maintenance",
                        "qa",
                        "fuzz",
                        "--runs",
                        "1",
                    ],
                    cwd=checkout,
                    env={
                        **os.environ,
                        "PATH": str(tools) + os.pathsep + os.environ["PATH"],
                        "TMPDIR": str(scratch),
                        "BUILD_MARKER": str(root / "built"),
                    },
                    capture_output=True,
                    text=True,
                    timeout=1,
                )
                self.assertEqual(result.returncode, 1, result.stderr)
                self.assertLess(time.monotonic() - started, 1)
                self.assertIn("ordinary regular file", result.stderr)
                self.assertFalse((root / "built").exists())
                self.assertFalse(list(scratch.glob("htmlcut-fuzz-build.*")))

    @unittest.skipIf(
        os.name != "posix" or getattr(os, "geteuid", lambda: 0)() == 0,
        "requires native unprivileged file admission",
    )
    def test_evidence_copy_failure_preserves_original_status_and_owned_build(self):
        for build_exit, expected in [(0, 125), (23, 23)]:
            with (
                self.subTest(build_exit=build_exit),
                tempfile.TemporaryDirectory() as directory,
            ):
                root = Path(directory)
                checkout = root / "checkout"
                copy_runner(checkout)
                case = checkout / "conformance/contract/seed"
                case.mkdir(parents=True)
                (case / "contract.json").write_text('{"select":"h1"}')
                (case / "page.html").write_text("<h1>x</h1>")
                (checkout / "fuzz/seeds/documents").mkdir(parents=True)
                tools = root / "bin"
                tools.mkdir()
                (tools / "python3").symlink_to(sys.executable)
                cargo = tools / "cargo"
                cargo.write_text("""#!/usr/bin/env python3
import os,sys
from pathlib import Path
build=Path(os.environ['CARGO_TARGET_DIR']).parent
release=build/'target/test-host/release';release.mkdir(parents=True)
for target in ['contract_bytes','extract']:
    harness=release/target
    harness.write_text('#!/usr/bin/env python3\\nprint("#1 INITED cov: 1")\\nprint("Done 3 runs")\\n')
    harness.chmod(0o755)
(build/'build.log').chmod(0)
sys.exit(int(os.environ['BUILD_EXIT']))
""")
                cargo.chmod(0o755)
                scratch = root / "scratch"
                scratch.mkdir()
                result = subprocess.run(
                    [
                        sys.executable,
                        "-m",
                        "tools.htmlcut_maintenance",
                        "qa",
                        "fuzz",
                        "--runs",
                        "3",
                    ],
                    cwd=checkout,
                    env={
                        **os.environ,
                        "PATH": str(tools) + os.pathsep + os.environ["PATH"],
                        "TMPDIR": str(scratch),
                        "BUILD_EXIT": str(build_exit),
                    },
                    capture_output=True,
                    text=True,
                    timeout=8,
                )
                self.assertEqual(result.returncode, expected, result.stderr)
                self.assertIn("Cannot preserve fuzz build evidence", result.stderr)
                (build,) = scratch.glob("htmlcut-fuzz-build.*")
                self.assertIn(str(build), result.stderr)
                report = json.loads((build / "build.resources.json").read_text())
                self.assertEqual(report["returncode"], build_exit)
                self.assertEqual(
                    report["cleanup_errors"], [], "process cleanup was confirmed"
                )
                with self.assertRaises(PermissionError):
                    (build / "build.log").read_bytes()
                (build / "build.log").chmod(0o600)
                (output,) = scratch.glob("htmlcut-fuzz.*")
                self.assertTrue(
                    (output / "build.resources.json").exists(),
                    "one copy error must not prevent remaining evidence preservation",
                )

    def test_oversized_combined_seed_refuses_before_build(self):
        with tempfile.TemporaryDirectory() as directory:
            import shutil

            root = Path(directory)
            checkout = root / "checkout"
            copy_runner(checkout)
            case = checkout / "conformance/contract/seed"
            case.mkdir(parents=True)
            (case / "contract.json").write_bytes(b'{"select":"h1"}')
            # Each source separately fits4096; the framed combined input does not.
            (case / "page.html").write_bytes(b"x" * 65536)
            (checkout / "fuzz/seeds/documents").mkdir(parents=True)
            tools = root / "bin"
            tools.mkdir()
            (tools / "python3").symlink_to(sys.executable)
            cargo = tools / "cargo"
            cargo.write_text('#!/usr/bin/env bash\ntouch "$BUILD_MARKER"\nexit 23\n')
            cargo.chmod(0o755)
            scratch = root / "scratch"
            scratch.mkdir()
            result = subprocess.run(
                [
                    sys.executable,
                    "-m",
                    "tools.htmlcut_maintenance",
                    "qa",
                    "fuzz",
                    "--runs",
                    "1",
                ],
                cwd=checkout,
                env={
                    **os.environ,
                    "PATH": str(tools) + os.pathsep + os.environ["PATH"],
                    "TMPDIR": str(scratch),
                    "BUILD_MARKER": str(root / "build-started"),
                },
                capture_output=True,
                text=True,
                timeout=5,
            )
            self.assertEqual(result.returncode, 1, result.stderr)
            self.assertIn("exceeds the smoke input bound", result.stderr)
            self.assertFalse((root / "build-started").exists())
            self.assertFalse(list(scratch.glob("htmlcut-fuzz-build.*")))
            # Metadata-only refusal: no10,000-input fuzz campaign is launched.
            (case / "page.html").write_bytes(b"x")
            for index in range(10000):
                (checkout / "fuzz/seeds/documents" / str(index)).touch()
            result = subprocess.run(
                [
                    sys.executable,
                    "-m",
                    "tools.htmlcut_maintenance",
                    "qa",
                    "fuzz",
                    "--runs",
                    "1",
                ],
                cwd=checkout,
                env={
                    **os.environ,
                    "PATH": str(tools) + os.pathsep + os.environ["PATH"],
                    "TMPDIR": str(scratch),
                    "BUILD_MARKER": str(root / "build-started"),
                },
                capture_output=True,
                text=True,
                timeout=5,
            )
            self.assertEqual(result.returncode, 1, result.stderr)
            self.assertIn(
                "exceeds the bounded initial corpus of 10000 seeds", result.stderr
            )
            self.assertFalse((root / "build-started").exists())
            # The second refusal discovers the total before copying any seed.
            self.assertEqual(
                len(list(scratch.glob("htmlcut-fuzz.*/extract/corpus/*"))), 0
            )

    def test_unconfirmed_cleanup_preserves_only_affected_roots(self):
        import shutil

        for mode in [
            "build-error",
            "build-report-missing",
            "build-exit-missing",
            "build-exit-null",
            "worker-error",
        ]:
            with self.subTest(mode=mode), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                checkout = root / "checkout"
                copy_runner(checkout)
                case = checkout / "conformance/contract/seed"
                case.mkdir(parents=True)
                (case / "contract.json").write_text('{"select":"h1"}')
                (case / "page.html").write_text("<h1>x</h1>")
                (checkout / "fuzz/seeds/documents").mkdir(parents=True)
                # Inject the watchdog's report contract; no writer survives this fixture.
                (
                    checkout / "tools/htmlcut_maintenance/process.py"
                ).write_text("""import json,os
from pathlib import Path
def check_process_ownership(): pass
def status(result): return result['status']
def run(command, root, *, label, **kwargs):
    root=Path(root);mode=os.environ['REPORT_MODE']
    (root/(label+'.log')).write_text('#1 INITED cov: 1\\nDone 3 runs\\n')
    errors=['injected unconfirmed child cleanup'] if (mode=='worker-error' and label=='run') or mode=='build-error' else []
    report={'cleanup_errors':errors}
    if mode!='build-exit-missing': report['returncode']=None if mode=='build-exit-null' else 125 if mode.startswith('build-') else 0
    if mode!='build-report-missing': (root/(label+'.resources.json')).write_text(json.dumps(report))
    if label=='build':
        (root/'owned-build-marker').write_text('preserve only if unconfirmed')
        release=root/'target/test-host/release';release.mkdir(parents=True)
        for target in ['contract_bytes','extract']:
            harness=release/target;harness.write_text('#!/bin/sh\\nexit 0\\n');harness.chmod(0o755)
    return {'status':125 if mode.startswith('build-') else 0}
""")
                tools = root / "bin"
                tools.mkdir()
                (tools / "python3").symlink_to(sys.executable)
                scratch = root / "scratch"
                scratch.mkdir()
                result = subprocess.run(
                    [
                        sys.executable,
                        "-m",
                        "tools.htmlcut_maintenance",
                        "qa",
                        "fuzz",
                        "--runs",
                        "3",
                    ],
                    cwd=checkout,
                    env={
                        **os.environ,
                        "PATH": str(tools) + os.pathsep + os.environ["PATH"],
                        "TMPDIR": str(scratch),
                        "REPORT_MODE": mode,
                    },
                    capture_output=True,
                    text=True,
                    timeout=5,
                )
                self.assertEqual(result.returncode, 125, result.stderr)
                self.assertIn("process cleanup is unconfirmed", result.stderr)
                builds = list(scratch.glob("htmlcut-fuzz-build.*"))
                if mode.startswith("build-"):
                    self.assertEqual(len(builds), 1)
                    self.assertTrue((builds[0] / "owned-build-marker").exists())
                    self.assertIn(str(builds[0]), result.stderr)
                else:
                    self.assertEqual(
                        builds, [], "confirmed build must still be removed"
                    )
                (evidence,) = scratch.glob("htmlcut-fuzz.*")
                self.assertTrue(evidence.is_dir())
                self.assertIn(str(evidence), result.stderr)

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
            (tools / "python3").symlink_to(sys.executable)
            env = {**os.environ, "PATH": str(tools) + os.pathsep + os.environ["PATH"]}
            for runs in ["0", "-1", "10001", "99999999999999999999", "oops"]:
                result = subprocess.run(
                    [
                        sys.executable,
                        "-m",
                        "tools.htmlcut_maintenance",
                        "qa",
                        "fuzz",
                        "--runs",
                        runs,
                    ],
                    cwd=ROOT,
                    env=env,
                    capture_output=True,
                    text=True,
                )
                self.assertEqual(result.returncode, 2, result.stderr)
                self.assertIn("--runs must be an integer", result.stderr)

    def test_refuses_scratch_inside_checkout(self):
        with tempfile.TemporaryDirectory() as directory:
            tools = Path(directory)
            (tools / "python3").symlink_to(sys.executable)
            result = subprocess.run(
                [
                    sys.executable,
                    "-m",
                    "tools.htmlcut_maintenance",
                    "qa",
                    "fuzz",
                    "--runs",
                    "1",
                ],
                cwd=ROOT,
                env={
                    **os.environ,
                    "PATH": str(tools) + os.pathsep + os.environ["PATH"],
                    "TMPDIR": str(ROOT),
                },
                capture_output=True,
                text=True,
            )
            self.assertEqual(result.returncode, 2, result.stderr)
            self.assertIn("Fuzz scratch must be outside", result.stderr)

    @unittest.skipUnless(os.name == "posix", "POSIX shell interruption")
    def test_build_interruption_stops_child_and_retains_evidence(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            tools = root / "tools"
            tools.mkdir()
            (tools / "python3").symlink_to(sys.executable)
            ready = root / "ready"
            cargo = tools / "cargo"
            cargo.write_text(
                "#!/usr/bin/env python3\nfrom pathlib import Path\nimport os, time\nPath(os.environ['READY']).write_text(str(os.getpid()))\ntime.sleep(30)\n"
            )
            cargo.chmod(0o755)
            scratch = root / "scratch"
            scratch.mkdir()
            process = subprocess.Popen(
                [
                    sys.executable,
                    "-m",
                    "tools.htmlcut_maintenance",
                    "qa",
                    "fuzz",
                    "--runs",
                    "1",
                ],
                cwd=ROOT,
                env={
                    **os.environ,
                    "PATH": str(tools) + os.pathsep + os.environ["PATH"],
                    "TMPDIR": str(scratch),
                    "READY": str(ready),
                },
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True,
            )
            try:
                deadline = time.monotonic() + 5
                while not ready.exists() and time.monotonic() < deadline:
                    time.sleep(0.02)
                self.assertTrue(ready.exists(), "build child did not start")
                process.send_signal(signal.SIGTERM)
                _, stderr = process.communicate(timeout=5)
                self.assertEqual(process.returncode, 143, stderr)
                self.assertFalse(list(scratch.glob("htmlcut-fuzz-build.*")))
                (evidence,) = scratch.glob("htmlcut-fuzz.*")
                self.assertTrue((evidence / "build.resources.json").is_file())
                with self.assertRaises(ProcessLookupError):
                    os.kill(int(ready.read_text()), 0)
            finally:
                if process.poll() is None:
                    process.kill()
                    process.wait()


class FullSmokeTests(unittest.TestCase):
    def test_full_seed_replay_cannot_be_reported_as_mutation_smoke(self):
        FuzzCleanupTest().exercise(runs=10000, seed_only=True)
