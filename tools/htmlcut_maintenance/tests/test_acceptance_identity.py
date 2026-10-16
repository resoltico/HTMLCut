# SPDX-License-Identifier: MPL-2.0
"""Real source/archive changes must reject acceptance despite unchanged binary bytes."""

import json
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest
from unittest.mock import patch
from tools.htmlcut_maintenance import config
from tools.htmlcut_maintenance.release import accept, package


class AcceptanceIdentityTests(unittest.TestCase):
    def fixture(self, root):
        repo = root / "source"
        repo.mkdir()
        (repo / "Cargo.toml").write_text('[workspace.package]\nversion="23.0.0"\n')
        (repo / ".gitignore").write_text("/dist/\n")
        (repo / "tracked.txt").write_text("original source")
        for name in ("LICENSE", "PATENTS.md"):
            shutil.copy2(config.ROOT / name, repo / name)
        for kind in ("contract", "outline"):
            (repo / "conformance" / kind / "one").mkdir(parents=True)

        def git(*args):
            return subprocess.check_output(
                ["git", "-C", str(repo), *args], stderr=subprocess.DEVNULL, text=True
            ).strip()

        git("init", "-b", "main")
        git("config", "user.name", "Acceptance fixture")
        git("config", "user.email", "fixture@example.invalid")
        git("add", ".")
        git("commit", "-m", "source fixture")
        sha = git("rev-parse", "HEAD")
        row = next(
            row
            for row in config.targets()
            if row["triple"] == "x86_64-unknown-linux-musl"
        )
        directory = root / ("htmlcut-23.0.0-" + row["triple"])
        directory.mkdir()
        binary = directory / "htmlcut"
        binary.write_bytes(b"unchanged production byte fixture")
        binary.chmod(0o755)
        for name in ("LICENSE", "PATENTS.md"):
            shutil.copy2(repo / name, directory / name)
        (directory / "README.md").write_text(package.guide("23.0.0", row, sha))
        (directory / "NOTICE").write_text(
            "Native dependency attribution: HTMLCut 23.0.0, "
            + row["triple"]
            + "\n/tree/"
            + sha
            + "\nhtmlcut-core 23.0.0\nhtmlcut-runtime 23.0.0\n"
        )
        archive = root / "native.tar.gz"
        package.pack(directory, archive, "tar.gz")
        return repo, row, archive, git

    def exercise(self, change):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            repo, row, archive, git = self.fixture(root)
            calls = []
            if change == "dirty-start":
                (repo / "tracked.txt").write_text("dirty before admission")

            def phase(command, work, **options):
                calls.append(options["label"])
                label = options["label"]
                work = Path(work)
                if label == "guide-0":
                    if change == "archive":
                        with archive.open("ab") as stream:
                            stream.write(b"changed archive")
                    if change in ("source", "committed-source"):
                        (repo / "tracked.txt").write_text("changed source")
                        if change == "committed-source":
                            git("add", "tracked.txt")
                            git("commit", "-m", "move source")
                data = {
                    "guide-0": b"htmlcut 23.0.0\n",
                    "guide-1": b"../guide.html\n",
                    "native-tests": b"HTMLCUT_CONFORMANCE_ID=contract/one\nHTMLCUT_CONFORMANCE_ID=outline/one\nHTMLCUT_CONFORMANCE_CASES=2\n",
                    "acquisition": b"passed\n",
                }[label]
                (work / (label + ".log")).write_bytes(data)
                report = dict(
                    command=command, returncode=0, refusal=None, cleanup_errors=[]
                )
                (work / (label + ".resources.json")).write_text(json.dumps(report))
                return report

            with patch.object(accept.process, "run", side_effect=phase):
                if change is None:
                    accept.accept(archive, row["triple"], "23.0.0", root=repo)
                else:
                    with self.assertRaisesRegex(
                        ValueError,
                        "clean source"
                        if change == "dirty-start"
                        else "archive bytes changed"
                        if change == "archive"
                        else "source changed",
                    ):
                        accept.accept(archive, row["triple"], "23.0.0", root=repo)
            (proof,) = repo.glob("dist/native-acceptance-*/acceptance.json")
            record = json.loads(proof.read_text())
            self.assertEqual(record["status"], "pass" if change is None else "fail")
            if change == "dirty-start":
                self.assertEqual(calls, [])
            else:
                self.assertEqual(
                    calls, ["guide-0", "guide-1", "native-tests", "acquisition"]
                )
            if record["status"] == "fail":
                shutil.rmtree(
                    record["work"]
                )  # no real subprocess was launched by this fixture

    def test_unchanged_identity_can_pass(self):
        self.exercise(None)

    def test_archive_changes_cannot_claim_acceptance(self):
        self.exercise("archive")

    def test_source_content_changes_cannot_claim_acceptance(self):
        self.exercise("source")

    def test_clean_but_changed_head_cannot_claim_acceptance(self):
        self.exercise("committed-source")

    def test_dirty_source_refuses_before_executable_phases(self):
        self.exercise("dirty-start")


class CorpusEvidenceTests(unittest.TestCase):
    def test_each_kind_and_actual_case_identity_are_required(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            for kind in ("contract", "outline"):
                (root / "conformance" / kind / "one").mkdir(parents=True)
            valid = b"HTMLCUT_CONFORMANCE_ID=contract/one\nHTMLCUT_CONFORMANCE_ID=outline/one\nHTMLCUT_CONFORMANCE_CASES=2\n"
            self.assertEqual(accept.conformance_evidence(root, valid), 2)
            serial = b"test every_case_runs_through_the_delivered_binary ... " + valid
            self.assertEqual(accept.conformance_evidence(root, serial), 2)
            contract, outline, count = valid.splitlines(keepends=True)
            for malformed in [outline + contract + count, contract + contract + outline + count, valid + outline, b"arbitrary prefix " + valid]:
                with self.assertRaisesRegex(ValueError, "missing or incomplete"):
                    accept.conformance_evidence(root, malformed)
            for log in [b"HTMLCUT_CONFORMANCE_CASES=2\n", valid.replace(b"outline/one", b"outline/wrong"), valid.replace(b"outline/one", b"contract/one")]:
                with self.assertRaisesRegex(ValueError, "missing or incomplete"):
                    accept.conformance_evidence(root, log)
            (root / "conformance/outline/one").rmdir()
            with self.assertRaisesRegex(ValueError, "outline corpus discovery is empty"):
                accept.conformance_evidence(root, valid)
