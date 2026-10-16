# SPDX-License-Identifier: MPL-2.0
"""Real child writers prove QA resource refusals and retained evidence."""

import json
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


class ResourceBudgetTests(unittest.TestCase):
    def test_windows_access_denied_accepts_only_a_now_missing_namespace_entry(self):
        functions = runpy.run_path(str(RUNNER))
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            sentinel = root / "preserved"
            sentinel.write_bytes(b"counted")
            removed = root / "removed"
            removed.mkdir()
            removed.rmdir()
            error = PermissionError(13, "removal window", str(removed))
            error.winerror = 5

            def failed_walk(*_, onerror, **__):
                onerror(error)
                return [(str(root), [], [sentinel.name])]

            with mock.patch.object(os, "walk", side_effect=failed_walk):
                self.assertGreater(functions["disk_bytes"](root), 0)

            original = Path.lstat

            def failed_entry(path, *args, **kwargs):
                if path == removed:
                    raise error
                return original(path, *args, **kwargs)

            with (
                mock.patch.object(
                    os,
                    "walk",
                    return_value=[(str(root), [], [sentinel.name, removed.name])],
                ),
                mock.patch.object(Path, "lstat", failed_entry),
            ):
                self.assertGreater(functions["disk_bytes"](root), 0)

            removed.mkdir()
            with mock.patch.object(os, "walk", side_effect=failed_walk):
                with self.assertRaises(PermissionError) as raised:
                    functions["disk_bytes"](root)
                self.assertIs(raised.exception, error)

            if os.name != "nt":
                removed.rmdir()
                removed.symlink_to(root / "missing-target", target_is_directory=True)
                # lstat must inspect the link itself, not follow its absent target.
                with mock.patch.object(os, "walk", side_effect=failed_walk):
                    with self.assertRaises(PermissionError) as raised:
                        functions["disk_bytes"](root)
                    self.assertIs(raised.exception, error)

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

    def test_poll_interval_respects_remaining_deadline(self):
        result, report, _ = self.run_child(
            "import time; time.sleep(5)", timeout=0.1, poll="10"
        )
        self.assertEqual(result.returncode, 125, result.stderr)
        self.assertEqual(report["refusal"], "wall-time-exceeded")
        self.assertLess(report["elapsed_seconds"], 2)

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

    def final_sample_failure(self, code, timeout=2):
        fault = """
functions = namespace['main'].__globals__
original_stop = functions['stop_process_tree']
original_sample = functions['disk_bytes']
cleaned = False
def stop_child(process):
    global cleaned
    original_stop(process)
    cleaned = True
def sample(root):
    if cleaned:
        raise PermissionError('final sample probe')
    return original_sample(root)
functions['stop_process_tree'] = stop_child
functions['disk_bytes'] = sample
"""
        return self.run_child(code, timeout=timeout, fault=fault)

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

    def separately_grouped_child_cleanup(self, overflow=False, defective=False):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            marker, ready, child_ready, release = [
                root / name
                for name in ("separate-child-wrote", "ready", "child-ready", "release")
            ]
            child = (
                "import pathlib,time\n"
                f"pathlib.Path({str(child_ready)!r}).write_text('ready')\n"
                f"while not pathlib.Path({str(release)!r}).exists(): time.sleep(.005)\n"
                f"pathlib.Path({str(marker)!r}).write_text('leaked')\n"
            )
            parent = (
                "import os,signal,subprocess,sys,time,pathlib\n"
                f"child=subprocess.Popen([sys.executable,'-c',{child!r}],start_new_session=True,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)\n"
                "def stop(*_):\n"
                + (
                    " sys.stdout.write('c'*(2*1024*1024)); sys.stdout.flush()\n"
                    if overflow
                    else ""
                )
                + (
                    " pass\n"
                    if defective
                    else " os.killpg(child.pid,signal.SIGKILL)\n child.wait()\n"
                )
                + " sys.exit(0)\n"
                "signal.signal(signal.SIGINT,stop)\n"
                f"while not pathlib.Path({str(child_ready)!r}).exists(): time.sleep(.005)\n"
                f"pathlib.Path({str(ready)!r}).write_text('ready')\n"
                + (
                    "sys.stdout.write('x'*(2*1024*1024)); sys.stdout.flush()\n"
                    if overflow
                    else ""
                )
                + "time.sleep(5)\n"
            )
            wrapper = (
                "import runpy,sys,pathlib,time\n"
                "namespace=runpy.run_path(sys.argv[1]); sys.argv=sys.argv[1:]\n"
                "original=namespace['run'].__globals__['subprocess'].Popen\n"
                "def start(*a,**kw):\n"
                " child=original(*a,**kw)\n"
                f" ready=pathlib.Path({str(ready)!r}); deadline=time.monotonic()+3\n"
                " while not ready.exists():\n"
                "  if time.monotonic()>=deadline: raise RuntimeError('child readiness deadline')\n"
                "  time.sleep(.005)\n"
                " return child\n"
                "namespace['run'].__globals__['subprocess'].Popen=start\n"
                "sys.exit(namespace['main']())\n"
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
                    "4",
                    "--log-limit-mib",
                    "1",
                    "--timeout",
                    ".5",
                    "--poll-seconds",
                    ".02",
                    "--label",
                    "test",
                    "--",
                    sys.executable,
                    "-c",
                    parent,
                ],
                capture_output=True,
                timeout=8,
            )
            self.assertTrue(ready.exists(), result.stderr)
            self.assertEqual(result.returncode, 125, result.stderr)
            report = json.loads((root / "test.resources.json").read_text())
            self.assertEqual(
                report["refusal"],
                "log-budget-exceeded" if overflow else "wall-time-exceeded",
            )
            self.assertEqual(report["cleanup_errors"], [])
            # Release only after the owner's cleanup report exists. A descendant
            # cannot write the negative marker before the property is tested.
            release.write_text("released after cleanup")
            deadline = time.monotonic() + 1
            while not marker.exists() and time.monotonic() < deadline:
                time.sleep(0.005)
            if defective:
                self.assertTrue(
                    marker.exists(), "negative cleanup control did not leak"
                )
            else:
                self.assertFalse(
                    marker.exists(), "separately grouped child survived cancellation"
                )

    @unittest.skipIf(os.name == "nt", "POSIX separately grouped child cleanup")
    def test_descendant_release_detects_deliberately_defective_cleanup(self):
        self.separately_grouped_child_cleanup(defective=True)

    @unittest.skipIf(os.name == "nt", "POSIX process-group orphan cleanup")
    def test_successful_parent_cannot_leave_background_writer(self):
        with tempfile.TemporaryDirectory() as directory:
            marker = Path(directory) / "orphan-wrote"
            child = (
                "import pathlib,time; time.sleep(0.4); pathlib.Path("
                + repr(str(marker))
                + ").write_text('leaked')"
            )
            parent = (
                "import subprocess,sys; subprocess.Popen([sys.executable, '-c', "
                + repr(child)
                + "])"
            )
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
                    "--poll-seconds",
                    "0.02",
                    "--label",
                    "test",
                    "--",
                    sys.executable,
                    "-c",
                    parent,
                ],
                capture_output=True,
                timeout=5,
            )
            self.assertEqual(result.returncode, 0, result.stderr)
            time.sleep(0.5)
            self.assertFalse(
                marker.exists(), "background writer survived command completion"
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
def forbidden(*args, **kwargs):
    raise AssertionError('identity lost: numeric PID/group must not be touched')
def start(*args, **kwargs):
    child = original_start(*args, **kwargs)
    child.kill = forbidden
    child.wait = forbidden
    child.poll = forbidden
    return child
def lose_identity(process):
    if not original_exited(process):
        return False
    functions['os'].waitpid(process.pid, 0)
    raise ChildProcessError('externally reaped ownership probe')
functions['subprocess'].Popen = start
functions['child_exited'] = lose_identity
functions['os'].killpg = forbidden
functions['os'].kill = forbidden
"""
        result, report, log = self.run_child("print('completed')", fault=fault)
        self.assertEqual(result.returncode, 125, result.stderr)
        self.assertIsNone(report["returncode"], report)
        self.assertIn("externally reaped ownership probe", report["refusal"])
        self.assertTrue(report["cleanup_errors"])
        self.assertEqual(log, b"completed\n")

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
