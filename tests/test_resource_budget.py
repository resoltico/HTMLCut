# SPDX-License-Identifier: MPL-2.0
"""Real child writers prove QA resource refusals and retained evidence."""
import json
from pathlib import Path
import subprocess
import os
import time
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
RUNNER = ROOT / "scripts/run-resource-bounded.py"


class ResourceBudgetTests(unittest.TestCase):
    def run_child(self, code, timeout=2, disk_limit=1):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            marker = root / "owner-evidence"
            marker.write_text("preserve")
            result = subprocess.run([sys.executable, str(RUNNER), "--root", directory,
                "--disk-limit-mib", str(disk_limit), "--log-limit-mib", "1", "--timeout", str(timeout),
                "--poll-seconds", "0.02", "--label", "test", "--", sys.executable, "-c", code,
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

    def test_success_records_actual_exit_and_output(self):
        result, report, log = self.run_child("print('native-child')")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIsNone(report["refusal"])
        self.assertEqual(log, b"native-child\n")
        self.assertGreater(report["peak_sampled_disk_bytes"], 0)

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
