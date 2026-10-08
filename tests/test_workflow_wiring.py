# SPDX-License-Identifier: MPL-2.0
"""Workflow handoff must preserve source, inventory and credential boundaries."""
from pathlib import Path
import re
import json
import os
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]


class WorkflowWiringTest(unittest.TestCase):
    def test_ci_exports_complete_matrix_as_one_action_output(self):
        text = (ROOT / ".github/workflows/ci.yml").read_text()
        step = text.split("      - id: matrix\n", 1)[1].split("\n  linux-maintainer:", 1)[0]
        body = step.split("        run: |\n", 1)[1]
        command = "\n".join(line[10:] for line in body.splitlines())
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "output"
            subprocess.run(["bash", "-c", command], cwd=ROOT,
                           env={**os.environ, "GITHUB_OUTPUT": str(output)}, check=True)
            lines = output.read_text().splitlines()
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "scripts").mkdir()
            producer = root / "scripts/release-targets.sh"
            producer.write_text("#!/usr/bin/env bash\nexit 23\n")
            producer.chmod(0o755)
            output = root / "output"
            failed = subprocess.run(["bash", "-c", command], cwd=root,
                                    env={**os.environ, "GITHUB_OUTPUT": str(output)})
            self.assertEqual(failed.returncode, 23)
            self.assertNotIn("EOF\n", output.read_text().split("\n", 1)[1])
        self.assertEqual(lines[0], "matrix<<EOF")
        self.assertEqual(lines[-1], "EOF")
        matrix = json.loads("\n".join(lines[1:-1]))
        self.assertEqual(len(matrix["include"]), 4)
        triples = subprocess.check_output([str(ROOT / "scripts/release-targets.sh"), "triples"], text=True).splitlines()
        self.assertEqual([row["target_triple"] for row in matrix["include"]], triples)

    def test_required_smoke_failure_propagates_and_retains_log(self):
        text = (ROOT / ".github/workflows/ci.yml").read_text()
        job = text.split("\n  focused-smoke:\n", 1)[1].split("\n  semver:\n", 1)[0]
        command = job.split("      - name: Execute both bounded smokes\n", 1)[1].split("      - name:", 1)[0].split("        run: ", 1)[1].strip()
        self.assertIn('if: always()', job)
        self.assertIn('path: ${{ runner.temp }}/focused-smoke/', job)
        aggregate = text.split("\n  check:\n", 1)[1]
        self.assertIn('focused-smoke,', aggregate)
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            bin_dir = root / 'bin'
            bin_dir.mkdir()
            cargo = bin_dir / 'cargo'
            cargo.write_text('#!/usr/bin/env bash\nprintf "intentional smoke failure\\n" >&2\nexit 23\n')
            cargo.chmod(0o755)
            result = subprocess.run(['bash', '-c', command], cwd=ROOT,
                                    env={**os.environ, 'PATH': str(bin_dir) + os.pathsep + os.environ['PATH'],
                                         'RUNNER_TEMP': str(root)}, capture_output=True, text=True)
            self.assertEqual(result.returncode, 23)
            evidence = root / 'focused-smoke/parse_document_bytes'
            self.assertIn('intentional smoke failure', (evidence / 'run.log').read_text())
            self.assertTrue(any((evidence / 'corpus').iterdir()))
            self.assertTrue((evidence / 'crashes').is_dir())

    def test_every_checkout_discards_credentials(self):
        for path in (ROOT / ".github/workflows").glob("*.yml"):
            for step in re.split(r"\n      - ", path.read_text()):
                if "uses: actions/checkout@" in step:
                    self.assertIn("persist-credentials: false", step)
                    self.assertIn("ref:", step)

    def test_release_resolves_once_without_shell_interpolating_dispatch_input(self):
        text = (ROOT / ".github/workflows/release.yml").read_text()
        self.assertIn("cancel-in-progress: false", text)
        self.assertLess(text.index("name: Validate release tag before checkout"), text.index("uses: actions/checkout@"))
        self.assertIn('git cat-file -t "refs/tags/$TAG"', text)
        self.assertEqual(text.count("RELEASE_SOURCE_SHA: ${{ needs.release-target-matrix.outputs.source_sha }}"), 4)
        for line in text.splitlines():
            if "${{ inputs.release_tag" in line:
                self.assertIn("CANDIDATE_TAG:", line)

    def test_published_native_execution_waits_for_publication_and_uses_downloaded_bytes(self):
        text = (ROOT / ".github/workflows/release.yml").read_text()
        body = text.split("\n  published-native:\n", 1)[1]
        self.assertIn("needs: [release-target-matrix, release]", body)
        self.assertIn("matrix: ${{ fromJson(needs.release-target-matrix.outputs.matrix) }}", body)
        self.assertIn("--source-digest \"$RELEASE_SOURCE_SHA\"", body)
        self.assertIn("curl --fail --location", body)
        self.assertIn("scripts/native-package-evidence.py", body)
        self.assertNotIn("scripts/build-release-artifact.sh", body)
        self.assertIn("dist/published-*.json", body)
        self.assertIn("dist/native-evidence-${{ matrix.id }}*.json", text)
