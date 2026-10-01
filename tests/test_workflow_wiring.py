"""Workflow handoff must preserve source, inventory and credential boundaries."""
from pathlib import Path
import re
import unittest

ROOT = Path(__file__).resolve().parents[1]


class WorkflowWiringTest(unittest.TestCase):
    def test_every_checkout_discards_credentials_and_mutations_use_planner_source(self):
        for path in (ROOT / ".github/workflows").glob("*.yml"):
            text = path.read_text()
            for step in re.split(r"\n      - ", text):
                if "uses: actions/checkout@" in step:
                    self.assertIn("persist-credentials: false", step)
                    self.assertIn("ref:", step)
        mutations = (ROOT / ".github/workflows/mutants.yml").read_text()
        for job, planner in [("mutants", "mutation-plan"), ("mutants-diff", "mutation-diff-plan"), ("mutation-summary", "mutation-plan"), ("mutation-diff-summary", "mutation-diff-plan")]:
            body = re.split(r"\n(?=  [\w-]+:\n)", mutations.split(f"\n  {job}:\n", 1)[1])[0]
            self.assertIn(f"needs.{planner}.outputs.source_sha", body)
        self.assertEqual(mutations.count("name: Preserve selected inventory"), 2)
        self.assertEqual(mutations.count("python3 scripts/verify-mutation-results.py \\"), 2)
        self.assertEqual(mutations.count('git rev-parse HEAD > "$stage_dir/mutants.out/source-commit.txt"'), 2)

    def test_release_resolves_once_without_shell_interpolating_dispatch_input(self):
        text = (ROOT / ".github/workflows/release.yml").read_text()
        self.assertIn("cancel-in-progress: false", text)
        self.assertLess(text.index("name: Validate release tag before checkout"), text.index("uses: actions/checkout@"))
        self.assertIn('git cat-file -t "refs/tags/$TAG"', text)
        self.assertEqual(text.count("RELEASE_SOURCE_SHA: ${{ needs.release-target-matrix.outputs.source_sha }}"), 3)
        for line in text.splitlines():
            if "${{ inputs.release_tag" in line:
                self.assertIn("CANDIDATE_TAG:", line)

    def test_cache_paths_and_environment_trigger_cover_bootstrap_changes(self):
        ci = (ROOT / ".github/workflows/ci.yml").read_text()
        self.assertIn("'scripts/contributor-rust-tools.sh'", ci)
        self.assertIn("'rust-toolchain.toml'", ci)
        for filename in ["ci.yml", "release.yml", "mutants.yml"]:
            text = (ROOT / ".github/workflows" / filename).read_text()
            for step in re.split(r"\n      - ", text):
                if "uses: Swatinem/rust-cache@" in step:
                    self.assertIn("../.htmlcut-artifacts/target", step)
                    self.assertIn("cache-directories: ../.htmlcut-artifacts/build", step)
                    self.assertIn("save-if:", step)
