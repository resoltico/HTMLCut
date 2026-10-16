# SPDX-License-Identifier: MPL-2.0
"""Mutation evidence, source binding and disposable ownership controls."""

import copy
import importlib.util
import json
import os
from pathlib import Path
import signal
import shutil
import subprocess
import sys
import tempfile
import tomllib
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[3]
spec = importlib.util.spec_from_file_location(
    "mutation_audit", ROOT / "tools/htmlcut_maintenance/mutation.py"
)
audit = importlib.util.module_from_spec(spec)
spec.loader.exec_module(audit)
VERSION = "27.1.0"


def phases(verdict):
    def phase(name, status):
        return {
            "phase": name,
            "process_status": status,
            "duration": 0.01,
            "argv": ["cargo", "test"],
        }

    if verdict == "Unviable":
        return [phase("Build", {"Failure": 101})]
    if verdict == "Timeout":
        return [phase("Build", "Success"), phase("Test", "Timeout")]
    return [
        phase("Build", "Success"),
        phase("Test", {"Failure": 101} if verdict == "CaughtMutant" else "Success"),
    ]


def evidence(output, verdicts=("CaughtMutant", "CaughtMutant")):
    root = output / "mutants.out"
    (root / "log").mkdir(parents=True)
    (root / "diff").mkdir()
    (root / "log/baseline.log").write_text("baseline evidence")
    outcomes = [
        {
            "scenario": "Baseline",
            "summary": "Success",
            "log_path": "log/baseline.log",
            "diff_path": None,
            "phase_results": phases("Success"),
        }
    ]
    plan = []
    counts = {"caught": 0, "missed": 0, "timeout": 0, "unviable": 0, "success": 0}
    for index, verdict in enumerate(verdicts):
        identity = {
            "name": f"src/lib.rs:1:{index}: replace flag with {index}",
            "file": "src/lib.rs",
            "package": "tiny",
            "replacement": str(index),
            "genre": "FnValue",
            "function": None,
            "span": {
                "start": {"line": 1, "column": index},
                "end": {"line": 1, "column": index + 1},
            },
        }
        plan.append({**identity, "diff": f"literal diff {index}"})
        (root / f"log/{index}.log").write_text("scenario evidence")
        (root / f"diff/{index}.diff").write_text(f"literal diff {index}")
        outcomes.append(
            {
                "scenario": {"Mutant": identity},
                "summary": verdict,
                "log_path": f"log/{index}.log",
                "diff_path": f"diff/{index}.diff",
                "phase_results": phases(verdict),
            }
        )
        counts[
            {
                "CaughtMutant": "caught",
                "MissedMutant": "missed",
                "Timeout": "timeout",
                "Unviable": "unviable",
            }[verdict]
        ] += 1
    result = {
        **counts,
        "outcomes": outcomes,
        "total_mutants": len(plan),
        "cargo_mutants_version": VERSION,
        "end_time": "2026-10-15T00:00:00Z",
    }
    (root / "mutants.json").write_text(json.dumps(plan))
    (root / "outcomes.json").write_text(json.dumps(result))
    return root, plan, result


def nextest_listing():
    return {"test-count": 1, "rust-suites": {"tiny": {
        "status": "listed", "package-name": "tiny", "binary-name": "tiny",
        "testcases": {"tests::detector": {"ignored": False, "filter-match": {"status": "matches"}}}
    }}}


def nextest_log(*, failed=False, label=None):
    identity = {"crate": "tiny", "test_binary": "tiny", "kind": "lib"}
    name = "tiny::tiny$tests::detector"
    terminal = {"type": "test", "event": "failed" if failed else "ok", "name": name, "exec_time": 0.01}
    if failed:
        terminal["stdout"] = "independent assertion failed"
    rows = [
        {"type": "suite", "event": "started", "test_count": 1, "nextest": identity},
        {"type": "test", "event": "started", "name": name},
        terminal,
        {"type": "suite", "event": "failed" if failed else "ok", "passed": int(not failed), "failed": int(failed), "ignored": 0, "measured": 0, "filtered_out": 0, "exec_time": 0.01, "nextest": identity},
    ]
    return "\n".join(map(json.dumps, rows)).replace('{"type": ', '{"type":') + "\n" + (" " + label + " [0.01s] tiny detector\n" if label else "") + " Summary [0.01s] 1 test run: complete\n"


