# SPDX-License-Identifier: MPL-2.0
"""Release admission requires current same-source main verification, never stale successes."""
import copy
import importlib.util
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location("release_verification", Path(__file__).resolve().parents[1] / "scripts/check-release-verification.py")
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class ReleaseVerificationTest(unittest.TestCase):
    def test_only_latest_matching_main_run_can_admit_source(self):
        source = "a" * 40
        run = dict(id=1, run_number=1, run_attempt=1, head_sha=source, head_branch="main",
            event="push", path=".github/workflows/ci.yml", status="completed", conclusion="success")
        def check(runs): return module.require_success(dict(workflow_runs=runs), source, run["path"], {"push"})
        self.assertEqual(check([run]), 1)
        for field, value in [("head_sha", "b" * 40), ("head_branch", "feature"), ("event", "pull_request"), ("path", "other.yml"), ("status", "in_progress"), ("conclusion", "failure")]:
            bad = copy.deepcopy(run); bad[field] = value
            with self.assertRaises(ValueError): check([bad])
        newer = copy.deepcopy(run); newer.update(id=2, run_number=2, conclusion="failure")
        with self.assertRaises(ValueError): check([run, newer])
        rerun = copy.deepcopy(run); rerun.update(run_attempt=2, conclusion="cancelled")
        with self.assertRaises(ValueError): check([run, rerun])
