# SPDX-License-Identifier: MPL-2.0
"""Default scratch is disposable on success; failure and caller output are retained."""
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
ROOT = Path(__file__).resolve().parents[1]


class FuzzCleanupTest(unittest.TestCase):
    def exercise(self, failure, explicit):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            scratch = root / "scratch"
            scratch.mkdir()
            tools = root / "bin"
            tools.mkdir()
            cargo = tools / "cargo"
            cargo.write_text("#!/usr/bin/env bash\n" + ("exit 42\n" if failure else
                "printf '#10000\\nDone 10000 runs\\n'\n"))
            cargo.chmod(0o755)
            destination = root / "caller-output"
            args = [str(ROOT / "scripts/focused-smoke.sh")]
            if explicit:
                destination.mkdir()
                (destination / "caller-marker").write_text("preserve")
                args.append(str(destination))
            result = subprocess.run(args, cwd=ROOT,
                env={**os.environ, "PATH": str(tools) + os.pathsep + os.environ["PATH"],
                     "TMPDIR": str(scratch)}, capture_output=True, text=True, timeout=10)
            self.assertEqual(result.returncode, 42 if failure else 0, result.stderr)
            if explicit:
                self.assertEqual((destination / "caller-marker").read_text(), "preserve")
            else:
                remaining = list(scratch.iterdir())
                if failure:
                    self.assertEqual(len(remaining), 1)
                    self.assertIn(str(remaining[0]), result.stderr)
                    self.assertTrue((remaining[0] / "contract_bytes/corpus").is_dir())
                else:
                    self.assertEqual(remaining, [])

    def test_default_success_removes_output(self):
        self.exercise(failure=False, explicit=False)

    def test_default_failure_retains_output_and_prints_path(self):
        self.exercise(failure=True, explicit=False)

    def test_explicit_output_survives_success_and_failure(self):
        self.exercise(failure=False, explicit=True)
        self.exercise(failure=True, explicit=True)