class OutcomeTests(unittest.TestCase):
    def test_complete_survivors_and_timeouts_keep_their_counts(self):
        for verdicts, status in [
            (("CaughtMutant", "Unviable"), 0),
            (("CaughtMutant", "MissedMutant"), 2),
            (("MissedMutant", "Timeout"), 3),
        ]:
            with (
                self.subTest(status=status),
                tempfile.TemporaryDirectory() as directory,
            ):
                output = Path(directory)
                evidence(output, verdicts)
                result = audit.complete_outcomes(output, status, VERSION)
                self.assertEqual(result["tested_mutants"], 2)
                self.assertEqual(sum(result["counts"].values()), 2)
                self.assertEqual(result["counts"]["timeout"], verdicts.count("Timeout"))
                self.assertEqual(
                    result["counts"]["missed"], verdicts.count("MissedMutant")
                )

    def test_malformed_or_incomplete_evidence_is_rejected(self):
        mutations = {
            "duplicate outcome": lambda plan, result: result["outcomes"].__setitem__(
                2, copy.deepcopy(result["outcomes"][1])
            ),
            "missing outcome": lambda plan, result: result["outcomes"].pop(),
            "empty plan": lambda plan, result: plan.clear(),
            "duplicate plan": lambda plan, result: plan.__setitem__(
                1, copy.deepcopy(plan[0])
            ),
            "missing baseline": lambda plan, result: result["outcomes"].pop(0),
            "false baseline": lambda plan, result: result["outcomes"][0].__setitem__(
                "phase_results", phases("CaughtMutant")
            ),
            "false caught verdict": lambda plan, result: result["outcomes"][
                1
            ].__setitem__("phase_results", phases("Success")),
            "wrong counter": lambda plan, result: result.__setitem__("caught", 1),
            "wrong total": lambda plan, result: result.__setitem__("total_mutants", 1),
            "incomplete completion marker": lambda plan, result: result.__setitem__(
                "end_time", None
            ),
            "changed mutant": lambda plan, result: result["outcomes"][1]["scenario"][
                "Mutant"
            ].__setitem__("replacement", "unexpected"),
            "invalid verdict": lambda plan, result: result["outcomes"][1].__setitem__(
                "summary", "Success"
            ),
        }
        for name, mutate in mutations.items():
            with self.subTest(name=name), tempfile.TemporaryDirectory() as directory:
                output = Path(directory)
                root, plan, result = evidence(output)
                mutate(plan, result)
                (root / "mutants.json").write_text(json.dumps(plan))
                (root / "outcomes.json").write_text(json.dumps(result))
                with self.assertRaises(ValueError):
                    audit.complete_outcomes(output, 0, VERSION)

    def test_missing_log_or_diff_and_false_exit_are_rejected(self):
        for missing in ["log/baseline.log", "log/0.log", "diff/0.diff", None]:
            with (
                self.subTest(missing=missing),
                tempfile.TemporaryDirectory() as directory,
            ):
                output = Path(directory)
                root, _, _ = evidence(output)
                if missing:
                    (root / missing).unlink()
                with self.assertRaises(ValueError):
                    audit.complete_outcomes(
                        output, 2 if missing is None else 0, VERSION
                    )


