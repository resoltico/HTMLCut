# SPDX-License-Identifier: MPL-2.0
"""Real child writers prove QA resource refusals and retained evidence."""

import json
import errno
import io
import shutil
import signal
import contextlib
import ctypes
import stat
import threading
from types import SimpleNamespace
import shlex
from pathlib import Path
import subprocess
import os
import runpy
import time
import sys
import tempfile
import unittest
from unittest import mock

ROOT = Path(__file__).resolve().parents[3]
RUNNER = ROOT / "tools/htmlcut_maintenance/process.py"


def retain_fixture_diagnostics(root, cleanup_errors):
    """Retain bounded failure facts even when the console log filled first."""
    facts = {"cleanup_errors": cleanup_errors}
    if sys.exc_info()[1] is not None:
        error = sys.exc_info()[1]
        facts["observation_error"] = (type(error).__name__ + ": " + str(error))[:4096]
    for name, limit in [("test.resources.json", 65536), ("handler-error.txt", 4096)]:
        try:
            with (root / name).open("rb") as source:
                value = source.read(limit + 1)
            facts[name] = value[:limit].decode("utf-8", errors="replace")
            if len(value) > limit:
                facts[name + "_truncated"] = True
        except OSError as error:
            facts[name] = str(error)
    try:
        with (root / "test.log").open("rb") as source:
            source.seek(0, 2)
            facts["log_bytes"] = source.tell()
            source.seek(max(0, source.tell() - 4096))
            facts["log_tail"] = source.read(4096).decode("utf-8", errors="replace")
    except OSError as error:
        facts["log_error"] = str(error)
    try:
        report = json.loads(facts["test.resources.json"])
        if not isinstance(report, dict):
            raise ValueError("resource report is not an object")
        child_status, refusal = report.get("returncode"), report.get("refusal")
    except (ValueError, TypeError):
        child_status, refusal = "unavailable", "unavailable"
    try:
        (root / "fixture-diagnostic.json").write_text(
            json.dumps(facts, indent=2) + "\n"
        )
    except OSError as error:
        print(f"Cannot retain fixture diagnostics at {root}: {error}", file=sys.stderr)
    preview = lambda value: str(value)[:512]
    summary = (
        f"Resource fixture evidence retained at {root}: "
        f"child={preview(child_status)}; refusal={preview(refusal)}; "
        f"cleanup={preview(cleanup_errors)}; handler={preview(facts['handler-error.txt'])}"
    )
    print(
        summary.encode("utf-8", errors="replace")[:4095].decode(
            "utf-8", errors="ignore"
        ),
        file=sys.stderr,
    )


