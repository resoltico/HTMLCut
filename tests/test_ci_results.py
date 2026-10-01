"""CI summary policy rejects every mandatory non-success and only the declared optional skip."""
import copy
import importlib.util
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location('ci_policy', Path(__file__).resolve().parents[1] / 'scripts/check-ci-results.py')
policy = importlib.util.module_from_spec(spec)
spec.loader.exec_module(policy)

class PolicyTest(unittest.TestCase):
    def base(self):
        jobs = {name: {'result': 'success'} for name in policy.REQUIRED}
        jobs['devcontainer-changes']['outputs'] = {'changed': 'false'}
        jobs['contributor-devcontainer'] = {'result': 'skipped'}
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

    def test_changed_environment_must_be_validated(self):
        jobs = self.base()
        jobs['devcontainer-changes']['outputs']['changed'] = 'true'
        self.assertIn('contributor-devcontainer', policy.failures(jobs))
        jobs['contributor-devcontainer']['result'] = 'success'
        self.assertEqual(policy.failures(jobs), [])
        jobs['devcontainer-changes']['outputs'].clear()
        self.assertIn('devcontainer-change-output', policy.failures(jobs))

if __name__ == '__main__':
    unittest.main()

class WorkflowTest(unittest.TestCase):
    def test_aggregate_runner_checks_out_its_validator_before_execution(self):
        workflow = (Path(__file__).resolve().parents[1] / '.github/workflows/ci.yml').read_text()
        aggregate = workflow.split('\n  check:\n', 1)[1]
        checkout = aggregate.index('uses: actions/checkout@')
        execute = aggregate.index('run: python3 scripts/check-ci-results.py')
        self.assertLess(checkout, execute)
        self.assertIn('ref: ${{ github.event.pull_request.head.sha || github.sha }}', aggregate)
        self.assertIn('JOB_RESULTS: ${{ toJSON(needs) }}', aggregate)