class RunnerTests(unittest.TestCase):
    def run_main(
        self,
        directory,
        status=0,
        resources=True,
        cleanup_errors=None,
        source_changed=False,
        output_inside=False,
        extra=(),
        interrupted=False,
        empty_plan=False,
        probe_status=0,
        probe_version=VERSION,
        probe_resources=True,
        probe_cleanup_errors=None,
        probe_interrupted=False,
        test_tool="cargo",
        nextest_status_label=None,
    ):
        base = Path(directory)
        source = base / "workspace"
        source.mkdir()
        (source / "Cargo.toml").write_text("[workspace]\n")
        crate = source / "crate"
        crate.mkdir()
        (crate / "Cargo.toml").write_text('[package]\nname="tiny"\nversion="0.1.0"\n')
        output = source / "output" if output_inside else base / "results"
        before = {
            "git_commit": "commit",
            "sha256": "before",
            "files": {"src/lib.rs": {"sha256": "before"}},
        }
        after = (
            {
                "git_commit": "commit",
                "sha256": "after",
                "files": {"src/lib.rs": {"sha256": "after"}},
            }
            if source_changed
            else before
        )

        def tools(command, **kwargs):
            if command[:2] == ["cargo", "metadata"]:
                return subprocess.CompletedProcess(
                    command, 0, json.dumps({"workspace_root": str(source), "workspace_members": ["tiny-id"], "packages": [{"id": "tiny-id", "name": "tiny", "version": "0.1.0"}]}), ""
                )
            if command[:3] == ["cargo", "mutants", "--version"]:
                raise AssertionError("pin query must use the bounded owned phase")
            raise AssertionError(command)

        phases_started = []

        def process(command, root, *, label, **kwargs):
            phases_started.append(label)
            probe = label == "version"
            phase_root = Path(root)
            phase_status = probe_status if probe else status
            phase_interrupted = probe_interrupted if probe else interrupted
            if probe:
                (phase_root / "version.log").write_text(
                    "cargo-nextest 0.9.146 (fixture)\nrelease: 0.9.146\n"
                    if command[:2] == ["cargo", "nextest"] else "cargo-mutants " + probe_version + "\n"
                )
            else:
                evidence(
                    output,
                    (
                        "CaughtMutant",
                        "Timeout"
                        if status == 3
                        else "MissedMutant"
                        if status == 2
                        else "CaughtMutant",
                    ),
                )
                if test_tool == "nextest":
                    result_path = output / "mutants.out/outcomes.json"
                    result = json.loads(result_path.read_text())
                    for scenario in result["outcomes"]:
                        for item in scenario["phase_results"]:
                            item["argv"] = ["cargo", "nextest", "run"]
                            if item["phase"] == "Test" and scenario["summary"] == "CaughtMutant":
                                item["process_status"] = {"Failure": 100}
                        (output / "mutants.out" / scenario["log_path"]).write_text(nextest_log(failed=scenario["summary"] == "CaughtMutant", label=nextest_status_label))
                    result_path.write_text(json.dumps(result))
                if empty_plan:
                    (output / "mutants.out/mutants.json").write_text("[]")
            result = {
                "returncode": 70 if phase_interrupted else phase_status,
                "refusal": "interrupted"
                if phase_interrupted
                else "wall-time-exceeded"
                if phase_status == 125
                else None,
                "cleanup_errors": (probe_cleanup_errors if probe else cleanup_errors)
                or [],
            }
            if probe_resources if probe else resources:
                (phase_root / (label + ".resources.json")).write_text(
                    json.dumps(result)
                )
                return result
            raise OSError("process report unavailable")

        argv = [
            "mutation-audit.py",
            "--dir",
            str(crate),
            "--output",
            str(output),
            "--jobs",
            "4",
            "--test-tool",
            test_tool,
            "--disk-limit-mib",
            "1",
            *extra,
        ]
        environment = {
            key: value
            for key, value in os.environ.items()
            if not key.startswith("HTMLCUT_")
            and key not in {
                "CARGO_TARGET_DIR",
                "CARGO_BUILD_TARGET_DIR",
                "CARGO_BUILD_BUILD_DIR",
            }
        }
        with (
            patch.dict(os.environ, environment, clear=True),
            patch.object(sys, "argv", argv),
            patch.object(audit.os, "cpu_count", return_value=8),
            patch.object(audit.subprocess, "run", side_effect=tools),
            patch.object(audit.process, "run", side_effect=process) as launched,
            patch.object(audit, "source_identity", side_effect=[before, after]),
            patch.object(audit.signal, "signal"),
            patch.object(audit.install, "version_matches", return_value=True, create=True),
            patch.object(audit, "doctest_baseline", return_value={"returncode": 0, "cleanup_errors": [], "source_sha256": "before", "tests": {"runnable": ["tiny::tiny$tests::detector"], "ignored": []}}),
            patch.object(audit.platform, "platform", return_value="test-host"),
            patch.object(audit.config, "tools", return_value={"tools": {
                "cargo-mutants": {"version": VERSION}, "cargo-nextest": {"version": "0.9.146"}
            }}),
        ):
            missing_diff = (
                "--in-diff" in extra
                and not Path(extra[extra.index("--in-diff") + 1]).is_file()
            )
            if output_inside or missing_diff:
                with self.assertRaises(SystemExit) as error:
                    audit.main()
                self.assertEqual(error.exception.code, 2)
                self.assertFalse(output.exists())
                launched.assert_not_called()
                return None, output
            actual = audit.main()
            if (
                probe_status != 0
                or probe_version != VERSION
                or not probe_resources
                or probe_cleanup_errors
                or probe_interrupted
            ):
                self.assertEqual(
                    phases_started,
                    ["version"],
                    "invalid pin phase must not launch a campaign",
                )
        return actual, output

    def test_exclusions_alone_are_focused_and_forward_exact_patterns(self):
        patterns = [
            "crates/htmlcut-runtime/src/authorization/windows.rs",
            "crates/htmlcut-runtime/src/authorization/windows/**",
        ]
        with tempfile.TemporaryDirectory() as directory:
            actual, output = self.run_main(
                directory,
                extra=tuple(
                    part for pattern in patterns for part in ("--exclude-file", pattern)
                ),
            )
            result = json.loads((output / "audit.json").read_text())
            self.assertEqual(actual, 0)
            self.assertEqual(result["scope"], "focused")
            self.assertEqual(result["exclude_files"], patterns)
            command = result["command"]
            self.assertEqual(
                [
                    command[index + 1]
                    for index, value in enumerate(command)
                    if value == "--exclude"
                ],
                patterns,
            )
            self.assertNotIn("--exclude-file", command)
            self.assertIn("--workspace", command)
            self.assertIn("--all-features", command)
            self.assertIn("--cargo-arg=--package=tiny@0.1.0", command)
            self.assertEqual(result["test_packages"], ["tiny@0.1.0"])
            self.assertEqual(command[command.index("--test-workspace") + 1], "true")

    def test_exclusion_only_empty_plan_cannot_claim_completion(self):
        with tempfile.TemporaryDirectory() as directory:
            actual, output = self.run_main(
                directory, extra=("--exclude-file", "**"), empty_plan=True
            )
            result = json.loads((output / "audit.json").read_text())
            self.assertEqual(actual, 125)
            self.assertEqual(result["scope"], "focused")
            self.assertEqual(result["exclude_files"], ["**"])
            self.assertFalse(result["execution_complete"])
            self.assertEqual(result["tool_returncode"], 0)
            self.assertIn("empty mutation plan", result["evidence_error"])

    def test_recursive_test_threads_are_forced_to_one(self):
        for configured, expected in [(None, "1"), ("3", "1")]:
            environment = {
                key: value
                for key, value in os.environ.items()
                if key != "RUST_TEST_THREADS"
            }
            if configured is not None:
                environment["RUST_TEST_THREADS"] = configured
            with (
                self.subTest(configured=configured),
                patch.dict(os.environ, environment, clear=True),
                tempfile.TemporaryDirectory() as directory,
            ):
                actual, output = self.run_main(directory)
                result = json.loads((output / "audit.json").read_text())
                self.assertEqual(actual, 0)
                self.assertNotIn("--cargo-test-arg=--fail-fast", result["command"])
                self.assertEqual(
                    result["execution_environment"]["RUST_TEST_THREADS"], expected
                )
                self.assertIn("--workspace", result["command"])
                self.assertEqual(
                    result["command"][result["command"].index("--test-workspace") + 1],
                    "true",
                )

    def test_invalid_or_refused_pin_probe_is_retained_without_mutation_completion(self):
        for options in [
            dict(probe_version="0.0.0"),
            dict(probe_status=2),
            dict(probe_status=125),
            dict(probe_resources=False),
            dict(probe_cleanup_errors=["pin child unconfirmed"]),
            dict(probe_cleanup_errors=[1]),
        ]:
            with (
                self.subTest(options=options),
                tempfile.TemporaryDirectory() as directory,
            ):
                actual, output = self.run_main(directory, **options)
                report = json.loads((output / "audit.json").read_text())
                self.assertEqual(actual, 125)
                self.assertFalse(report["execution_complete"])
                self.assertTrue(report["source_stable"])
                self.assertIsNone(report["tool_returncode"])
                self.assertFalse((output / "mutants.out").exists())
                self.assertEqual(
                    report["tool_version"]["command"], ["cargo", "mutants", "--version"]
                )
                self.assertTrue((output / "tool-version/version.log").is_file())
                if "probe_version" in options:
                    self.assertIn(
                        "install the pinned tool", report["tool_version_error"]
                    )
                if options.get("probe_status") == 2:
                    self.assertEqual(report["tool_version"]["tool_returncode"], 2)
                if options.get("probe_resources") is False or options.get(
                    "probe_cleanup_errors"
                ):
                    self.assertTrue((output / "scratch").is_dir())
                    self.assertIn(
                        str((output / "tool-version").resolve()),
                        report["retained_phase_roots"],
                    )

    def test_interrupted_pin_probe_records_own_status_and_skips_campaign(self):
        with tempfile.TemporaryDirectory() as directory:
            actual, output = self.run_main(
                directory, probe_status=125, probe_interrupted=True
            )
            report = json.loads((output / "audit.json").read_text())
            self.assertEqual(actual, 130)
            self.assertEqual(report["tool_version"]["tool_returncode"], 70)
            self.assertIsNone(report["tool_returncode"])
            self.assertFalse(report["execution_complete"])

    def test_shared_build_overrides_refused_before_any_tool_launch(self):
        for key in (
            "CARGO_TARGET_DIR",
            "CARGO_BUILD_TARGET_DIR",
            "CARGO_BUILD_BUILD_DIR",
        ):
            with self.subTest(key=key), tempfile.TemporaryDirectory() as directory:
                output = Path(directory) / "output"
                with (
                    patch.dict(os.environ, {key: str(Path(directory) / "owner-build")}, clear=True),
                    patch.object(
                        sys, "argv", ["mutation-audit.py", "--output", str(output)]
                    ),
                    patch.object(audit.subprocess, "run") as launched,
                ):
                    with self.assertRaises(SystemExit) as error:
                        audit.main()
                    self.assertEqual(error.exception.code, 2)
                    self.assertFalse(output.exists())
                    launched.assert_not_called()

    def test_nonzero_completed_outcomes_preserve_status_and_review_requirement(self):
        for status in [0, 2, 3]:
            with (
                self.subTest(status=status),
                tempfile.TemporaryDirectory() as directory,
            ):
                actual, output = self.run_main(directory, status=status)
                result = json.loads((output / "audit.json").read_text())
                self.assertEqual(actual, status)
                self.assertEqual(result["tool_returncode"], status)
                self.assertTrue(result["execution_complete"])
                self.assertFalse(result["full_review_complete"])
                self.assertTrue(result["review_required"])
                self.assertEqual(result["jobs"], 4)
                self.assertEqual(result["scope"], "full workspace")
                self.assertEqual(result["exclude_files"], [])
                self.assertNotIn("--exclude", result["command"])
                self.assertFalse((output / "scratch").exists())

    def test_interrupted_runner_preserves_distinct_helper_and_tool_status(self):
        with tempfile.TemporaryDirectory() as directory:
            actual, output = self.run_main(directory, status=125, interrupted=True)
            result = json.loads((output / "audit.json").read_text())
            self.assertEqual(actual, 130)
            self.assertEqual(result["tool_returncode"], 70)
            self.assertFalse(result["execution_complete"])
            self.assertFalse((output / "scratch").exists())

    def test_source_change_invalidates_completed_execution(self):
        with tempfile.TemporaryDirectory() as directory:
            actual, output = self.run_main(directory, source_changed=True)
            result = json.loads((output / "audit.json").read_text())
            self.assertEqual(actual, 125)
            self.assertFalse(result["execution_complete"])
            self.assertFalse(result["source_stable"])
            self.assertEqual(result["changed_source_files"], ["src/lib.rs"])
            self.assertEqual(result["tool_returncode"], 0)

    def test_unconfirmed_cleanup_keeps_owned_scratch(self):
        for resources in [False, True]:
            with (
                self.subTest(resources=resources),
                tempfile.TemporaryDirectory() as directory,
            ):
                actual, output = self.run_main(
                    directory, resources=resources, cleanup_errors=["child still alive"]
                )
                result = json.loads((output / "audit.json").read_text())
                self.assertEqual(actual, 125)
                self.assertFalse(result["execution_complete"])
                self.assertTrue((output / "scratch").is_dir())
                self.assertEqual(
                    result["retained_scratch"], str((output / "scratch").resolve())
                )
                self.assertTrue(result["cleanup_errors"])

    def test_subcrate_does_not_allow_output_inside_workspace(self):
        with tempfile.TemporaryDirectory() as directory:
            self.run_main(directory, output_inside=True)

    def test_missing_diff_is_rejected_before_owned_output_creation(self):
        with tempfile.TemporaryDirectory() as directory:
            self.run_main(
                directory, extra=("--in-diff", str(Path(directory) / "absent.diff"))
            )

    def test_diff_selection_is_staged_and_bound(self):
        with tempfile.TemporaryDirectory() as directory:
            diff = Path(directory) / "input.diff"
            contents = b"--- a/src/lib.rs\n+++ b/src/lib.rs\n"
            diff.write_bytes(contents)
            actual, output = self.run_main(directory, extra=("--in-diff", str(diff)))
            result = json.loads((output / "audit.json").read_text())
            self.assertEqual(actual, 0)
            self.assertEqual(result["scope"], "focused")
            self.assertEqual((output / "selection.diff").read_bytes(), contents)
            selected = result["command"][result["command"].index("--in-diff") + 1]
            self.assertEqual(Path(selected), (output / "selection.diff").resolve())
            self.assertEqual(
                result["selection_diff"]["sha256"], audit.file_digest(diff)
            )


