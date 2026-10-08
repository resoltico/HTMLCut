# SPDX-License-Identifier: MPL-2.0
"""CI summary policy rejects every mandatory non-success."""
import importlib.util
import json
import os
import subprocess
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location('ci_policy', Path(__file__).resolve().parents[1] / 'scripts/check-ci-results.py')
policy = importlib.util.module_from_spec(spec)
spec.loader.exec_module(policy)

class PolicyTest(unittest.TestCase):
    def base(self):
        jobs = {name: {'result': 'success'} for name in policy.REQUIRED}
        return jobs

    def test_core_only_change_requires_full_maintainer_success(self):
        self.assertEqual(policy.failures(self.base()), [])
        for name in policy.REQUIRED:
            for state in ['failure', 'cancelled', 'skipped', None]:
                with self.subTest(name=name, state=state):
                    jobs = self.base()
                    jobs[name]['result'] = state
                    self.assertIn(name, policy.failures(jobs))
            jobs = self.base()
            del jobs[name]
            self.assertIn(name, policy.failures(jobs))

    def test_skipped_smoke_is_rejected_by_real_aggregate_command(self):
        jobs = self.base()
        jobs['focused-smoke']['result'] = 'skipped'
        result = subprocess.run(
            ['python3', str(Path(__file__).resolve().parents[1] / 'scripts/check-ci-results.py')],
            env={**os.environ, 'JOB_RESULTS': json.dumps(jobs)}, capture_output=True, text=True)
        self.assertEqual(result.returncode, 1)
        self.assertIn('focused-smoke', result.stderr)

class WorkflowTest(unittest.TestCase):
    def test_aggregate_runner_checks_out_its_validator_before_execution(self):
        workflow = (Path(__file__).resolve().parents[1] / '.github/workflows/ci.yml').read_text()
        aggregate = workflow.split('\n  check:\n', 1)[1]
        checkout = aggregate.index('uses: actions/checkout@')
        execute = aggregate.index('run: python3 scripts/check-ci-results.py')
        self.assertLess(checkout, execute)
        self.assertIn('ref: ${{ github.event.pull_request.head.sha || github.sha }}', aggregate)
        self.assertIn('JOB_RESULTS: ${{ toJSON(needs) }}', aggregate)


if __name__ == '__main__':
    unittest.main()
