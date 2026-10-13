# SPDX-License-Identifier: MPL-2.0
"""Prerequisite refusals happen before the expensive maintainer checks."""
import os
from pathlib import Path
import shlex
import subprocess
import sys
import tempfile
import time
import unittest

ROOT = Path(__file__).resolve().parents[1]


class CheckPrerequisitesTest(unittest.TestCase):
    def refused(self, old_python):
        with tempfile.TemporaryDirectory() as directory:
            bin_dir = Path(directory)
            log = bin_dir / "cargo-called"
            # An isolated path proves that no system cargo-deny can fill the gap.
            (bin_dir / "dirname").symlink_to("/usr/bin/dirname")
            python = bin_dir / "python3"
            python.write_text("#!/bin/bash\n" + ("exit 1\n" if old_python else
                "exec " + shlex.quote(sys.executable) + ' "$@"\n'))
            python.chmod(0o755)
            cargo = bin_dir / "cargo"
            cargo.write_text('#!/bin/bash\nprintf called > "$CARGO_CALL_LOG"\nexit 99\n')
            cargo.chmod(0o755)
            if old_python:
                deny = bin_dir / "cargo-deny"
                deny.write_text("#!/bin/bash\nexit 0\n")
                deny.chmod(0o755)
            started = time.monotonic()
            result = subprocess.run(["/bin/bash", str(ROOT / "check.sh")],
                env={**os.environ, "PATH": str(bin_dir), "CARGO_CALL_LOG": str(log)},
                capture_output=True, text=True, timeout=1)
            self.assertLess(time.monotonic() - started, 1)
            self.assertEqual(result.returncode, 2, result)
            self.assertEqual(result.stdout, "")
            self.assertEqual(len(result.stderr.splitlines()), 1, result.stderr)
            self.assertFalse(log.exists(), "Cargo ran before prerequisite refusal")
            return result.stderr

    def test_old_python_stops_before_cargo(self):
        self.assertIn("Python 3.11 or newer", self.refused(old_python=True))

    def test_missing_cargo_deny_stops_before_cargo(self):
        self.assertIn("cargo-deny", self.refused(old_python=False))