class SourceTests(unittest.TestCase):
    def test_actual_source_bytes_are_bound_and_build_products_ignored(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "Cargo.toml").write_text("[workspace]\n")
            before = audit.source_identity(root)
            (root / "target").mkdir()
            (root / "target/build-output").write_bytes(b"compile product")
            self.assertEqual(audit.source_identity(root), before)
            (root / "Cargo.toml").write_text("[workspace]\nmembers=[]\n")
            self.assertNotEqual(audit.source_identity(root), before)


class NextestEvidenceTests(unittest.TestCase):
    def validate(self, text, failed=True):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "scenario.log"
            path.write_text(text)
            return audit.nextest_evidence(path, failed=failed)

    def test_clean_test_protocol_is_required_in_addition_to_exit_status(self):
        self.assertEqual(self.validate(nextest_log(failed=True))["tests_failed"], 1)
        self.assertEqual(self.validate(nextest_log(), False)["tests_finished"], 1)

    def test_exec_timeout_leak_and_ambiguous_protocol_never_count_as_caught(self):
        good = nextest_log(failed=True)
        variants = [
            good.replace('"stdout": "independent assertion failed"', '"reason": "time limit exceeded"'),
            good.replace('independent assertion failed', '--- EXECUTION ERROR ---\\nspawn failed'),
            good.replace('independent assertion failed', 'normal failure\\n--- EXECUTION ERRORS ---\\nread failure'),
            good.replace('"event": "failed"', '"event": "ok"', 1),
            good.replace(', "stdout": "independent assertion failed"', ''),
            good.replace(' Summary [0.01s] 1 test run: complete\n', ''),
            '{"type":malformed}\n' + good,
            '\n'.join(good.splitlines()[1:]) + '\n',
        ]
        variants += [nextest_log(failed=True, label=label)
                     for label in ("XFAIL", "LEAK", "LEAK-FAIL", "FAIL + LEAK", "PASS + LEAK", "SLOW + LEAK", "TIMEOUT", "TIMEOUT-PASS", "SETUP-FAIL", "SETUP LEAK")]
        for text in variants:
            with self.subTest(text=text), self.assertRaises(ValueError):
                self.validate(text)

    def test_nextest_no_tests_or_setup_exit_is_not_a_test_failure(self):
        for code in (4, 70, 94, 96, 101, 102, 104, 105, 110):
            outcome = {"summary": "CaughtMutant", "phase_results": phases("CaughtMutant")}
            for phase in outcome["phase_results"]:
                phase["argv"] = ["cargo", "nextest", "run"]
            outcome["phase_results"][1]["process_status"] = {"Failure": code}
            with self.subTest(code=code), self.assertRaises(ValueError):
                audit.validate_phases(outcome, test_tool="nextest")


