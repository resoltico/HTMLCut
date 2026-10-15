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

ROOT = Path(__file__).resolve().parents[1]
RUNNER = ROOT / "scripts/run-resource-bounded.py"


class ResourceBudgetTests(unittest.TestCase):
    def test_shell_status_preserves_failures_outside_eight_bit_transport(self):
        status = runpy.run_path(str(RUNNER))["shell_exit_status"]
        for raw, expected in [(0, 0), (2, 2), (3, 3), (17, 17), (125, 125),
                              (255, 255), (256, 1), (512, 1), (0xC0000005, 1), (-9, 137)]:
            with self.subTest(raw=raw):
                self.assertEqual(status(raw), expected)

    @unittest.skipUnless(os.name == "nt", "native Windows DWORD status through Git Bash")
    def test_native_windows_wide_child_status_remains_failure_through_bash(self):
        bash = runpy.run_path(str(ROOT / "scripts/native-shell.py"))["bash_program"]()
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            command = [Path(sys.executable).as_posix(), RUNNER.as_posix(),
                       "--root", root.as_posix(), "--disk-limit-mib", "1",
                       "--log-limit-mib", "1", "--timeout", "2",
                       "--poll-seconds", ".02", "--label", "wide", "--",
                       Path(sys.executable).as_posix(), "-c",
                       "import ctypes; ctypes.windll.kernel32.ExitProcess(256)"]
            script = (shlex.join(command) + "; status=$?; "
                      "printf 'watchdog_status=%s\\n' \"$status\"; exit \"$status\"")
            result = subprocess.run([bash, "-c", script], capture_output=True,
                                    text=True, timeout=10)
            report = json.loads((root / "wide.resources.json").read_text())
            self.assertEqual(result.returncode, 1, result.stderr)
            self.assertEqual(result.stdout, "watchdog_status=1\n")
            self.assertEqual(report["returncode"], 256, report)
            self.assertIsNone(report["refusal"], report)
            self.assertEqual(report["cleanup_errors"], [], report)

    def run_child(self, code, timeout=2, disk_limit=1, poll="0.02", fault=None):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            marker = root / "owner-evidence"
            marker.write_text("preserve")
            invocation = [sys.executable, str(RUNNER)]
            if fault:
                wrapper = ("import runpy,sys\nnamespace=runpy.run_path(sys.argv[1])\n"
                           "sys.argv=sys.argv[1:]\n" + fault + "\nsys.exit(namespace['main']())\n")
                invocation = [sys.executable, "-c", wrapper, str(RUNNER)]
            result = subprocess.run([*invocation, "--root", directory,
                "--disk-limit-mib", str(disk_limit), "--log-limit-mib", "1", "--timeout", str(timeout),
                "--poll-seconds", str(poll), "--label", "test", "--", sys.executable, "-c", code,
                directory], capture_output=True, text=True, timeout=8)
            report = json.loads((root / "test.resources.json").read_text())
            log = (root / "test.log").read_bytes()
            self.assertEqual(marker.read_text(), "preserve")
            return result, report, log

    def test_nonfinite_time_or_sampling_cannot_disable_termination(self):
        for flag in ["--timeout", "--poll-seconds"]:
            for value in ["nan", "inf"]:
                with tempfile.TemporaryDirectory() as directory:
                    command=[sys.executable,str(RUNNER),"--root",directory,"--disk-limit-mib","1","--log-limit-mib","1","--timeout","1","--poll-seconds",".02","--label","test",flag,value,"--",sys.executable,"-c","raise RuntimeError('must not run')"]
                    result=subprocess.run(command,capture_output=True,timeout=3)
                    self.assertEqual(result.returncode,2)
                    self.assertFalse(list(Path(directory).iterdir()))

    def test_huge_finite_poll_cannot_overflow_or_leave_child_running(self):
        result, report, _ = self.run_child("import time; time.sleep(5)", timeout=0.1, poll="1e308")
        self.assertEqual(result.returncode, 125, result.stderr)
        self.assertEqual(report["refusal"], "wall-time-exceeded")
        self.assertNotEqual(report["returncode"], 0, "sleeping child was not terminated")
        self.assertLess(report["elapsed_seconds"], 2)

    def test_huge_finite_intervals_allow_successful_child(self):
        result, report, log = self.run_child("import sys; sys.stdout.buffer.write(b'short child\\n')", timeout=1e308, poll="1e308")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIsNone(report["refusal"])
        self.assertEqual(log, b"short child\n")
        self.assertLess(report["elapsed_seconds"], 3)

    def test_poll_interval_respects_remaining_deadline(self):
        result, report, _ = self.run_child("import time; time.sleep(5)", timeout=0.1, poll="10")
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
        self.assertNotEqual(report["returncode"], 0, "sleeping child was not terminated")
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
        self.assertEqual([error["phase"] for error in report["disk_accounting_errors"]], ["monitor", "final"])
        self.assertNotEqual(report["returncode"], 0, "sleeping child was not terminated")

    @unittest.skipIf(os.name == "nt" or getattr(os, "geteuid", lambda: 1)() == 0,
                     "requires ordinary POSIX directory access permissions")
    def test_actual_inaccessible_directory_causes_disk_refusal(self):
        result, report, _ = self.run_child(
            "import pathlib,sys,time; denied=pathlib.Path(sys.argv[1])/'denied'; "
            "denied.mkdir(); denied.chmod(0); time.sleep(5)")
        self.assertEqual(result.returncode, 125, result.stderr)
        self.assertTrue(report["refusal"].startswith("disk-accounting-failed:"), report)
        self.assertEqual([error["phase"] for error in report["disk_accounting_errors"]], ["monitor", "final"])
        self.assertNotEqual(report["returncode"], 0, "sleeping child was not terminated")

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
        result, report, log = self.final_sample_failure("import sys; sys.stdout.buffer.write(b'retained child evidence\\n')")
        self.assertEqual(result.returncode, 125, result.stderr)
        self.assertEqual(report["returncode"], 0)
        self.assertIn("disk-accounting-failed: final sample probe", report["refusal"])
        self.assertEqual(log, b"retained child evidence\n")
        self.assertEqual(report["disk_accounting_errors"], [{"phase": "final", "error": "final sample probe"}])

    def test_final_sample_failure_preserves_original_timeout_refusal(self):
        result, report, _ = self.final_sample_failure("import time; time.sleep(5)", timeout=0.1)
        self.assertEqual(result.returncode, 125, result.stderr)
        self.assertEqual(report["refusal"], "wall-time-exceeded")
        self.assertEqual(report["disk_accounting_errors"], [{"phase": "final", "error": "final sample probe"}])

    def test_success_records_actual_exit_and_output(self):
        result, report, log = self.run_child("import sys; sys.stdout.buffer.write(b'native-child\\n')")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIsNone(report["refusal"])
        self.assertEqual(log, b"native-child\n")
        self.assertGreater(report["peak_sampled_disk_bytes"], 0)

    def test_child_crlf_bytes_are_preserved_without_normalization(self):
        result, report, log = self.run_child("import sys; sys.stdout.buffer.write(b'line\\r\\n')")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIsNone(report["refusal"])
        self.assertEqual(log, b"line\r\n")

    def test_disk_writer_is_refused(self):
        result, report, _ = self.run_child(
            "import pathlib,sys,time; (pathlib.Path(sys.argv[1])/'large').write_bytes(b'x'*(2*1024*1024)); time.sleep(5)")
        self.assertEqual(result.returncode, 125, result.stderr)
        self.assertEqual(report["refusal"], "disk-budget-exceeded")
        self.assertGreater(report["peak_sampled_disk_bytes"], report["disk_limit_bytes"])

    def test_time_limit_terminates_child(self):
        result, report, _ = self.run_child("import time; time.sleep(5)", timeout=0.1)
        self.assertEqual(result.returncode, 125, result.stderr)
        self.assertEqual(report["refusal"], "wall-time-exceeded")
        self.assertLess(report["elapsed_seconds"], 2)

    def test_log_limit_retains_only_bounded_output(self):
        result, report, log = self.run_child("import sys; sys.stdout.buffer.write(b'x'*(2*1024*1024))", disk_limit=4)
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
        result, report, _ = self.run_child("import time; time.sleep(5)", timeout=0.1, fault=fault)
        self.assertEqual(result.returncode, 125, result.stderr)
        self.assertEqual(report["refusal"], "wall-time-exceeded")
        self.assertEqual(report["cleanup_errors"], ["PermissionError: group cleanup probe"])
        self.assertIsNotNone(report["returncode"])
        self.assertNotEqual(report["returncode"], 0)

    @unittest.skipIf(os.name == "nt", "POSIX cooperative interrupt grace")
    def test_forced_stop_cannot_claim_separate_descendant_cleanup(self):
        result, report, _ = self.run_child(
            "import signal,time; signal.signal(signal.SIGINT, signal.SIG_IGN); time.sleep(5)", timeout=0.2)
        self.assertEqual(result.returncode, 125, result.stderr)
        self.assertEqual(report["refusal"], "wall-time-exceeded")
        self.assertIn("escaped descendant cleanup is unconfirmed", report["cleanup_errors"][0])
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
        result, report, _ = self.run_child("import time; time.sleep(5)", timeout=0.1, fault=fault)
        self.assertEqual(result.returncode, 125, result.stderr)
        self.assertEqual(report["refusal"], "wall-time-exceeded")
        self.assertEqual(report["reader_errors"], ["late reader probe"])

    @unittest.skipUnless(sys.platform == "darwin", "Darwin exited-group EPERM inspection")
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

    def separately_grouped_child_cleanup(self, overflow=False):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            marker = root / "separate-child-wrote"
            ready = root / "ready"
            child = "import pathlib,time; time.sleep(1.5); pathlib.Path(" + repr(str(marker)) + ").write_text('leaked')"
            parent = (
                "import os,signal,subprocess,sys,time,pathlib\n"
                "child=subprocess.Popen([sys.executable,'-c'," + repr(child) + "],start_new_session=True)\n"
                "def stop(*_):\n"
                + (" sys.stdout.write('c'*(2*1024*1024)); sys.stdout.flush()\n" if overflow else "") +
                " os.killpg(child.pid,signal.SIGKILL)\n"
                " child.wait()\n"
                " sys.exit(0)\n"
                "signal.signal(signal.SIGINT,stop)\n"
                "pathlib.Path(" + repr(str(ready)) + ").write_text('ready')\n"
                + ("sys.stdout.write('x'*(2*1024*1024)); sys.stdout.flush()\n" if overflow else "") +
                "time.sleep(5)\n"
            )
            result = subprocess.run([sys.executable, str(RUNNER), "--root", directory,
                "--disk-limit-mib", "4", "--log-limit-mib", "1", "--timeout", "0.5", "--poll-seconds", "0.02",
                "--label", "test", "--", sys.executable, "-c", parent],
                capture_output=True, timeout=8)
            self.assertTrue(ready.exists(), result.stderr)
            self.assertEqual(result.returncode, 125, result.stderr)
            report = json.loads((root / "test.resources.json").read_text())
            self.assertEqual(report["refusal"], "log-budget-exceeded" if overflow else "wall-time-exceeded")
            self.assertEqual(report["cleanup_errors"], [])
            time.sleep(1.5)
            self.assertFalse(marker.exists(), "separately grouped child survived cancellation")

    @unittest.skipIf(os.name == "nt", "POSIX process-group orphan cleanup")
    def test_successful_parent_cannot_leave_background_writer(self):
        with tempfile.TemporaryDirectory() as directory:
            marker = Path(directory) / "orphan-wrote"
            child = "import pathlib,time; time.sleep(0.4); pathlib.Path(" + repr(str(marker)) + ").write_text('leaked')"
            parent = "import subprocess,sys; subprocess.Popen([sys.executable, '-c', " + repr(child) + "])"
            result = subprocess.run([sys.executable, str(RUNNER), "--root", directory,
                "--disk-limit-mib", "1", "--timeout", "2", "--poll-seconds", "0.02", "--label", "test", "--",
                sys.executable, "-c", parent], capture_output=True, timeout=5)
            self.assertEqual(result.returncode, 0, result.stderr)
            time.sleep(0.5)
            self.assertFalse(marker.exists(), "background writer survived command completion")

    def test_existing_report_is_never_overwritten(self):
        with tempfile.TemporaryDirectory() as directory:
            report = Path(directory) / "test.resources.json"
            report.write_text("immutable")
            result = subprocess.run([sys.executable, str(RUNNER), "--root", directory,
                "--disk-limit-mib", "1", "--timeout", "1", "--label", "test", "--",
                sys.executable, "-c", "raise Exception('must not run')"], capture_output=True)
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
        for code, timeout, expected in [("print('completed')", 2, 0),
                                         ("import time; time.sleep(5)", .1, 125)]:
            with self.subTest(timeout=timeout):
                result, report, _ = self.run_child(code, timeout=timeout, fault=fault)
                self.assertEqual(result.returncode, expected, result.stderr)
                self.assertEqual(report['cleanup_errors'], [], report)

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
        self.assertIsNone(report['returncode'], report)
        self.assertIn('externally reaped ownership probe', report['refusal'])
        self.assertTrue(report['cleanup_errors'])
        self.assertEqual(log, b'completed\n')

    @unittest.skipIf(os.name == "nt", "POSIX ownership prerequisites")
    def test_unavailable_waitid_and_nondefault_sigchld_refuse_before_launch(self):
        for fault in ["del namespace['main'].__globals__['os'].WNOWAIT",
                      "namespace['main'].__globals__['signal'].signal(__import__('signal').SIGCHLD, __import__('signal').SIG_IGN)",
                      "namespace['main'].__globals__['signal'].signal(__import__('signal').SIGCHLD, lambda *_: None)"]:
            with self.subTest(fault=fault), tempfile.TemporaryDirectory() as directory:
                wrapper = "import runpy,sys\nnamespace=runpy.run_path(sys.argv[1])\nsys.argv=sys.argv[1:]\n" + fault + "\nsys.exit(namespace['main']())\n"
                result = subprocess.run([sys.executable, '-c', wrapper, str(RUNNER), '--root', directory,
                    '--disk-limit-mib', '1', '--timeout', '1', '--label', 'test', '--',
                    sys.executable, '-c', "raise AssertionError('must not launch')"], capture_output=True, text=True, timeout=3)
                self.assertEqual(result.returncode, 2, result.stderr)
                self.assertTrue('waitid/WNOWAIT' in result.stderr or 'SIGCHLD' in result.stderr)
                self.assertFalse(list(Path(directory).iterdir()))

    @unittest.skipIf(os.name == "nt", "POSIX shell prerequisite integration")
    def test_shell_entrypoints_refuse_missing_ownership_capability_before_cargo(self):
        for entrypoint in ["check.sh", "scripts/focused-smoke.sh"]:
            with self.subTest(entrypoint=entrypoint), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                probe = root / "python-probe.py"
                probe.write_text("import os,runpy,sys\nif sys.argv[1:2] == ['-c']:\n exec(sys.argv[2])\nelse:\n del os.WNOWAIT\n if sys.argv[1:2] == ['-']:\n  exec(sys.stdin.read())\n else:\n  sys.argv=sys.argv[1:]\n  runpy.run_path(sys.argv[0],run_name='__main__')\n")
                python = root / "python3"
                python.write_text("#!/bin/sh\nexec " + shlex.quote(sys.executable) + " " + shlex.quote(str(probe)) + ' "$@"\n')
                marker = root / "cargo-started"
                for tool in [python, root / "cargo", root / "cargo-deny"]:
                    if tool != python:
                        tool.write_text("#!/bin/sh\necho launched > " + shlex.quote(str(marker)) + "\nexit 99\n")
                    tool.chmod(0o755)
                result = subprocess.run(["bash", str(ROOT / entrypoint)], env={**os.environ, "PATH": str(root) + os.pathsep + os.environ.get("PATH", "")}, capture_output=True, text=True, timeout=3)
                self.assertEqual(result.returncode, 2, result.stderr)
                self.assertIn("waitid/WNOWAIT", result.stderr)
                self.assertFalse(marker.exists(), "Cargo started before ownership prerequisite refusal")