class ResourceBudgetTests(unittest.TestCase):
    def test_failure_diagnostics_keep_status_and_bound_late_error_evidence(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            report = {"returncode": 7, "refusal": "log-budget-exceeded"}
            (root / "test.resources.json").write_text(json.dumps(report))
            (root / "handler-error.txt").write_text("handler failure: " + "e" * 6000)
            (root / "test.log").write_bytes(b"x" * (2 * 1024 * 1024))
            with mock.patch("sys.stderr", new_callable=io.StringIO) as console:
                retain_fixture_diagnostics(root, [])
            facts = json.loads((root / "fixture-diagnostic.json").read_text())
            self.assertEqual(json.loads(facts["test.resources.json"]), report)
            self.assertTrue(facts["handler-error.txt_truncated"])
            self.assertEqual(len(facts["handler-error.txt"]), 4096)
            self.assertEqual(facts["log_tail"], "x" * 4096)
            self.assertIn("child=7; refusal=log-budget-exceeded", console.getvalue())
            self.assertIn("handler failure:", console.getvalue())
            self.assertLess(len(console.getvalue()), 1500)
            self.assertLess((root / "fixture-diagnostic.json").stat().st_size, 80000)
            (root / "test.resources.json").write_text(
                json.dumps({"returncode": "u" * 15000, "refusal": "r" * 15000})
            )
            with mock.patch("sys.stderr", new_callable=io.StringIO) as console:
                retain_fixture_diagnostics(root, ["c" * 12000])
            self.assertLessEqual(len(console.getvalue().encode("utf-8")), 4096)
            for value in ["u", "r", "c"]:
                self.assertNotIn(value * 513, console.getvalue())

    def test_windows_access_denied_accepts_only_a_now_missing_namespace_entry(self):
        functions = runpy.run_path(str(RUNNER))
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            sentinel = root / "preserved"
            sentinel.write_bytes(b"counted")
            removed = root / "removed"
            error = PermissionError(13, "removal window", str(removed))
            error.winerror = 5
            original = os.lstat

            def failed_entry(*, follow_symlinks):
                self.assertFalse(follow_symlinks)
                raise error

            def known_entry(*, follow_symlinks):
                self.assertFalse(follow_symlinks)
                return original(sentinel)

            @contextlib.contextmanager
            def entries(_):
                yield iter(
                    [
                        SimpleNamespace(path=str(sentinel), stat=known_entry),
                        SimpleNamespace(path=str(removed), stat=failed_entry),
                    ]
                )

            with mock.patch.object(os, "scandir", side_effect=entries):
                expected = sum(
                    max(info.st_size, getattr(info, "st_blocks", 0) * 512)
                    for info in [original(root), original(sentinel)]
                )
                self.assertEqual(functions["disk_bytes"](root), expected)

            removed.mkdir()
            with (
                mock.patch.object(os, "scandir", side_effect=entries),
                mock.patch.object(os, "lstat", side_effect=original) as checked,
            ):
                with self.assertRaises(PermissionError) as raised:
                    functions["disk_bytes"](root)
                self.assertIs(raised.exception, error)
                self.assertEqual(
                    [call.args[0] for call in checked.call_args_list],
                    [root, str(removed)],
                )

            if os.name != "nt":
                removed.rmdir()
                removed.symlink_to(root / "missing-target", target_is_directory=True)
                with mock.patch.object(os, "scandir", side_effect=entries):
                    with self.assertRaises(PermissionError) as raised:
                        functions["disk_bytes"](root)
                    self.assertIs(raised.exception, error)

    def test_disk_accounting_counts_links_and_directories_without_following_them(self):
        disk_bytes = runpy.run_path(str(RUNNER))["disk_bytes"]
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            owned, outside = root / "owned", root / "outside"
            owned.mkdir()
            outside.mkdir()
            (outside / "large").write_bytes(b"x" * 8192)
            nested = owned / "nested"
            nested.mkdir()
            leaf = nested / "leaf"
            leaf.write_bytes(b"owned")
            expected_paths = [owned, nested, leaf]
            if os.name != "nt":
                link = owned / "external-link"
                link.symlink_to(outside, target_is_directory=True)
                expected_paths.append(link)
                self.assertEqual(
                    disk_bytes(link),
                    max(link.lstat().st_size, link.lstat().st_blocks * 512),
                )
            expected = sum(
                max(info.st_size, getattr(info, "st_blocks", 0) * 512)
                for info in (path.lstat() for path in expected_paths)
            )
            self.assertEqual(disk_bytes(owned), expected)

    def test_reparse_directory_is_counted_without_descending(self):
        disk_bytes = runpy.run_path(str(RUNNER))["disk_bytes"]
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            info = root.lstat()
            entry = mock.Mock(path=str(root / "junction"))
            entry.stat.return_value = SimpleNamespace(
                st_mode=stat.S_IFDIR | 0o755,
                st_size=100,
                st_blocks=1,
                st_file_attributes=0x400,
            )

            @contextlib.contextmanager
            def entries(path):
                self.assertEqual(path, str(root))
                yield iter([entry])

            with mock.patch.object(os, "scandir", side_effect=entries) as scanned:
                self.assertEqual(
                    disk_bytes(root),
                    max(info.st_size, getattr(info, "st_blocks", 0) * 512) + 512,
                )
            scanned.assert_called_once_with(str(root))
            entry.stat.assert_called_once_with(follow_symlinks=False)

    def test_windows_namespace_recheck_keeps_original_denial_on_every_other_result(
        self,
    ):
        vanished = runpy.run_path(str(RUNNER))["vanished_entry"]
        for path, code in [
            (None, 5),
            ("", 5),
            ("not-present", None),
            ("not-present", 32),
        ]:
            error = PermissionError(13, "original denial", path)
            error.winerror = code
            with (
                self.subTest(path=path, code=code),
                mock.patch.object(os, "lstat") as check,
            ):
                self.assertFalse(vanished(error))
                check.assert_not_called()
        error = PermissionError(13, "original denial", "fixture")
        error.winerror = 5
        for result in [
            PermissionError(13, "still denied"),
            OSError(5, "metadata failure"),
        ]:
            with (
                self.subTest(result=result),
                mock.patch.object(os, "lstat", side_effect=result) as check,
            ):
                self.assertFalse(vanished(error))
                check.assert_called_once_with("fixture")

    def test_windows_listing_denial_remains_an_accounting_refusal(self):
        fault = """
def inaccessible(path):
    error = PermissionError(13, 'listing denied', str(path))
    error.winerror = 5
    raise error
namespace['main'].__globals__['os'].scandir = inaccessible
"""
        result, report, _ = self.run_child("import time; time.sleep(5)", fault=fault)
        self.assertEqual(result.returncode, 125, result.stderr)
        self.assertIn("disk-accounting-failed:", report["refusal"])
        self.assertIn("listing denied", report["refusal"])
        self.assertEqual(
            [error["phase"] for error in report["disk_accounting_errors"]],
            ["monitor", "final"],
        )
        self.assertNotEqual(report["returncode"], 0)

    def test_command_status_preserves_failures_outside_eight_bit_transport(self):
        status = runpy.run_path(str(RUNNER))["command_exit_status"]
        for raw, expected in [
            (0, 0),
            (2, 2),
            (3, 3),
            (17, 17),
            (125, 125),
            (255, 255),
            (256, 1),
            (512, 1),
            (0xC0000005, 1),
            (-9, 137),
        ]:
            with self.subTest(raw=raw):
                self.assertEqual(status(raw), expected)

    @unittest.skipUnless(os.name == "nt", "native Windows DWORD process exit")
    def test_native_windows_wide_child_status_remains_failure(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            result = subprocess.run(
                [
                    sys.executable,
                    str(RUNNER),
                    "--root",
                    directory,
                    "--disk-limit-mib",
                    "1",
                    "--timeout",
                    "2",
                    "--label",
                    "wide",
                    "--",
                    sys.executable,
                    "-c",
                    "import ctypes; ctypes.windll.kernel32.ExitProcess(256)",
                ],
                capture_output=True,
                timeout=10,
            )
            report = json.loads((root / "wide.resources.json").read_text())
            self.assertEqual(result.returncode, 1)
            self.assertEqual(report["returncode"], 256)
            self.assertIsNone(report["refusal"])
            self.assertEqual(report["cleanup_errors"], [])

    def run_child(self, code, timeout=2, disk_limit=1, poll="0.02", fault=None):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            marker = root / "owner-evidence"
            marker.write_text("preserve")
            invocation = [sys.executable, str(RUNNER)]
            if fault:
                wrapper = (
                    "import runpy,sys\nnamespace=runpy.run_path(sys.argv[1])\n"
                    "sys.argv=sys.argv[1:]\n"
                    + fault
                    + "\nsys.exit(namespace['main']())\n"
                )
                invocation = [sys.executable, "-c", wrapper, str(RUNNER)]
            result = subprocess.run(
                [
                    *invocation,
                    "--root",
                    directory,
                    "--disk-limit-mib",
                    str(disk_limit),
                    "--log-limit-mib",
                    "1",
                    "--timeout",
                    str(timeout),
                    "--poll-seconds",
                    str(poll),
                    "--label",
                    "test",
                    "--",
                    sys.executable,
                    "-c",
                    code,
                    directory,
                ],
                capture_output=True,
                text=True,
                timeout=8,
            )
            report = json.loads((root / "test.resources.json").read_text())
            log = (root / "test.log").read_bytes()
            self.assertEqual(marker.read_text(), "preserve")
            return result, report, log

    def test_nonfinite_time_or_sampling_cannot_disable_termination(self):
        for flag in ["--timeout", "--poll-seconds"]:
            for value in ["nan", "inf"]:
                with tempfile.TemporaryDirectory() as directory:
                    command = [
                        sys.executable,
                        str(RUNNER),
                        "--root",
                        directory,
                        "--disk-limit-mib",
                        "1",
                        "--log-limit-mib",
                        "1",
                        "--timeout",
                        "1",
                        "--poll-seconds",
                        ".02",
                        "--label",
                        "test",
                        flag,
                        value,
                        "--",
                        sys.executable,
                        "-c",
                        "raise RuntimeError('must not run')",
                    ]
                    result = subprocess.run(command, capture_output=True, timeout=3)
                    self.assertEqual(result.returncode, 2)
                    self.assertFalse(list(Path(directory).iterdir()))

    def test_huge_finite_poll_cannot_overflow_or_leave_child_running(self):
        result, report, _ = self.run_child(
            "import time; time.sleep(5)", timeout=0.1, poll="1e308"
        )
        self.assertEqual(result.returncode, 125, result.stderr)
        self.assertEqual(report["refusal"], "wall-time-exceeded")
        self.assertNotEqual(
            report["returncode"], 0, "sleeping child was not terminated"
        )
        self.assertLess(report["elapsed_seconds"], 2)

    def test_huge_finite_intervals_allow_successful_child(self):
        result, report, log = self.run_child(
            "import sys; sys.stdout.buffer.write(b'short child\\n')",
            timeout=1e308,
            poll="1e308",
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIsNone(report["refusal"])
        self.assertEqual(log, b"short child\n")
        self.assertLess(report["elapsed_seconds"], 3)

    def test_exit_observation_is_separate_from_disk_sampling(self):
        from tools.htmlcut_maintenance import process

        class OwnerEvent(threading.Event):
            def __init__(self, release):
                super().__init__()
                self.release = release
                self.waits = []

            def wait(self, timeout=None):
                self.waits.append(timeout)
                self.release.write_text("owner observed the child")
                return super().wait(min(timeout, 0.005))

        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            release = root / "release"
            owner = OwnerEvent(release)
            code = (
                "import pathlib,time,sys\n"
                "release=pathlib.Path(sys.argv[1]); deadline=time.monotonic()+2\n"
                "while not release.exists():\n"
                " if time.monotonic()>=deadline: raise RuntimeError('no owner observation')\n"
                " time.sleep(.005)\n"
            )
            result = process.run(
                [sys.executable, "-c", code, str(release)],
                root,
                disk_limit_mib=1,
                timeout=5,
                label="observation",
                poll_seconds=1,
                cancelled=owner,
            )
            self.assertEqual(process.status(result), 0)
            self.assertTrue(owner.waits)
            self.assertTrue(all(0 <= interval <= 0.05 for interval in owner.waits))
            self.assertGreaterEqual(result["disk_samples"], 2)
            self.assertGreaterEqual(result["disk_scan_seconds"], 0)

    @unittest.skipIf(os.name == "nt", "POSIX nonreaping completed-child control")
    def test_an_already_exited_child_still_gets_initial_and_final_samples(self):
        from tools.htmlcut_maintenance import process

        command = [sys.executable, "-c", "pass"]
        original = subprocess.Popen

        def exited(*args, **kwargs):
            child = original(*args, **kwargs)
            if args[0] == command:
                os.waitid(os.P_PID, child.pid, os.WEXITED | os.WNOWAIT)
            return child

        for denied in (False, True):
            event = threading.Event()
            with (
                self.subTest(denied=denied),
                tempfile.TemporaryDirectory() as directory,
                mock.patch.object(subprocess, "Popen", side_effect=exited),
                mock.patch.object(
                    process, "disk_bytes", wraps=process.disk_bytes,
                    side_effect=PermissionError(errno.EACCES, "exited denial") if denied else None,
                ) as sampled,
                mock.patch.object(event, "wait", wraps=event.wait) as waited,
                contextlib.redirect_stderr(io.StringIO()),
            ):
                result = process.run(
                    command, Path(directory), disk_limit_mib=1, timeout=5,
                    label="exited", cancelled=event,
                )
                self.assertEqual(process.status(result), 125 if denied else 0)
                self.assertEqual(sampled.call_count, 4 if denied else 2)
                self.assertEqual(result["disk_samples"], sampled.call_count)
                self.assertEqual(result["disk_scan_retries"], 2 if denied else 0)
                waited.assert_not_called()

    def test_poll_interval_respects_remaining_deadline(self):
        result, report, _ = self.run_child(
            "import time; time.sleep(5)", timeout=0.1, poll="10"
        )
        self.assertEqual(result.returncode, 125, result.stderr)
        self.assertEqual(report["refusal"], "wall-time-exceeded")
        self.assertLess(report["elapsed_seconds"], 2)

    def test_cancellation_during_cleanup_cannot_become_success(self):
        from tools.htmlcut_maintenance import process

        event = threading.Event()
        original = process.stop_process_tree

        def cleanup(child):
            original(child)
            event.set()

        with (
            tempfile.TemporaryDirectory() as directory,
            mock.patch.object(process, "stop_process_tree", side_effect=cleanup),
        ):
            result = process.run(
                [sys.executable, "-c", "pass"],
                Path(directory),
                disk_limit_mib=1,
                timeout=5,
                label="cleanup-cancel",
                cancelled=event,
            )
            self.assertEqual(result["returncode"], 0)
            self.assertEqual(result["cleanup_errors"], [])
            self.assertEqual(result["refusal"], "interrupted")
            self.assertNotEqual(process.status(result), 0)

    def test_monitor_exception_still_terminates_real_child_and_reports(self):
        fault = """
def fail_sample(root):
    raise RuntimeError('sample probe')
namespace['main'].__globals__['disk_bytes'] = fail_sample
"""
        result, report, _ = self.run_child("import time; time.sleep(5)", fault=fault)
        self.assertEqual(result.returncode, 125, result.stderr)
        self.assertIn("monitor-failed: RuntimeError: sample probe", report["refusal"])
        self.assertNotEqual(
            report["returncode"], 0, "sleeping child was not terminated"
        )
        self.assertLess(report["elapsed_seconds"], 2)

    def test_walk_permission_error_refuses_instead_of_skipping_subtree(self):
        fault = """
def inaccessible(path):
    raise PermissionError('enumeration probe')
namespace['main'].__globals__['os'].scandir = inaccessible
"""
        result, report, _ = self.run_child("import time; time.sleep(5)", fault=fault)
        self.assertEqual(result.returncode, 125, result.stderr)
        self.assertIn("disk-accounting-failed: enumeration probe", report["refusal"])
        self.assertEqual(
            [error["phase"] for error in report["disk_accounting_errors"]],
            ["monitor", "final"],
        )
        self.assertNotEqual(
            report["returncode"], 0, "sleeping child was not terminated"
        )

    @unittest.skipIf(
        os.name == "nt" or getattr(os, "geteuid", lambda: 1)() == 0,
        "requires ordinary POSIX directory access permissions",
    )
    def test_actual_permission_recovery_requires_a_complete_fresh_count(self):
        fault = """
functions = namespace['main'].__globals__
original = functions['disk_bytes']
def observe_denial(root):
    try:
        return original(root)
    except PermissionError:
        (root / 'denial-observed').write_bytes(b'actual native denial')
        raise
functions['disk_bytes'] = observe_denial
"""
        for size, over_budget in [(256 * 1024, False), (2 * 1024 * 1024, True)]:
            code = (
                "import pathlib,sys,time\nroot=pathlib.Path(sys.argv[1])\n"
                "denied=root/'denied'; denied.mkdir()\n"
                "(denied/'hidden').write_bytes(b'x'*(128*1024)); denied.chmod(0)\n"
                "try:\n"
                " deadline=time.monotonic()+2\n"
                " while not (root/'denial-observed').exists():\n"
                "  if time.monotonic()>=deadline: raise RuntimeError('no real denial observed')\n"
                "  time.sleep(.005)\n"
                " time.sleep(.12)\n"
                f" (root/'other-large').write_bytes(b'x'*{size})\n"
                " denied.chmod(0o700)\n"
                f" time.sleep({5 if over_budget else .15})\n"
                "finally:\n denied.chmod(0o700)\n"
            )
            with self.subTest(over_budget=over_budget):
                result, report, _ = self.run_child(code, poll="1", fault=fault)
                self.assertEqual(result.returncode, 125 if over_budget else 0, result.stderr)
                self.assertEqual(report["refusal"], "disk-budget-exceeded" if over_budget else None)
                self.assertEqual(report["disk_accounting_errors"], [])
                self.assertGreaterEqual(report["disk_scan_access_errors"], 1)
                self.assertGreaterEqual(report["disk_scan_retries"], 1)
                self.assertGreaterEqual(report["disk_completed_samples"], 2)
                self.assertGreaterEqual(report["peak_sampled_disk_bytes"], size + 128 * 1024)
                self.assertIn("denied", report["disk_scan_access_error_witnesses"][0]["error"])
                self.assertEqual(report["disk_samples"], report["disk_completed_samples"] + report["disk_scan_access_errors"])

    def test_access_recovery_has_bounded_attempts_and_witnesses(self):
        from tools.htmlcut_maintenance import process

        class ImmediateEvent(threading.Event):
            def wait(self, timeout=None):
                self.waits.append(timeout)
                return self.is_set()

        owner = ImmediateEvent()
        owner.waits = []
        with (
            tempfile.TemporaryDirectory() as directory,
            mock.patch.object(process, "disk_bytes", side_effect=PermissionError(errno.EACCES, "x" * 900)),
            contextlib.redirect_stderr(io.StringIO()),
        ):
            result = process.run([sys.executable, "-c", "import time;time.sleep(5)"],
                                 Path(directory), disk_limit_mib=1, timeout=5,
                                 label="denied", cancelled=owner)
        self.assertEqual(process.status(result), 125)
        self.assertEqual(result["disk_samples"], 23)  # 21 live attempts + two final confirmations.
        self.assertEqual(result["disk_completed_samples"], 0)
        self.assertEqual(result["disk_scan_retries"], 21)
        self.assertEqual(result["disk_scan_access_errors"], 23)
        self.assertEqual(len(result["disk_scan_access_error_witnesses"]), 16)
        self.assertEqual(result["disk_scan_access_error_witnesses_omitted"], 7)
        self.assertTrue(all(item["error_truncated"] and len(item["error"]) == 512
                            for item in result["disk_scan_access_error_witnesses"]))
        self.assertTrue(owner.waits)
        self.assertTrue(all(0 <= wait <= .05 for wait in owner.waits))

    def test_cancellation_during_access_recovery_keeps_its_stop_reason(self):
        from tools.htmlcut_maintenance import process

        class CancellingEvent(threading.Event):
            def wait(self, timeout=None):
                self.set()
                return True

        with (
            tempfile.TemporaryDirectory() as directory,
            mock.patch.object(process, "disk_bytes", side_effect=PermissionError(errno.EACCES, "cancel denial")),
            contextlib.redirect_stderr(io.StringIO()),
        ):
            result = process.run([sys.executable, "-c", "import time;time.sleep(5)"],
                                 Path(directory), disk_limit_mib=1, timeout=5,
                                 label="cancel", cancelled=CancellingEvent())
        self.assertEqual(result["refusal"], "interrupted")
        self.assertEqual(result["disk_scan_retries"], 1)
        self.assertEqual(result["disk_samples"], 3)
        self.assertLess(result["elapsed_seconds"], 2)
        self.assertEqual(result["cleanup_errors"], [])

    def test_other_accounting_errors_do_not_retry(self):
        result, report, _ = self.run_child("import time;time.sleep(5)", fault="""
def failed(root):
    raise OSError(5, 'non-permission accounting failure')
namespace['main'].__globals__['disk_bytes'] = failed
""")
        self.assertEqual(result.returncode, 125, result.stderr)
        self.assertEqual(report["disk_samples"], 2)
        self.assertEqual(report["disk_scan_retries"], 0)
        self.assertEqual(report["disk_scan_access_errors"], 0)

    def test_access_recovery_preserves_deadline_and_log_refusals(self):
        fault = """
def denied(root):
    raise PermissionError(13, 'recovery stop probe')
namespace['main'].__globals__['disk_bytes'] = denied
"""
        for code, timeout, refusal in [
            ("import time;time.sleep(5)", .1, "wall-time-exceeded"),
            ("import sys,time;sys.stdout.buffer.write(b'x'*(2*1024*1024));sys.stdout.flush();time.sleep(5)",
             2, "log-budget-exceeded"),
        ]:
            with self.subTest(refusal=refusal):
                result, report, _ = self.run_child(code, timeout=timeout, poll="1", fault=fault)
                self.assertEqual(result.returncode, 125, result.stderr)
                self.assertEqual(report["refusal"], refusal)
                self.assertGreater(report["disk_scan_access_errors"], 0)
                self.assertLess(report["disk_scan_retries"], 20)

    def test_fast_sampling_does_not_allow_a_longer_access_recovery_window(self):
        fault = """
functions = namespace['main'].__globals__
original = functions['disk_bytes']
until = None
def denied_briefly(root):
    global until
    if until is None:
        until = functions['time'].monotonic()+.12
    if functions['time'].monotonic()<until:
        raise PermissionError(13, 'longer than the configured sample interval')
    return original(root)
functions['disk_bytes'] = denied_briefly
"""
        result, report, _ = self.run_child("import time;time.sleep(.3)", poll=".02", fault=fault)
        self.assertEqual(result.returncode, 125, result.stderr)
        self.assertTrue(report["refusal"].startswith("disk-accounting-failed:"), report)
        self.assertEqual(report["disk_accounting_errors"][0]["phase"], "monitor")
        self.assertGreaterEqual(report["disk_scan_access_errors"], 1)

    @unittest.skipIf(
        os.name == "nt" or getattr(os, "geteuid", lambda: 1)() == 0,
        "requires ordinary POSIX directory access permissions",
    )
    def test_actual_inaccessible_directory_causes_disk_refusal(self):
        result, report, _ = self.run_child(
            "import pathlib,sys,time; denied=pathlib.Path(sys.argv[1])/'denied'; "
            "denied.mkdir(); denied.chmod(0); time.sleep(5)"
        )
        self.assertEqual(result.returncode, 125, result.stderr)
        self.assertTrue(report["refusal"].startswith("disk-accounting-failed:"), report)
        self.assertEqual(
            [error["phase"] for error in report["disk_accounting_errors"]],
            ["monitor", "final"],
        )
        self.assertNotEqual(
            report["returncode"], 0, "sleeping child was not terminated"
        )

    def final_sample_failure(self, code, timeout=2, *, recover_access=False):
        fault = """
functions = namespace['main'].__globals__
original_stop = functions['stop_process_tree']
original_sample = functions['disk_bytes']
cleaned = False
access_failed = False
def stop_child(process):
    global cleaned
    original_stop(process)
    cleaned = True
def sample(root):
    global access_failed
    if cleaned:
        if RECOVER_ACCESS:
            if not access_failed:
                access_failed = True
                raise PermissionError(13, 'final access window')
        else:
            raise PermissionError('final sample probe')
    return original_sample(root)
functions['stop_process_tree'] = stop_child
functions['disk_bytes'] = sample
"""
        return self.run_child(code, timeout=timeout, poll="1", fault=fault.replace("RECOVER_ACCESS", repr(recover_access)))

    def test_final_access_window_requires_a_fresh_complete_confirmation(self):
        result, report, log = self.final_sample_failure(
            "import sys; sys.stdout.buffer.write(b'completed child\\n')", recover_access=True
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(report["returncode"], 0)
        self.assertIsNone(report["refusal"])
        self.assertEqual(report["disk_accounting_errors"], [])
        self.assertEqual(report["disk_scan_retries"], 1)
        self.assertEqual(report["disk_scan_access_errors"], 1)
        self.assertEqual(report["disk_scan_access_error_witnesses"][0]["phase"], "final")
        self.assertEqual(report["disk_samples"], report["disk_completed_samples"] + 1)
        self.assertEqual(log, b"completed child\n")

    def test_final_access_recovery_does_not_query_an_unconfirmed_child(self):
        fault = """
functions = namespace['main'].__globals__
original_start = functions['subprocess'].Popen
original_stop = functions['stop_process_tree']
original_exited = functions['child_exited']
original_sample = functions['disk_bytes']
cleaned = False
failed = False
retained_wait = None
def start(*args, **kwargs):
    global retained_wait
    child = original_start(*args, **kwargs)
    if args[0][:1] != [sys.executable]:
        return child
    retained_wait = child.wait
    def unconfirmed(*args, **kwargs):
        raise functions['subprocess'].TimeoutExpired('synthetic unconfirmed cleanup', 2)
    child.wait = unconfirmed
    return child
def stop(child):
    global cleaned
    original_stop(child)
    cleaned = True
def exited(child):
    if cleaned:
        raise AssertionError('final accounting queried an unconfirmed child identity')
    return original_exited(child)
def sample(root):
    global failed
    if cleaned:
        if not failed:
            failed = True
            raise PermissionError(13, 'final access window')
        # Fixture teardown confirms the stopped, still-owned child after the
        # owner has already retained its deliberately unconfirmed wait result.
        retained_wait(timeout=2)
    return original_sample(root)
functions['subprocess'].Popen = start
functions['stop_process_tree'] = stop
functions['child_exited'] = exited
functions['disk_bytes'] = sample
"""
        result, report, log = self.run_child(
            "import sys; sys.stdout.buffer.write(b'completed\\n')", poll="1", fault=fault
        )
        self.assertEqual(result.returncode, 125, result.stderr)
        self.assertIsNone(report["returncode"])
        self.assertEqual(report["refusal"], "process-cleanup-failed")
        self.assertEqual(report["cleanup_errors"], ["leader did not exit within cleanup grace"])
        self.assertEqual(report["disk_accounting_errors"], [])
        self.assertEqual(report["disk_scan_retries"], 1)
        self.assertEqual(log, b"completed\n")

    def test_final_sample_failure_retains_successful_child_log_and_report(self):
        result, report, log = self.final_sample_failure(
            "import sys; sys.stdout.buffer.write(b'retained child evidence\\n')"
        )
        self.assertEqual(result.returncode, 125, result.stderr)
        self.assertEqual(report["returncode"], 0)
        self.assertIn("disk-accounting-failed: final sample probe", report["refusal"])
        self.assertEqual(log, b"retained child evidence\n")
        self.assertEqual(
            report["disk_accounting_errors"],
            [{"phase": "final", "error": "final sample probe"}],
        )

    def test_final_sample_failure_preserves_original_timeout_refusal(self):
        result, report, _ = self.final_sample_failure(
            "import time; time.sleep(5)", timeout=0.1
        )
        self.assertEqual(result.returncode, 125, result.stderr)
        self.assertEqual(report["refusal"], "wall-time-exceeded")
        self.assertEqual(
            report["disk_accounting_errors"],
            [{"phase": "final", "error": "final sample probe"}],
        )

    def test_success_records_actual_exit_and_output(self):
        result, report, log = self.run_child(
            "import sys; sys.stdout.buffer.write(b'native-child\\n')"
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIsNone(report["refusal"])
        self.assertEqual(log, b"native-child\n")
        self.assertGreater(report["peak_sampled_disk_bytes"], 0)

    def test_child_crlf_bytes_are_preserved_without_normalization(self):
        result, report, log = self.run_child(
            "import sys; sys.stdout.buffer.write(b'line\\r\\n')"
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIsNone(report["refusal"])
        self.assertEqual(log, b"line\r\n")

    def test_disk_writer_is_refused(self):
        result, report, _ = self.run_child(
            "import pathlib,sys,time; (pathlib.Path(sys.argv[1])/'large').write_bytes(b'x'*(2*1024*1024)); time.sleep(5)"
        )
        self.assertEqual(result.returncode, 125, result.stderr)
        self.assertEqual(report["refusal"], "disk-budget-exceeded")
        self.assertGreater(
            report["peak_sampled_disk_bytes"], report["disk_limit_bytes"]
        )

    def test_time_limit_terminates_child(self):
        result, report, _ = self.run_child("import time; time.sleep(5)", timeout=0.1)
        self.assertEqual(result.returncode, 125, result.stderr)
        self.assertEqual(report["refusal"], "wall-time-exceeded")
        self.assertLess(report["elapsed_seconds"], 2)

    def test_log_limit_retains_only_bounded_output(self):
        result, report, log = self.run_child(
            "import sys; sys.stdout.buffer.write(b'x'*(2*1024*1024))", disk_limit=4
        )
        self.assertEqual(result.returncode, 125, result.stderr)
        self.assertEqual(report["refusal"], "log-budget-exceeded")
        self.assertEqual(len(log), 1024 * 1024)

    def test_failed_command_does_not_become_success(self):
        result, report, _ = self.run_child("import sys; sys.exit(7)")
        self.assertEqual(result.returncode, 7, result.stderr)
        self.assertEqual(report["returncode"], 7)
        self.assertIsNone(report["refusal"])

    def test_group_cleanup_failure_preserves_original_refusal_and_reaps_leader(self):
        fault = """
def fail_cleanup(process):
    raise PermissionError('group cleanup probe')
namespace['main'].__globals__['stop_process_tree'] = fail_cleanup
"""
        result, report, _ = self.run_child(
            "import time; time.sleep(5)", timeout=0.1, fault=fault
        )
        self.assertEqual(result.returncode, 125, result.stderr)
        self.assertEqual(report["refusal"], "wall-time-exceeded")
        self.assertEqual(
            report["cleanup_errors"], ["PermissionError: group cleanup probe"]
        )
        self.assertIsNotNone(report["returncode"])
        self.assertNotEqual(report["returncode"], 0)

    @unittest.skipIf(os.name == "nt", "POSIX cooperative interrupt grace")
    def test_forced_stop_cannot_claim_separate_descendant_cleanup(self):
        fault = """
original_start = namespace['run'].__globals__['subprocess'].Popen
def start(*a, **kw):
    child = original_start(*a, **kw)
    ready = __import__('pathlib').Path(a[0][-1]) / 'handler-ready'
    deadline = __import__('time').monotonic() + 3
    while not ready.exists():
        if __import__('time').monotonic() >= deadline: raise RuntimeError('handler readiness deadline')
        __import__('time').sleep(.005)
    return child
namespace['run'].__globals__['subprocess'].Popen = start
"""
        result, report, _ = self.run_child(
            "import signal,time,pathlib,sys; signal.signal(signal.SIGINT, signal.SIG_IGN); (pathlib.Path(sys.argv[1])/'handler-ready').write_text('ready'); time.sleep(5)",
            timeout=0.2,
            fault=fault,
        )
        self.assertEqual(result.returncode, 125, result.stderr)
        self.assertEqual(report["refusal"], "wall-time-exceeded")
        self.assertIn(
            "escaped descendant cleanup is unconfirmed", report["cleanup_errors"][0]
        )
        self.assertIsNotNone(report["returncode"])
        self.assertLess(report["elapsed_seconds"], 4)

    def test_late_reader_failure_does_not_replace_original_timeout(self):
        fault = """
original_start = namespace['main'].__globals__['subprocess'].Popen
class BrokenReader:
    def __init__(self, original):
        self.original = original
    def close(self):
        self.original.close()
    def read1(self, size):
        __import__('time').sleep(0.3)
        raise OSError('late reader probe')
def start(*a, **kw):
    child = original_start(*a, **kw)
    child.stdout = BrokenReader(child.stdout)
    return child
namespace['main'].__globals__['subprocess'].Popen = start
"""
        result, report, _ = self.run_child(
            "import time; time.sleep(5)", timeout=0.1, fault=fault
        )
        self.assertEqual(result.returncode, 125, result.stderr)
        self.assertEqual(report["refusal"], "wall-time-exceeded")
        self.assertEqual(report["reader_errors"], ["late reader probe"])

    @unittest.skipUnless(
        sys.platform == "darwin", "Darwin exited-group EPERM inspection"
    )
    def test_exited_group_eperm_requires_empty_native_process_snapshot(self):
        fault = """
def denied_group(*_):
    raise PermissionError('exited group probe')
namespace['main'].__globals__['os'].killpg = denied_group
"""
        result, report, log = self.run_child("print('completed')", fault=fault)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(report["cleanup_errors"], [])
        self.assertEqual(log, b"completed\n")

    @unittest.skipUnless(sys.platform == "darwin", "Darwin live-group EPERM rejection")
    def test_live_group_eperm_is_not_ignored(self):
        fault = """
def denied_group(*_):
    raise PermissionError('live group probe')
namespace['main'].__globals__['os'].killpg = denied_group
namespace['main'].__globals__['subprocess'].check_output = lambda *a, **kw: str(__import__('os').getpid()) + ' R\\n'
original_start = namespace['main'].__globals__['subprocess'].Popen
def start(*a, **kw):
    child = original_start(*a, **kw)
    namespace['main'].__globals__['subprocess'].check_output = lambda *a, **kw: str(child.pid) + ' R\\n'
    return child
namespace['main'].__globals__['subprocess'].Popen = start
"""
        result, report, _ = self.run_child("print('completed')", fault=fault)
        self.assertEqual(result.returncode, 125, result.stderr)
        self.assertEqual(report["refusal"], "process-cleanup-failed")
        self.assertTrue(report["cleanup_errors"])

    @unittest.skipIf(os.name == "nt", "POSIX separately grouped child cleanup")
    def test_cooperative_interrupt_cleans_separate_child_group(self):
        self.separately_grouped_child_cleanup()

    @unittest.skipIf(os.name == "nt", "POSIX separately grouped child cleanup")
    def test_log_cap_does_not_block_separate_child_group_cleanup(self):
        self.separately_grouped_child_cleanup(overflow=True)

    def separately_grouped_child_cleanup(
        self, overflow=False, defective=False, readiness_failure=False
    ):
        # The test retains the separately sessioned child's unreaped Popen
        # identity until the cooperative parent can make no further group signals.
        # This also gives finally cleanup authority if the owner/probe fails.
        from tools.htmlcut_maintenance import process as owner
        import shutil
        import signal

        root = Path(
            tempfile.mkdtemp(
                prefix="htmlcut-resource-group.",
                dir=os.environ.get("RUNNER_TEMP") or None,
            )
        )
        marker, ready, child_ready, release = [
            root / name
            for name in ("separate-child-wrote", "ready", "child-ready", "release")
        ]
        descendant = None
        parents = []
        confirmed = True
        passed = False
        cleanup_errors = []
        original_start = subprocess.Popen
        child = (
            "import pathlib,time,sys\n"
            f"pathlib.Path({str(child_ready)!r}).write_text('ready')\n"
            "deadline=time.monotonic()+10\n"
            f"while not pathlib.Path({str(release)!r}).exists():\n"
            " if time.monotonic()>=deadline: sys.exit(42)\n"
            " time.sleep(.005)\n"
            f"pathlib.Path({str(marker)!r}).write_text('leaked')\n"
        )
        try:
            descendant = original_start(
                [sys.executable, "-c", child],
                start_new_session=True,
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
            )
            deadline = time.monotonic() + 3
            while not child_ready.exists() and time.monotonic() < deadline:
                time.sleep(0.005)
            self.assertTrue(child_ready.exists(), "descendant readiness deadline")
            parent = (
                "import os,signal,sys,time,pathlib\n"
                "def flood(byte):\n"
                " data=byte*(2*1024*1024)\n"
                " while data:\n"
                "  count=os.write(1,data)\n"
                "  if count <= 0: raise OSError('stdout made no progress')\n"
                "  data=data[count:]\n"
                "def stop(*_):\n"
                " try:\n"
                "  try:\n"
                + ("   flood(b'c')\n" if overflow else "   pass\n")
                + "  finally:\n"
                + (
                    "   pass\n"
                    if defective
                    else f"   os.killpg({descendant.pid},signal.SIGKILL)\n"
                )
                + " except BaseException as error:\n"
                + f"  pathlib.Path({str(root / 'handler-error.txt')!r}).write_bytes((type(error).__name__+': '+str(error)).encode('utf-8',errors='replace')[:4096])\n"
                + "  raise\n"
                + " sys.exit(0)\n"
                "signal.signal(signal.SIGINT,stop)\n"
                + (
                    "sys.exit(43)\n"
                    if readiness_failure
                    else f"pathlib.Path({str(ready)!r}).write_text('ready')\n"
                )
                + ("flood(b'x')\n" if overflow else "")
                + "time.sleep(5)\n"
            )

            def start(*args, **kwargs):
                process = original_start(*args, **kwargs)
                parents.append(process)
                deadline = time.monotonic() + 3
                while not ready.exists():
                    if time.monotonic() >= deadline:
                        raise RuntimeError("parent readiness deadline")
                    time.sleep(0.005)
                return process

            with mock.patch.object(owner.subprocess, "Popen", side_effect=start):
                if readiness_failure:
                    with self.assertRaisesRegex(RuntimeError, "readiness deadline"):
                        owner.run(
                            [sys.executable, "-c", parent],
                            root,
                            disk_limit_mib=4,
                            log_limit_mib=1,
                            timeout=0.5,
                            poll_seconds=0.02,
                            label="test",
                        )
                    return
                report = owner.run(
                    [sys.executable, "-c", parent],
                    root,
                    disk_limit_mib=4,
                    log_limit_mib=1,
                    timeout=0.5,
                    poll_seconds=0.02,
                    label="test",
                )
            self.assertEqual(owner.status(report), 125)
            self.assertEqual(
                report["returncode"],
                0,
                "cooperative cleanup handler must complete, including its output",
            )
            self.assertEqual(
                report["refusal"],
                "log-budget-exceeded" if overflow else "wall-time-exceeded",
            )
            self.assertEqual(report["cleanup_errors"], [])
            # Only after the owner's completed cleanup report can an escaped
            # descendant write the marker. The defective control must do so.
            release.write_text("released after cleanup")
            descendant.wait(timeout=2)
            self.assertEqual(
                marker.exists(), defective, "post-cleanup descendant-release oracle"
            )
            passed = True
        finally:
            # Do not reap a group leader before its last numeric group signal.
            # Popen.returncode is inspected without poll(): unknown means the
            # directly owned identity is still retained until this kill/wait.
            for process in parents:
                try:
                    if process.returncode is None:
                        owner.stop_process_tree(process)
                        process.wait(timeout=2)
                    if process.stdout is not None:
                        process.stdout.close()
                except (OSError, subprocess.TimeoutExpired) as error:
                    confirmed = False
                    cleanup_errors.append(f"parent: {error}")
                    print(
                        f"Parent cleanup unconfirmed; retained {root}: {error}",
                        file=sys.stderr,
                    )
            if descendant is not None:
                try:
                    if confirmed:
                        release.touch()
                    if descendant.returncode is None:
                        owner.stop_process_tree(descendant)
                        descendant.wait(timeout=2)
                except (OSError, subprocess.TimeoutExpired) as error:
                    confirmed = False
                    cleanup_errors.append(f"descendant: {error}")
                    print(
                        f"Descendant cleanup unconfirmed; retained {root}: {error}",
                        file=sys.stderr,
                    )
            if confirmed and passed:
                shutil.rmtree(root)
            else:
                retain_fixture_diagnostics(root, cleanup_errors)
            if not confirmed and sys.exc_info()[0] is None:
                self.fail(f"fixture cleanup was not confirmed; retained {root}")

    @unittest.skipIf(os.name == "nt", "POSIX separately grouped child cleanup")
    def test_readiness_failure_still_tears_down_the_owned_separate_descendant(self):
        self.separately_grouped_child_cleanup(readiness_failure=True)

    @unittest.skipIf(os.name == "nt", "POSIX separately grouped child cleanup")
    def test_descendant_release_detects_deliberately_defective_cleanup(self):
        self.separately_grouped_child_cleanup(defective=True)

    @unittest.skipIf(os.name == "nt", "POSIX failed-observation fixture retention")
    def test_failed_observation_retains_fixture_after_confirmed_teardown(self):
        root = Path(
            tempfile.mkdtemp(
                prefix="htmlcut-resource-group.",
                dir=os.environ.get("RUNNER_TEMP") or None,
            )
        )
        case = root / "case"
        case.mkdir()
        passed = False
        try:
            with (
                mock.patch.object(tempfile, "mkdtemp", return_value=str(case)),
                mock.patch.object(
                    self,
                    "assertEqual",
                    side_effect=AssertionError("controlled observation failure"),
                ),
            ):
                with self.assertRaisesRegex(
                    AssertionError, "controlled observation failure"
                ):
                    self.separately_grouped_child_cleanup()
            self.assertTrue(
                case.is_dir(), "successful teardown erased a failed observation"
            )
            facts = json.loads((case / "fixture-diagnostic.json").read_text())
            self.assertEqual(facts["cleanup_errors"], [])
            self.assertEqual(json.loads(facts["test.resources.json"])["returncode"], 0)
            self.assertIn("controlled observation failure", facts["observation_error"])
            passed = True
        finally:
            if passed:
                shutil.rmtree(root)
            else:
                retain_fixture_diagnostics(case, [])

    @unittest.skipIf(os.name == "nt", "POSIX process-group orphan cleanup")
    def test_exited_parent_group_cleanup_stops_a_gated_background_writer(self):
        root = Path(
            tempfile.mkdtemp(
                prefix="htmlcut-exited-group.",
                dir=os.environ.get("RUNNER_TEMP") or None,
            )
        )
        marker, gate, ready, expired = (
            root / name
            for name in [
                "orphan-wrote",
                "write-after-cleanup",
                "writer-ready",
                "writer-expired",
            ]
        )
        child = (
            "import os,pathlib,time\n"
            "ready=pathlib.Path(" + repr(str(ready)) + ")\n"
            "ready.with_suffix('.pending').write_text(str(os.getpgrp())); ready.with_suffix('.pending').replace(ready)\n"
            "gate=pathlib.Path(" + repr(str(gate)) + ")\n"
            "deadline=time.monotonic()+10\n"
            "while not gate.exists() and time.monotonic()<deadline: time.sleep(.01)\n"
            "if gate.exists(): pathlib.Path("
            + repr(str(marker))
            + ").write_text('leaked')\n"
            "else: pathlib.Path("
            + repr(str(expired))
            + ").write_text('natural expiry')\n"
        )
        parent_code = (
            "import pathlib,subprocess,sys,time\n"
            "subprocess.Popen([sys.executable,'-c'," + repr(child) + "])\n"
            "ready=pathlib.Path(" + repr(str(ready)) + ")\n"
            "deadline=time.monotonic()+3\n"
            "while not ready.exists() and time.monotonic()<deadline: time.sleep(.01)\n"
            "assert ready.exists(), 'background writer did not become ready'\n"
        )
        parent = subprocess.Popen(
            [sys.executable, "-c", parent_code],
            start_new_session=True,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
        passed = False
        cleanup_errors = []

        def live_group_members():
            listing = subprocess.check_output(
                ["ps", "-axo", "pgid=,pid=,stat="], text=True, timeout=2
            )
            members = []
            for line in listing.splitlines():
                if not line.strip():
                    continue
                group, pid, state = line.split()
                if int(group) == parent.pid and not state.startswith("Z"):
                    members.append((int(pid), state))
            return members

        try:
            deadline = time.monotonic() + 4
            while (
                os.waitid(os.P_PID, parent.pid, os.WEXITED | os.WNOHANG | os.WNOWAIT)
                is None
                and time.monotonic() < deadline
            ):
                time.sleep(0.01)
            exited = os.waitid(
                os.P_PID, parent.pid, os.WEXITED | os.WNOHANG | os.WNOWAIT
            )
            self.assertIsNotNone(exited, "parent did not exit after writer readiness")
            self.assertEqual((exited.si_code, exited.si_status), (os.CLD_EXITED, 0))
            self.assertEqual(int(ready.read_text()), parent.pid)
            # The exited leader remains waitable, reserving this group identity
            # through both the actual helper and the independent final signal.
            runpy.run_path(str(RUNNER))["stop_process_tree"](parent)
            deadline = time.monotonic() + 2
            members = live_group_members()
            while members and time.monotonic() < deadline:
                time.sleep(0.01)
                members = live_group_members()
            self.assertEqual(members, [], "owned group still has live members")
            self.assertFalse(
                expired.exists(), "writer expired before group cleanup could be proved"
            )
            gate.write_text("write only after confirmed group termination")
            self.assertFalse(
                marker.exists(), "background writer survived group cleanup"
            )
            passed = True
        finally:
            try:
                # Do not use the possibly faulted helper for fixture cleanup.
                os.killpg(parent.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
            except PermissionError as error:
                try:
                    if live_group_members():
                        cleanup_errors.append(str(error))
                except (
                    OSError,
                    ValueError,
                    subprocess.SubprocessError,
                ) as inspect_error:
                    cleanup_errors.append(str(inspect_error))
            except OSError as error:
                cleanup_errors.append(str(error))
            try:
                deadline = time.monotonic() + 2
                members = live_group_members()
                while members and time.monotonic() < deadline:
                    time.sleep(0.01)
                    members = live_group_members()
                if members:
                    cleanup_errors.append(f"live owned group members: {members}")
                parent.wait(timeout=2)
            except (OSError, ValueError, subprocess.SubprocessError) as error:
                cleanup_errors.append(str(error))
            if passed and not cleanup_errors:
                shutil.rmtree(root)
            else:
                retain_fixture_diagnostics(root, cleanup_errors)
            if cleanup_errors:
                self.fail(
                    f"Exited-group fixture cleanup unconfirmed at {root}: {cleanup_errors}"
                )

    @unittest.skipUnless(
        os.name == "nt", "requires actual Windows taskkill/process handles"
    )
    def test_native_windows_tree_cancellation_detects_leader_only_stop(self):
        # Native handles and creation times pin the command leader and its
        # child writer before either cancellation primitive can run.
        from ctypes import wintypes
        from tools.htmlcut_maintenance import process as owner

        kernel = ctypes.WinDLL("kernel32", use_last_error=True)
        kernel.OpenProcess.argtypes = [wintypes.DWORD, wintypes.BOOL, wintypes.DWORD]
        kernel.OpenProcess.restype = wintypes.HANDLE
        kernel.WaitForSingleObject.argtypes = [wintypes.HANDLE, wintypes.DWORD]
        kernel.WaitForSingleObject.restype = wintypes.DWORD
        kernel.GetProcessTimes.argtypes = [wintypes.HANDLE] + [
            ctypes.POINTER(wintypes.FILETIME)
        ] * 4
        kernel.GetProcessTimes.restype = wintypes.BOOL

        def birth(handle):
            times = [wintypes.FILETIME() for _ in range(4)]
            if not kernel.GetProcessTimes(
                handle, *(ctypes.byref(value) for value in times)
            ):
                raise ctypes.WinError(ctypes.get_last_error())
            return times[0].dwLowDateTime | (times[0].dwHighDateTime << 32)

        def owned_handle(pid, expected_birth):
            handle = kernel.OpenProcess(0x00101001, False, pid)
            if not handle:
                raise ctypes.WinError(ctypes.get_last_error())
            try:
                if birth(handle) != expected_birth:
                    raise AssertionError(
                        "process identity changed before handle acquisition"
                    )
            except (OSError, AssertionError):
                kernel.CloseHandle(handle)
                raise
            return handle

        kernel.TerminateProcess.argtypes = [wintypes.HANDLE, wintypes.UINT]
        kernel.TerminateProcess.restype = wintypes.BOOL
        kernel.CloseHandle.argtypes = [wintypes.HANDLE]
        kernel.CloseHandle.restype = wintypes.BOOL
        root = Path(
            tempfile.mkdtemp(
                prefix="htmlcut-process-tree.",
                dir=os.environ.get("RUNNER_TEMP") or None,
            )
        )
        controls = []
        facts = []
        confirmed = False
        observations_complete = False
        try:
            for leader_only in [False, True]:
                owned = root / ("leader-only" if leader_only else "tree-stop")
                owned.mkdir()
                ready, writer_ready, marker, start = (
                    owned / name
                    for name in ["ready.json", "writer-ready", "late-write", "start"]
                )
                writer_code = (
                    "from pathlib import Path; import os,sys,time; "
                    'p=Path(sys.argv[1]); p.with_suffix(".pending").write_text(str(os.getpid())); p.with_suffix(".pending").replace(p); '
                    "deadline=time.monotonic()+15; "
                    "\nwhile not Path(sys.argv[3]).exists() and time.monotonic()<deadline: time.sleep(.01)\n"
                    "if not Path(sys.argv[3]).exists(): sys.exit(2)\n"
                    'Path(sys.argv[2]).write_text("writer survived"); time.sleep(15)'
                )
                child_code = """from pathlib import Path
import ctypes,json,os,subprocess,sys,time
from ctypes import wintypes as W
k=ctypes.WinDLL('kernel32',use_last_error=True)
k.GetCurrentProcess.restype=W.HANDLE
k.GetProcessTimes.argtypes=[W.HANDLE]+[ctypes.POINTER(W.FILETIME)]*4
k.GetProcessTimes.restype=W.BOOL
def birth(handle):
    times=[W.FILETIME() for _ in range(4)]
    if not k.GetProcessTimes(handle,*(ctypes.byref(value) for value in times)):
        raise ctypes.WinError(ctypes.get_last_error())
    return times[0].dwLowDateTime | (times[0].dwHighDateTime<<32)
writer=subprocess.Popen([sys.executable,'-c',sys.argv[1],sys.argv[3],sys.argv[4],sys.argv[5]], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
p=Path(sys.argv[2])
p.with_suffix('.pending').write_text(json.dumps({
    'child':{'pid':os.getpid(),'birth':birth(k.GetCurrentProcess())},
    'writer':{'pid':writer.pid,'birth':birth(int(writer._handle))}}))
p.with_suffix('.pending').replace(p)
time.sleep(30)
"""
                command = [
                    sys.executable,
                    "-c",
                    child_code,
                    writer_code,
                    str(ready),
                    str(writer_ready),
                    str(marker),
                    str(start),
                ]
                control = {"process": None, "handles": [], "owned": owned}
                cancellation = threading.Event()
                original_start = subprocess.Popen

                def start_command(*args, **kwargs):
                    process = original_start(*args, **kwargs)
                    if args[0] != command:
                        return process
                    control["process"] = process
                    controls.append(control)
                    deadline = time.monotonic() + 5
                    while (
                        not ready.exists() or not writer_ready.exists()
                    ) and time.monotonic() < deadline:
                        time.sleep(0.01)
                    self.assertTrue(
                        ready.exists() and writer_ready.exists(),
                        "owned writer did not start",
                    )
                    identities = json.loads(ready.read_text())
                    self.assertEqual(
                        int(writer_ready.read_text()), identities["writer"]["pid"]
                    )
                    for role in ["child", "writer"]:
                        expected = identities[role]
                        handle = owned_handle(expected["pid"], expected["birth"])
                        control["handles"].append((role, expected["pid"], handle))
                    with self.assertRaisesRegex(AssertionError, "identity changed"):
                        owned_handle(
                            identities["writer"]["pid"],
                            identities["writer"]["birth"] + 1,
                        )
                    self.assertFalse(
                        marker.exists(), "writer fired before cancellation"
                    )
                    cancellation.set()
                    return process

                # The negative control changes only the actual owner's stop
                # primitive. No nested watchdog/controller process is restored.
                primitive = (
                    mock.patch.object(
                        owner,
                        "stop_process_tree",
                        side_effect=lambda process: process.terminate(),
                    )
                    if leader_only
                    else contextlib.nullcontext()
                )
                with (
                    mock.patch.object(
                        owner.subprocess, "Popen", side_effect=start_command
                    ),
                    primitive,
                ):
                    report = owner.run(
                        command,
                        owned,
                        disk_limit_mib=8,
                        timeout=15,
                        log_limit_mib=1,
                        label="control",
                        cancelled=cancellation,
                    )
                process = control["process"]
                self.assertEqual(report["refusal"], "interrupted", report)
                self.assertIsNotNone(report["returncode"])
                self.assertEqual(report["cleanup_errors"], [], report)
                start.write_text("write only after owner returned its cleanup report")
                if leader_only:
                    deadline = time.monotonic() + 4
                    while not marker.exists() and time.monotonic() < deadline:
                        time.sleep(0.01)
                    self.assertEqual(marker.read_text(), "writer survived")
                    self.assertEqual(
                        kernel.WaitForSingleObject(control["handles"][1][2], 0), 258
                    )
                else:
                    for role, _, handle in control["handles"]:
                        self.assertEqual(
                            kernel.WaitForSingleObject(handle, 2000),
                            0,
                            f"{role} survived tree stop",
                        )
                    self.assertFalse(marker.exists())
                facts.append(
                    {
                        "case": "leader-only" if leader_only else "tree-stop",
                        "late_writer_marker": marker.exists(),
                        "reported_cleanup_errors": report["cleanup_errors"],
                        "command_returncode": process.returncode,
                    }
                )
            self.assertEqual([x["late_writer_marker"] for x in facts], [False, True])
            observations_complete = True
        finally:
            failures, cleanup_attempt_errors = [], []
            for control in controls:
                process = control["process"]
                try:
                    if process.poll() is None:
                        try:
                            subprocess.run(
                                ["taskkill", "/PID", str(process.pid), "/T", "/F"],
                                stdout=subprocess.DEVNULL,
                                stderr=subprocess.DEVNULL,
                                check=True,
                                timeout=5,
                            )
                        except (OSError, subprocess.SubprocessError) as error:
                            cleanup_attempt_errors.append("command tree: " + str(error))
                            process.kill()
                        process.wait(timeout=2)
                except (OSError, subprocess.SubprocessError) as error:
                    failures.append("command cleanup: " + str(error))
                if len(control["handles"]) != 2:
                    failures.append(
                        "descendant handles were not both pinned before failure"
                    )
                # Both handles keep exact process identities owned. Stop each
                # still-live process even if another cleanup action failed.
                for role, pid, handle in control["handles"]:
                    if kernel.WaitForSingleObject(handle, 0) == 258:
                        try:
                            subprocess.run(
                                ["taskkill", "/PID", str(pid), "/T", "/F"],
                                stdout=subprocess.DEVNULL,
                                stderr=subprocess.DEVNULL,
                                check=True,
                                timeout=5,
                            )
                        except (OSError, subprocess.SubprocessError) as error:
                            cleanup_attempt_errors.append(role + " tree: " + str(error))
                        if kernel.WaitForSingleObject(handle, 0) == 258:
                            if not kernel.TerminateProcess(handle, 1):
                                cleanup_attempt_errors.append(
                                    role
                                    + " handle termination: "
                                    + str(ctypes.get_last_error())
                                )
                for role, _, handle in control["handles"]:
                    if kernel.WaitForSingleObject(handle, 2000) != 0:
                        failures.append(f"{role} cleanup not confirmed")
                    if not kernel.CloseHandle(handle):
                        failures.append(role + " handle close failed")
            confirmed = not failures
            if confirmed and observations_complete:
                shutil.rmtree(root)
            else:
                (root / "control-failure.json").write_text(
                    json.dumps(
                        {
                            "facts": facts,
                            "cleanup_failures": failures,
                            "cleanup_attempt_errors": cleanup_attempt_errors,
                        },
                        indent=2,
                    )
                )
                print(
                    f"Owned process evidence retained at {root}: {failures}",
                    file=sys.stderr,
                )
            if failures:
                self.fail(f"Owned process cleanup unconfirmed at {root}: {failures}")
        self.assertTrue(confirmed)
        print(
            "native Windows direct-owner cancellation control: "
            + json.dumps(facts, sort_keys=True)
        )

    def test_existing_report_is_never_overwritten(self):
        with tempfile.TemporaryDirectory() as directory:
            report = Path(directory) / "test.resources.json"
            report.write_text("immutable")
            result = subprocess.run(
                [
                    sys.executable,
                    str(RUNNER),
                    "--root",
                    directory,
                    "--disk-limit-mib",
                    "1",
                    "--timeout",
                    "1",
                    "--label",
                    "test",
                    "--",
                    sys.executable,
                    "-c",
                    "raise Exception('must not run')",
                ],
                capture_output=True,
            )
            self.assertEqual(result.returncode, 2)
            self.assertEqual(report.read_text(), "immutable")
            self.assertFalse((Path(directory) / "test.log").exists())

    @unittest.skipIf(os.name == "nt", "POSIX nonreaping child ownership")
    def test_native_exit_and_cooperation_keep_leader_waitable_until_group_signal(self):
        fault = """
functions = namespace['main'].__globals__
original_start = functions['subprocess'].Popen
original_group = functions['os'].killpg
owned = None
group_attempted = False
def forbidden_poll():
    raise AssertionError('Popen.poll would release the child identity')
def start(*args, **kwargs):
    global owned
    child = original_start(*args, **kwargs)
    if owned is None:
        owned = child
        original_wait = child.wait
        child.poll = forbidden_poll
        def wait(*args, **kwargs):
            if not group_attempted:
                raise AssertionError('wait before group cleanup releases ownership')
            return original_wait(*args, **kwargs)
        child.wait = wait
    return child
def group(pid, sig):
    global group_attempted
    assert pid == owned.pid
    observed = functions['os'].waitid(functions['os'].P_PID, pid, functions['os'].WEXITED | functions['os'].WNOHANG | functions['os'].WNOWAIT)
    assert observed is not None and observed.si_pid == pid, 'exited leader must remain waitable at group signal'
    group_attempted = True
    return original_group(pid, sig)
functions['subprocess'].Popen = start
functions['os'].killpg = group
"""
        for code, timeout, expected in [
            ("print('completed')", 2, 0),
            ("import time; time.sleep(5)", 0.1, 125),
        ]:
            with self.subTest(timeout=timeout):
                result, report, _ = self.run_child(code, timeout=timeout, fault=fault)
                self.assertEqual(result.returncode, expected, result.stderr)
                self.assertEqual(report["cleanup_errors"], [], report)

    @unittest.skipIf(os.name == "nt", "POSIX lost-child identity refusal")
    def test_reaped_identity_cannot_signal_or_wait_for_a_reused_numeric_id(self):
        fault = """
functions = namespace['main'].__globals__
original_exited = functions['child_exited']
original_start = functions['subprocess'].Popen
original_stop = functions['stop_process_tree']
original_sample = functions['disk_bytes']
children = []
lost = False
sampled = False
def forbidden(*args, **kwargs):
    functions['os'].write(2, b'identity-reuse-violation\\n')
    raise ChildProcessError('identity lost: numeric PID/group must not be touched')
def start(*args, **kwargs):
    child = original_start(*args, **kwargs)
    child.kill = forbidden
    child.wait = forbidden
    child.poll = forbidden
    children.append(child)
    return child
def lose_identity(process):
    global lost
    if lost:
        forbidden()
    if not original_exited(process):
        return False
    functions['os'].waitpid(process.pid, 0)
    lost = True
    raise ChildProcessError('externally reaped ownership probe')
def stop(process):
    if lost:
        forbidden()
    original_stop(process)
def denied_after_exit(root):
    global sampled
    if not sampled:
        sampled = True
        deadline = functions['time'].monotonic()+2
        while not original_exited(children[0]):
            if functions['time'].monotonic()>=deadline:
                raise RuntimeError('child did not exit for ownership control')
            functions['time'].sleep(.005)
        raise PermissionError(13, 'first sampling denial')
    return original_sample(root)
functions['subprocess'].Popen = start
functions['child_exited'] = lose_identity
functions['stop_process_tree'] = stop
functions['os'].killpg = forbidden
functions['os'].kill = forbidden
if FORCE_ACCESS:
    functions['disk_bytes'] = denied_after_exit
"""
        for access_denied in (False, True):
            with self.subTest(access_denied=access_denied):
                result, report, log = self.run_child(
                    "print('completed')", fault=fault.replace("FORCE_ACCESS", repr(access_denied))
                )
                self.assertEqual(result.returncode, 125, result.stderr)
                self.assertIsNone(report["returncode"], report)
                self.assertTrue(report["refusal"].startswith("monitor-failed: ChildProcessError:"), report)
                self.assertIn("externally reaped ownership probe", report["refusal"])
                self.assertEqual(report["cleanup_errors"], [
                    "child identity lost; no further PID/group signals or waits attempted"
                ])
                self.assertEqual(log, b"completed\n")
                self.assertEqual(report["disk_scan_access_errors"], int(access_denied))

    @unittest.skipIf(os.name == "nt", "POSIX ownership prerequisites")
    def test_unavailable_waitid_and_nondefault_sigchld_refuse_before_launch(self):
        for fault in [
            "del namespace['main'].__globals__['os'].WNOWAIT",
            "namespace['main'].__globals__['signal'].signal(__import__('signal').SIGCHLD, __import__('signal').SIG_IGN)",
            "namespace['main'].__globals__['signal'].signal(__import__('signal').SIGCHLD, lambda *_: None)",
        ]:
            with self.subTest(fault=fault), tempfile.TemporaryDirectory() as directory:
                wrapper = (
                    "import runpy,sys\nnamespace=runpy.run_path(sys.argv[1])\nsys.argv=sys.argv[1:]\n"
                    + fault
                    + "\nsys.exit(namespace['main']())\n"
                )
                result = subprocess.run(
                    [
                        sys.executable,
                        "-c",
                        wrapper,
                        str(RUNNER),
                        "--root",
                        directory,
                        "--disk-limit-mib",
                        "1",
                        "--timeout",
                        "1",
                        "--label",
                        "test",
                        "--",
                        sys.executable,
                        "-c",
                        "raise AssertionError('must not launch')",
                    ],
                    capture_output=True,
                    text=True,
                    timeout=3,
                )
                self.assertEqual(result.returncode, 2, result.stderr)
                self.assertTrue(
                    "waitid/WNOWAIT" in result.stderr or "SIGCHLD" in result.stderr
                )
                self.assertFalse(list(Path(directory).iterdir()))

    @unittest.skipIf(os.name == "nt", "POSIX ownership prerequisite")
    def test_commands_refuse_missing_ownership_capability_before_cargo(self):
        for command in [["check"], ["qa", "fuzz"]]:
            code = "import os,sys,runpy; del os.WNOWAIT; sys.argv=['maintenance',*sys.argv[1:]];runpy.run_module('tools.htmlcut_maintenance',run_name='__main__')"
            result = subprocess.run(
                [sys.executable, "-c", code, *command],
                cwd=ROOT,
                capture_output=True,
                text=True,
                timeout=3,
            )
            self.assertEqual(result.returncode, 2, result.stderr)
            self.assertIn("waitid/WNOWAIT", result.stderr)


class DirectOwnershipTests(unittest.TestCase):
    @unittest.skipIf(os.name == "nt", "POSIX cooperative SIGTERM delivery")
    def test_direct_api_owns_signal_cleanup_without_an_argparse_wrapper(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            ready = root / "ready"
            child = (
                "import pathlib,os,time;pathlib.Path("
                + repr(str(ready))
                + ").write_text(str(os.getpid()));time.sleep(5)"
            )
            code = "from tools.htmlcut_maintenance import process;import sys;result=process.run([sys.executable,'-c',sys.argv[2]],sys.argv[1],disk_limit_mib=4,timeout=10,poll_seconds=.02,label='direct');sys.exit(process.status(result))"
            owner = subprocess.Popen(
                [sys.executable, "-c", code, str(root), child],
                cwd=ROOT,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
            )
            try:
                deadline = time.monotonic() + 3
                while not ready.exists() and time.monotonic() < deadline:
                    time.sleep(0.005)
                self.assertTrue(
                    ready.exists(), "direct owner child readiness was not observed"
                )
                owner.terminate()
                _, stderr = owner.communicate(timeout=8)
                self.assertEqual(owner.returncode, 125, stderr)
                report = json.loads((root / "direct.resources.json").read_text())
                self.assertEqual(report["refusal"], "interrupted")
                self.assertEqual(report["cleanup_errors"], [])
                self.assertIsNotNone(report["returncode"])
                with self.assertRaises(ProcessLookupError):
                    os.kill(int(ready.read_text()), 0)
            finally:
                if owner.poll() is None:
                    owner.kill()
                    owner.communicate(timeout=8)