class NextestRunnerTests(unittest.TestCase):
    run_main = RunnerTests.run_main

    def test_optimized_runner_records_bounded_policy_and_actual_baseline_phase(self):
        with tempfile.TemporaryDirectory() as directory:
            status, output = self.run_main(directory, test_tool="nextest")
            report = json.loads((output / "audit.json").read_text())
            self.assertEqual(status, 0)
            self.assertTrue(report["execution_complete"])
            self.assertEqual(report["test_threads"], 2)
            self.assertEqual(report["execution_environment"]["RUST_TEST_THREADS"], "1")
            self.assertEqual(report["execution_environment"]["NEXTEST_DOUBLE_SPAWN"], "0")
            self.assertIn("--leak-dirs", report["command"])
            for flag in ("--fail-fast", "--retries=0", "--no-tests=fail", "--ignore-default-filter",
                         "--message-format=libtest-json-plus", "--message-format-version=0.1",
                         "--test-threads=2", "--failure-output=never", "--success-output=never"):
                self.assertIn("--cargo-test-arg=" + flag, report["command"])
                self.assertNotIn("--cargo-arg=" + flag, report["command"])
            self.assertEqual(report["doctest_baseline"]["source_sha256"], "before")
            self.assertIn("baseline-only", report["test_scope"])
            self.assertGreater(report["phase_timings"]["Test"]["count"], 0)

    def test_failed_assertion_with_leak_retains_scratch_before_any_removal(self):
        with tempfile.TemporaryDirectory() as directory:
            status, output = self.run_main(directory, test_tool="nextest", nextest_status_label="FAIL + LEAK")
            report = json.loads((output / "audit.json").read_text())
            self.assertEqual(status, 125)
            self.assertFalse(report["execution_complete"])
            self.assertTrue(report["descendant_cleanup_uncertain"])
            self.assertTrue((output / "scratch").is_dir())

    def test_reserved_test_controls_refuse_before_creating_output(self):
        for key in ("HTMLCUT_TEST_BINARY", "HTMLCUT_LITERAL_DASH_CHILD", "HTMLCUT_UNRECOGNIZED_CONTROL"):
            for value in ("", "controlled"):
                with self.subTest(key=key, value=value), tempfile.TemporaryDirectory() as directory:
                    output = Path(directory) / "result"
                    with patch.dict(os.environ, {key: value}, clear=True), patch.object(sys, "argv", ["mutation", "--output", str(output)]), patch.object(audit.subprocess, "run") as launched:
                        with self.assertRaises(SystemExit) as error:
                            audit.main()
                        self.assertEqual(error.exception.code, 2)
                        launched.assert_not_called()
                        self.assertFalse(output.exists())


class DoctestBaselineTests(unittest.TestCase):
    @unittest.skipUnless(shutil.which("cargo"), "Cargo is required for config parsing")
    def test_native_build_directory_config_is_accepted_by_cargo(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "src").mkdir()
            (root / "src/lib.rs").write_text("", encoding="utf-8")
            (root / "Cargo.toml").write_text(
                '[package]\nname="path-probe"\nversion="0.0.0"\n[workspace]\n', encoding="utf-8")
            suffix = "config path 雪🦀" if os.name == "nt" else "D:\\a\\_temp\\baseline 雪🦀\\build"
            intended = root / suffix
            command = ["cargo", "+" + audit.config.toolchain(), "metadata", "--offline", "--no-deps",
                       "--format-version=1", "--manifest-path", str(root / "Cargo.toml"), "--config"]
            old = subprocess.run(command + ['build.build-dir="' + str(intended) + '"'],
                                 cwd=root, capture_output=True, timeout=30)
            self.assertNotEqual(old.returncode, 0, old.stdout)
            self.assertIn(b"failed to parse value from --config", old.stderr)
            quoted = audit.build_directory_config(intended)
            self.assertEqual(tomllib.loads(quoted)["build"]["build-dir"], str(intended))
            accepted = subprocess.run(command + [quoted], cwd=root, capture_output=True, timeout=30)
            self.assertEqual(accepted.returncode, 0, accepted.stderr)
            self.assertEqual(json.loads(accepted.stdout)["build_directory"], str(intended))

    def test_baseline_uses_hash_checked_owned_source_and_records_the_real_phase(self):
        from types import SimpleNamespace
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / "workspace"
            source.mkdir()
            (source / "Cargo.toml").write_text('[package]\nname="tiny"\nversion="0.1.0"\n')
            output = root / "results"
            output.mkdir()
            identity = audit.source_identity(source)
            args = SimpleNamespace(timeout=10, disk_limit_mib=1)
            observed = []
            def phase(command, owned, label, **kwargs):
                copied = owned / "source/Cargo.toml"
                self.assertEqual(copied.read_bytes(), (source / "Cargo.toml").read_bytes())
                if label == "doctests":
                    self.assertIn("--doc", command)
                else:
                    (owned / "discovery.log").write_text(json.dumps(nextest_listing()) + "\n")
                self.assertIn("--locked", command)
                self.assertIn("--offline", command)
                build_config = command[command.index("--config") + 1]
                self.assertEqual(tomllib.loads(build_config)["build"]["build-dir"], str(owned / "build"))
                self.assertEqual(Path(kwargs["env"]["TMPDIR"]).parent, owned)
                self.assertLessEqual(kwargs["timeout"], 10)
                observed.append(command)
                return {"returncode": 0, "cleanup_errors": [], "command": command, "log_path": str(owned / "discovery.log")}, {}
            with patch.object(audit, "guarded_phase", side_effect=phase):
                result = audit.doctest_baseline(source, output, identity, args, {}, audit.time.monotonic())
            self.assertEqual(len(observed), 2)
            self.assertEqual(result["source_sha256"], identity["sha256"])
            self.assertFalse((output / "doctest-baseline/source").exists())
            self.assertTrue((source / "Cargo.toml").exists())

    def test_uncertain_doctest_cleanup_retains_only_owned_products(self):
        from types import SimpleNamespace
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / "workspace"
            source.mkdir()
            (source / "Cargo.toml").write_text("[workspace]\n")
            output = root / "results"
            output.mkdir()
            def phase(command, owned, label, **kwargs):
                (owned / "target").mkdir()
                (owned / "target/product").write_text("owned")
                return {"returncode": 125, "cleanup_errors": ["child not confirmed stopped"]}, {}
            with patch.object(audit, "guarded_phase", side_effect=phase):
                result = audit.doctest_baseline(source, output, audit.source_identity(source), SimpleNamespace(timeout=10, disk_limit_mib=1), {}, audit.time.monotonic())
            self.assertEqual(result["returncode"], 125)
            self.assertTrue((output / "doctest-baseline/target/product").exists())
            self.assertTrue((output / "doctest-baseline/source/Cargo.toml").exists())


class SourceSymlinkTests(unittest.TestCase):
    def test_standalone_directory_symlink_is_bound_without_following_it(self):
        with tempfile.TemporaryDirectory() as directory:
            base = Path(directory)
            root = base / "source"
            root.mkdir()
            target = base / "outside"
            target.mkdir()
            (target / "not-enumerated").write_text("outside")
            alias = root / "linked-directory"
            try:
                alias.symlink_to(target, target_is_directory=True)
            except OSError as error:
                self.skipTest("directory symlink unavailable: " + str(error))
            identity = audit.source_identity(root)
            self.assertEqual(identity["files"], {"linked-directory": {"symlink": str(target)}})
            alias.unlink()
            alias.symlink_to(base / "different", target_is_directory=True)
            self.assertNotEqual(identity, audit.source_identity(root))


class NextestDiscoveryTests(unittest.TestCase):
    def test_discovery_rejects_empty_filtered_duplicate_or_malformed_detectors(self):
        cases = [None, {}, {"test-count": 1, "rust-suites": {}}, nextest_listing()]
        cases[-1]["rust-suites"]["tiny"]["testcases"]["tests::detector"]["filter-match"] = {"status": "mismatch", "reason": "expression"}
        duplicate = nextest_listing()
        duplicate["rust-suites"]["duplicate"] = copy.deepcopy(duplicate["rust-suites"]["tiny"])
        cases.append(duplicate)
        for value in cases:
            with self.subTest(value=value), tempfile.TemporaryDirectory() as directory:
                path = Path(directory) / "listing"
                path.write_text(json.dumps(value) + "\n")
                with self.assertRaises(ValueError):
                    audit.nextest_discovery(path)

    def test_ignored_only_counter_underflow_does_not_waive_runnable_coverage(self):
        rows = [json.loads(line) for line in nextest_log().splitlines() if line.startswith('{')]
        identity = {"crate": "tiny", "test_binary": "native", "kind": "test"}
        ignored = "tiny::native$requires_native_acl"
        rows += [
            {"type": "suite", "event": "started", "test_count": 1, "nextest": identity},
            {"type": "test", "event": "started", "name": ignored},
            {"type": "suite", "event": "ok", "passed": 0, "failed": 0, "ignored": 1, "measured": 0, "filtered_out": 18446744073709551615, "nextest": identity},
        ]
        text = '\n'.join(json.dumps(row, separators=(',', ':')) for row in rows) + '\n Summary [0.01s] 1 test run: 1 passed, 1 skipped\n'
        expected = {"runnable": ["tiny::tiny$tests::detector"], "ignored": [ignored]}
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "scenario"
            path.write_text(text)
            self.assertEqual(audit.nextest_evidence(path, failed=False, expected=expected)["tests_finished"], 1)
            expected["runnable"].append("tiny::tiny$missing_detector")
            with self.assertRaises(ValueError):
                audit.nextest_evidence(path, failed=False, expected=expected)
