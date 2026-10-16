#!/usr/bin/env python3
# SPDX-License-Identifier: MPL-2.0
"""Run cargo-mutants in disposable build roots, retaining review evidence."""

import argparse
from collections import Counter
import hashlib
import json
import math
import os
from pathlib import Path
import platform
import re
import shutil
import signal
import stat
import statistics
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parents[2]
from tools.htmlcut_maintenance import config, install, process

VERDICTS = {
    "CaughtMutant": "caught",
    "MissedMutant": "missed",
    "Timeout": "timeout",
    "Unviable": "unviable",
}


def nextest_discovery(path):
    """Read the pinned authoritative listing, not a hand-maintained test inventory."""
    try:
        candidates = []
        with path.open(encoding="utf-8") as log:
            for line in log:
                if line.startswith("{"):
                    value = json.loads(line)
                    if isinstance(value, dict) and "rust-suites" in value:
                        candidates.append(value)
        if len(candidates) != 1:
            raise ValueError("missing or duplicate Nextest discovery")
        discovery = candidates[0]
        runnable, ignored = set(), set()
        for suite in discovery["rust-suites"].values():
            if suite["status"] != "listed":
                raise ValueError("Nextest discovery skipped a binary")
            for name, case in suite["testcases"].items():
                identity = suite["package-name"] + "::" + suite["binary-name"] + "$" + name
                if identity in runnable or identity in ignored or type(case["ignored"]) is not bool:
                    raise ValueError("duplicate or invalid Nextest discovery identity")
                if case["ignored"]:
                    if case["filter-match"] != {"status": "mismatch", "reason": "ignored"}:
                        raise ValueError("unexpected ignored-test filter")
                    ignored.add(identity)
                else:
                    if case["filter-match"] != {"status": "matches"}:
                        raise ValueError("Nextest discovery filtered a runnable detector")
                    runnable.add(identity)
        if not runnable or type(discovery["test-count"]) is not int or discovery["test-count"] != len(runnable) + len(ignored):
            raise ValueError("empty or inconsistent Nextest discovery")
        return {"runnable": sorted(runnable), "ignored": sorted(ignored)}
    except (KeyError, TypeError, AttributeError) as error:
        raise ValueError("malformed Nextest discovery") from error


def nextest_evidence(path, *, failed, expected=None):
    """Check test events against discovery; suite counters in 0.1 are unreliable."""
    tests, terminals, summaries = {}, {}, []
    known = set(expected["runnable"]) | set(expected["ignored"]) if expected else None
    ignored = set(expected["ignored"]) if expected else set()
    rejected_status = re.compile(
        r"^\s*(?:(?:FAIL|PASS|SLOW)\s*\+\s*LEAK|LEAK(?:-FAIL|-PASS)?|XFAIL|TIMEOUT(?:-PASS)?|SETUP[- ](?:FAIL|LEAK))\s+\["
    )
    suites_started, suites_finished = Counter(), Counter()
    with path.open(encoding="utf-8") as log:
        while line := log.readline(8 * 1024 * 1024 + 1):
            if len(line) > 8 * 1024 * 1024:
                raise ValueError("oversized Nextest evidence line")
            if rejected_status.match(line):
                raise ValueError("Nextest execution, timeout or leak status: " + line.strip())
            summary = re.match(r"^\s*Summary\s+\[[^]]+\]\s+(\d+)(?:/\d+)? tests? run:", line)
            if summary:
                summaries.append(int(summary[1]))
            if not line.startswith('{"type":'):
                continue
            event = json.loads(line)
            if not isinstance(event, dict):
                raise ValueError("invalid Nextest event object")
            kind, action = event.get("type"), event.get("event")
            if kind == "suite":
                identity = event.get("nextest", {})
                if not isinstance(identity, dict) or any(not isinstance(identity.get(key), str) or not identity[key]
                       for key in ("crate", "test_binary", "kind")):
                    raise ValueError("missing Nextest suite identity")
                prefix = identity["crate"] + "::" + identity["test_binary"]
                if known is not None and not any(name.startswith(prefix + "$") for name in known):
                    raise ValueError("unknown Nextest suite identity")
                suite = (prefix, identity["kind"])
                if action == "started":
                    if type(event.get("test_count")) is not int or event["test_count"] <= 0:
                        raise ValueError("invalid Nextest suite size")
                    suites_started[suite] += 1
                elif action in ("ok", "failed"):
                    if any(type(event.get(field)) is not int or event[field] < 0
                           for field in ("passed", "failed", "ignored", "measured", "filtered_out")):
                        raise ValueError("invalid Nextest suite fields")
                    suites_finished[suite] += 1
                else:
                    raise ValueError("unknown Nextest suite event")
                # 0.1 splits suites and underflows ignored-only filtered_out.
                # Independent discovered identities and terminal test events are the oracle.
                continue
            if kind != "test":
                raise ValueError("unknown Nextest JSON event")
            name = event.get("name")
            if not isinstance(name, str) or "$" not in name or known is not None and name not in known:
                raise ValueError("unknown Nextest test identity")
            if action == "started":
                if name in tests:
                    raise ValueError("duplicate Nextest test start/retry")
                tests[name] = True
            elif action in ("ok", "failed", "ignored"):
                if name not in tests or name in terminals:
                    raise ValueError("unstarted or duplicate Nextest test completion")
                if action == "failed":
                    output = event.get("stdout")
                    if (event.get("reason") is not None or not isinstance(output, str)
                        or output.startswith(("--- EXECUTION ERROR ---\n", "error: [double-spawn]"))
                        or "\n--- EXECUTION ERRORS ---\n" in output):
                        raise ValueError("Nextest failure is not clean test execution evidence")
                if action != "ignored" and (type(event.get("exec_time")) not in (int, float)
                                             or not math.isfinite(event["exec_time"]) or event["exec_time"] < 0):
                    raise ValueError("invalid Nextest test duration")
                if action == "ignored":
                    if expected and name not in ignored:
                        raise ValueError("Nextest skipped a runnable detector")
                    ignored.add(name)
                elif name in ignored:
                    raise ValueError("Nextest unexpectedly executed an ignored test")
                terminals[name] = action
            else:
                raise ValueError("unknown Nextest test event")
    executed = {name for name, state in terminals.items() if state in ("ok", "failed")}
    failures = sum(state == "failed" for state in terminals.values())
    required = set(expected["runnable"]) if expected else set(tests) - ignored
    if (not executed or summaries != [len(executed)] or not suites_started
        or suites_started != suites_finished or bool(failures) != failed
        or set(tests) - set(terminals) - ignored
        or not failed and executed != required):
        raise ValueError("missing or inconsistent complete Nextest execution evidence")
    return {"tests_finished": len(executed), "tests_failed": failures}


def timing_summary(outcomes):
    groups = {}
    for outcome in outcomes:
        for phase in outcome["phase_results"]:
            groups.setdefault(phase["phase"], []).append(phase["duration"])
    return {name: {"count": len(values), "total_seconds": sum(values),
                   "median_seconds": statistics.median(values), "max_seconds": max(values)}
            for name, values in groups.items()}


def file_digest(path):
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def source_identity(workspace):
    """Bind actual inputs; ignored build/cache products are not release source."""
    listed = subprocess.run(
        [
            "git",
            "-C",
            str(workspace),
            "ls-files",
            "-z",
            "--cached",
            "--others",
            "--exclude-standard",
            "--",
            ".",
        ],
        capture_output=True,
        check=False,
    )
    if listed.returncode == 0:
        names = sorted(
            {os.fsdecode(name) for name in listed.stdout.split(b"\0") if name}
        )
        commit = subprocess.check_output(
            ["git", "-C", str(workspace), "rev-parse", "HEAD"], text=True
        ).strip()
    else:
        # Standalone workspaces also permit small, real tool integration controls.
        names = []
        for directory, children, files in os.walk(workspace, followlinks=False):
            children[:] = [name for name in children if name not in {"target", ".git"}]
            for name in children:
                child = Path(directory) / name
                if child.is_symlink():
                    names.append(str(child.relative_to(workspace)))
            for name in files:
                names.append(str((Path(directory) / name).relative_to(workspace)))
        names.sort()
        commit = None
    manifest = {}
    for name in names:
        path = workspace / name
        try:
            info = path.lstat()
        except FileNotFoundError:
            manifest[name] = {"missing": True}
            continue
        if stat.S_ISLNK(info.st_mode):
            manifest[name] = {"symlink": os.readlink(path)}
        elif stat.S_ISREG(info.st_mode):
            manifest[name] = {
                "sha256": file_digest(path),
                "executable": bool(info.st_mode & 0o111),
            }
        elif stat.S_ISDIR(info.st_mode):
            raise ValueError(
                "source submodules/directories need their own source binding: " + name
            )
        else:
            raise ValueError("unsupported source file type: " + name)
    encoded = json.dumps(manifest, sort_keys=True, separators=(",", ":")).encode()
    return {
        "git_commit": commit,
        "sha256": hashlib.sha256(encoded).hexdigest(),
        "files": manifest,
    }


def evidence_file(root, value):
    if not isinstance(value, str) or not value:
        raise ValueError("missing evidence path")
    path = (root / value).resolve()
    if root.resolve() not in path.parents or not path.is_file():
        raise ValueError("missing or external evidence file: " + value)
    return path


def validate_phases(outcome, baseline=False, test_tool="cargo"):
    phases = outcome["phase_results"]
    summary = outcome["summary"]
    actual = [(phase["phase"], phase["process_status"]) for phase in phases]
    for phase in phases:
        if (
            type(phase["duration"]) not in (int, float)
            or not math.isfinite(phase["duration"])
            or phase["duration"] < 0
            or not isinstance(phase["argv"], list)
            or not phase["argv"]
            or any(not isinstance(arg, str) for arg in phase["argv"])
        ):
            raise ValueError("invalid phase execution record")
        if test_tool == "nextest" and phase["argv"][1:3] != ["nextest", "run"]:
            raise ValueError("Nextest phase used an unexpected runner")
    failure = lambda value, code: (
        isinstance(value, dict)
        and set(value) == {"Failure"}
        and type(value["Failure"]) is int
        and value["Failure"] == code
    )
    valid = actual == [("Build", "Success"), ("Test", "Success")] if baseline else False
    if not baseline:
        valid = (
            summary == "CaughtMutant"
            and len(actual) == 2
            and actual[0] == ("Build", "Success")
            and actual[1][0] == "Test"
            and failure(actual[1][1], 100 if test_tool == "nextest" else 101)
            or summary == "MissedMutant"
            and actual == [("Build", "Success"), ("Test", "Success")]
            or summary == "Unviable"
            and len(actual) == 1
            and actual[0][0] == "Build"
            and failure(actual[0][1], 101)
            or summary == "Timeout"
            and actual
            in [[("Build", "Timeout")], [("Build", "Success"), ("Test", "Timeout")]]
        )
    if not valid or baseline and summary != "Success":
        raise ValueError("inconsistent scenario phases: " + str(summary))


def complete_outcomes(output, tool_status, version, test_tool="cargo", expected_tests=None):
    root = output / "mutants.out"
    plan = json.loads((root / "mutants.json").read_text(encoding="utf-8"))
    result = json.loads((root / "outcomes.json").read_text(encoding="utf-8"))
    if not isinstance(plan, list) or not plan:
        raise ValueError("empty mutation plan")
    if any(
        not isinstance(item, dict)
        or not isinstance(item.get("name"), str)
        or not item["name"]
        for item in plan
    ):
        raise ValueError("invalid planned mutant identity")
    planned = {
        item["name"]: {key: value for key, value in item.items() if key != "diff"}
        for item in plan
    }
    if len(planned) != len(plan):
        raise ValueError("duplicate planned mutants")
    baseline = []
    tested = {}
    counts = Counter()
    for item in result["outcomes"]:
        scenario = item["scenario"]
        evidence_file(root, item["log_path"])
        if scenario == "Baseline":
            baseline.append(item)
            validate_phases(item, baseline=True, test_tool=test_tool)
        elif isinstance(scenario, dict) and set(scenario) == {"Mutant"}:
            mutant = scenario["Mutant"]
            name = mutant["name"]
            if name in tested or planned.get(name) != mutant:
                raise ValueError("duplicate, unplanned or changed mutant: " + str(name))
            if item["summary"] not in VERDICTS:
                raise ValueError("invalid mutant verdict: " + str(item["summary"]))
            validate_phases(item, test_tool=test_tool)
            evidence_file(root, item["diff_path"])
            tested[name] = item
            counts[VERDICTS[item["summary"]]] += 1
        else:
            raise ValueError("invalid mutation scenario")
        if test_tool == "nextest" and item["summary"] == "Timeout":
            raise ValueError("Nextest timeout leaves descendant cleanup uncertain")
        if test_tool == "nextest" and any(phase["phase"] == "Test" for phase in item["phase_results"]):
            if expected_tests is None:
                raise ValueError("missing independent Nextest test discovery")
            nextest_evidence(evidence_file(root, item["log_path"]),
                             failed=item["summary"] == "CaughtMutant", expected=expected_tests)
    if len(baseline) != 1 or set(tested) != set(planned):
        raise ValueError("missing baseline or incomplete mutation outcomes")
    for key in ["caught", "missed", "timeout", "unviable", "success"]:
        if type(result[key]) is not int or result[key] != counts[key]:
            raise ValueError("incorrect outcome count: " + key)
    if type(result["total_mutants"]) is not int or result["total_mutants"] != len(
        planned
    ):
        raise ValueError("incorrect total mutant count")
    expected_status = 3 if counts["timeout"] else 2 if counts["missed"] else 0
    if (
        tool_status != expected_status
        or result["cargo_mutants_version"] != version
        or not result["end_time"]
    ):
        raise ValueError("inconsistent completion status/version")
    return {
        "planned_mutants": len(planned),
        "tested_mutants": len(tested),
        "counts": {
            key: counts[key] for key in ["caught", "missed", "timeout", "unviable"]
        },
        "phase_timings": timing_summary(result["outcomes"]),
    }


def guarded_phase(command, root, label, *, disk_limit_mib, timeout, log_limit_mib, env):
    """Direct ownership, without a second process supervising the supervisor."""
    resources = None
    errors = []
    try:
        resources = process.run(
            command,
            root,
            label=label,
            disk_limit_mib=disk_limit_mib,
            timeout=timeout,
            log_limit_mib=log_limit_mib,
            env=env,
        )
        if (
            not isinstance(resources, dict)
            or not isinstance(resources.get("cleanup_errors"), list)
            or any(not isinstance(error, str) for error in resources["cleanup_errors"])
        ):
            raise ValueError("missing cleanup confirmation")
        errors.extend(resources["cleanup_errors"])
        if type(resources.get("returncode")) is not int:
            errors.append("process exit was not confirmed")
    except (OSError, ValueError, TypeError) as error:
        errors.append("process cleanup evidence unavailable: " + str(error))
    resources = resources if isinstance(resources, dict) else {}
    refusal = resources.get("refusal")
    status = (
        130
        if refusal == "interrupted"
        else 125
        if errors or refusal
        else process.command_exit_status(resources.get("returncode", 125))
    )
    details = dict(
        command=command,
        log_path=str(root / (label + ".log")),
        resource_report=str(root / (label + ".resources.json")),
        tool_returncode=resources.get("returncode"),
        elapsed_seconds=resources.get("elapsed_seconds"),
        refusal=refusal,
        cleanup_errors=errors,
        returncode=status,
    )
    return details, resources


def doctest_baseline(workspace, output, identity, args, env, started):
    """Run actual same-source doctests without modifying the audited checkout."""
    root = output / "doctest-baseline"
    root.mkdir()
    source = root / "source"
    source.mkdir()
    for name, facts in identity["files"].items():
        destination = source / name
        destination.parent.mkdir(parents=True, exist_ok=True)
        if "symlink" in facts:
            destination.symlink_to(facts["symlink"])
        elif "sha256" in facts:
            shutil.copy2(workspace / name, destination)
            if file_digest(destination) != facts["sha256"]:
                raise ValueError("source changed while staging doctest baseline: " + name)
        else:
            raise ValueError("missing doctest source input: " + name)
    remaining = args.timeout - (time.monotonic() - started)
    if remaining <= 0:
        raise ValueError("campaign budget expired before doctest baseline")
    command = ["cargo", "test", "--doc", "--workspace", "--all-features", "--locked", "--offline",
               "--manifest-path", str(source / "Cargo.toml"), "--target-dir", str(root / "target"),
               "--config", 'build.build-dir="' + str(root / "build") + '"']
    temporary = root / "temporary"
    temporary.mkdir()
    phase, _ = guarded_phase(command, root, "doctests", disk_limit_mib=args.disk_limit_mib,
                             timeout=remaining, log_limit_mib=8,
                             env={**env, "TMPDIR": str(temporary), "TMP": str(temporary), "TEMP": str(temporary)})
    phase["source_sha256"] = identity["sha256"]
    if not phase["returncode"] and not phase["cleanup_errors"]:
        remaining = args.timeout - (time.monotonic() - started)
        if remaining <= 0:
            raise ValueError("campaign budget expired before Nextest discovery")
        discovery_command = ["cargo", "nextest", "list", "--message-format=json", "--workspace",
                             "--all-features", "--locked", "--offline", "--manifest-path", str(source / "Cargo.toml"),
                             "--target-dir", str(root / "target"), "--config", 'build.build-dir="' + str(root / "build") + '"',
                             "--config-file", str(output / "nextest.toml"), "--profile=mutation", "--user-config-file=none"]
        discovery, _ = guarded_phase(discovery_command, root, "discovery", disk_limit_mib=args.disk_limit_mib,
                                     timeout=remaining, log_limit_mib=8,
                                     env={**env, "TMPDIR": str(temporary), "TMP": str(temporary), "TEMP": str(temporary)})
        phase["test_discovery"] = discovery
        if discovery["returncode"] or discovery["cleanup_errors"]:
            phase["returncode"] = discovery["returncode"] or 125
            phase["cleanup_errors"].extend(discovery["cleanup_errors"])
            return phase
        phase["tests"] = nextest_discovery(Path(discovery["log_path"]))
        for products in (source, root / "target", root / "build", temporary):
            if products.exists():
                shutil.rmtree(products)
    return phase


def main():
    parser = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.ArgumentDefaultsHelpFormatter
    )
    parser.add_argument(
        "--output", type=Path, required=True, help="new directory outside the workspace"
    )
    parser.add_argument(
        "--dir",
        type=Path,
        default=ROOT,
        help="crate directory; containing workspace is audited",
    )
    parser.add_argument(
        "--jobs",
        type=int,
        default=min(2, os.cpu_count() or 1),
        help="parallel workers, at most available CPUs",
    )
    parser.add_argument("--test-tool", choices=("nextest", "cargo"), default="nextest",
                        help="nextest runs workspace binary tests; cargo also runs doctests per mutant")
    parser.add_argument("--test-threads", type=int,
                        help="Nextest test slots per worker; defaults to CPUs divided by workers")
    parser.add_argument(
        "--disk-limit-mib",
        type=int,
        default=4096,
        help="campaign sampled disk threshold, not a quota; pinned-tool probe separately allows 4 MiB",
    )
    parser.add_argument(
        "--timeout",
        type=int,
        default=1800,
        help="campaign wall time including baseline; pinned-tool probe separately allows ten seconds",
    )
    parser.add_argument(
        "--file", action="append", default=[], help="focus on matching files"
    )
    parser.add_argument(
        "--exclude-file",
        action="append",
        default=[],
        help="exclude matching files from the mutation plan",
    )
    parser.add_argument("--regex", help="focus on matching mutant names")
    parser.add_argument(
        "--in-diff",
        type=Path,
        help="focus on a diff, copied into owned evidence before execution",
    )
    args = parser.parse_args()
    if sys.version_info < (3, 11):
        parser.error("requires Python 3.11+ (3.13+ for bounded QA on macOS)")
    try:
        config.prerequisites()
    except ValueError as error:
        parser.error(str(error))
    if (
        not 1 <= args.jobs <= (os.cpu_count() or 1)
        or min(args.disk_limit_mib, args.timeout) <= 0
        or args.test_threads is not None and not 1 <= args.test_threads <= (os.cpu_count() or 1)
    ):
        parser.error(
            "positive resource limits and jobs no greater than available CPUs are required"
        )
    if args.test_tool == "cargo" and args.test_threads is not None:
        parser.error("--test-threads selects Nextest slots; Cargo recursive children remain serial")
    test_threads = args.test_threads or max(1, (os.cpu_count() or 1) // args.jobs)
    overrides = [
        key
        for key in (
            "CARGO_TARGET_DIR",
            "CARGO_BUILD_TARGET_DIR",
            "CARGO_BUILD_BUILD_DIR",
        )
        if os.environ.get(key)
    ]
    overrides.extend(sorted(key for key in os.environ if key.startswith("HTMLCUT_")))
    if overrides:
        parser.error(
            "unset shared build overrides and reserved HTMLCUT_* test controls: "
            + ", ".join(overrides)
        )
    try:
        source = args.dir.resolve(strict=True)
        diff = args.in_diff.resolve(strict=True) if args.in_diff else None
        if not source.is_dir() or diff and not diff.is_file():
            raise ValueError(
                "crate directory and an existing regular diff file are required"
            )
        if diff and diff.stat().st_size >= args.disk_limit_mib * 1024 * 1024:
            raise ValueError("diff input exceeds the selected disk budget")
        metadata = subprocess.run(
            [
                "cargo",
                "metadata",
                "--no-deps",
                "--format-version",
                "1",
                "--locked",
                "--offline",
                "--manifest-path",
                str(source / "Cargo.toml"),
            ],
            check=True,
            capture_output=True,
            text=True,
            timeout=30,
        )
        workspace_metadata = json.loads(metadata.stdout)
        members = set(workspace_metadata["workspace_members"])
        packages_by_id = {package["id"]: package for package in workspace_metadata["packages"]}
        if not members or not members <= packages_by_id.keys():
            raise ValueError("workspace test-package discovery is incomplete")
        test_packages = sorted(
            packages_by_id[member]["name"] + "@" + packages_by_id[member]["version"]
            for member in members
        )
        workspace = Path(workspace_metadata["workspace_root"]).resolve(
            strict=True
        )
        output = args.output.resolve()
        if output.exists() or output == workspace or workspace in output.parents:
            raise ValueError(
                "output must be a new directory outside the actual workspace"
            )
        if not output.parent.is_dir():
            raise ValueError("output parent must be an existing directory")
        if (
            shutil.disk_usage(output.parent).free
            < max(args.disk_limit_mib, 4) * 1024 * 1024
        ):
            raise ValueError("insufficient free space for the selected disk budget")
        before = source_identity(workspace)
        version = config.tools()["tools"]["cargo-mutants"]["version"]
    except subprocess.CalledProcessError as error:
        parser.error(
            error.stderr.strip()
            if isinstance(error.stderr, str) and error.stderr.strip()
            else str(error)
        )
    except (OSError, ValueError, KeyError, subprocess.SubprocessError) as error:
        parser.error(str(error))
    report = {
        "scope": "focused"
        if args.file or args.exclude_file or args.regex or diff
        else "full workspace",
        "workspace": str(workspace),
        "execution_complete": False,
        "full_review_complete": False,
        "review_required": True,
        "jobs": args.jobs,
        "test_tool": args.test_tool,
        "test_threads": test_threads if args.test_tool == "nextest" else 1,
        "test_scope": "workspace binary tests via nextest; doctests baseline-only" if args.test_tool == "nextest" else "workspace Cargo tests including doctests per mutant",
        "disk_limit_mib": args.disk_limit_mib,
        "host": platform.platform(),
        "source_before": before,
        "exclude_files": args.exclude_file,
    }
    output.mkdir()
    scratch = output / "scratch"
    scratch.mkdir()
    status = 125
    resources = None
    cleanup_errors = []
    report.update(
        tool_returncode=None, tool_version_expected="cargo-mutants " + version
    )
    (output / "audit.json").write_text(
        json.dumps(report, indent=2) + "\n", encoding="utf-8"
    )
    try:
        command = [
            "cargo",
            "mutants",
            "--dir",
            str(workspace),
            "--workspace",
            "--all-features",
            "--test-workspace",
            "true",
            "--no-config",
            "--copy-target",
            "false",
            "--copy-vcs",
            "false",
            "--jobs",
            str(args.jobs),
            "--jobserver-tasks",
            str(os.cpu_count() or 1),
            "--baseline",
            "run",
            "--cargo-arg=--locked",
            "--cargo-arg=--offline",
            "--cargo-arg=--target-dir=target",
            '--cargo-arg=--config=build.build-dir="target"',
            "--build-timeout",
            "300",
            "--timeout",
            "180",
            "--output",
            str(output),
        ]
        # cargo-mutants 27.1 baselines otherwise select only mutated packages,
        # even when mutant scenarios use --test-workspace=true.
        command += ["--cargo-arg=--package=" + package for package in test_packages]
        report["test_packages"] = test_packages
        if args.test_tool == "nextest":
            profile = output / "nextest.toml"
            # Priorities apply only to packages present in the selected workspace.
            # They never filter test membership, including in standalone tool fixtures.
            sections = (ROOT / ".config/nextest.toml").read_text().split("[[profile.mutation.overrides]]")
            packages = {package["name"] for package in workspace_metadata.get("packages", [])}
            text = sections[0]
            for section in sections[1:]:
                selected = re.search(r"package\(=([^)]*)\)", section)
                if selected is None:
                    raise ValueError("mutation profile priorities must select a package")
                if selected[1] in packages:
                    text += "[[profile.mutation.overrides]]" + section
            profile.write_text(text)
            report["runner_profile"] = {"path": str(profile), "sha256": file_digest(profile)}
            command += ["--test-tool", "nextest", "--leak-dirs",
                        "--cargo-arg=--config-file=" + str(profile),
                        "--cargo-arg=--profile=mutation", "--cargo-arg=--color=never",
                        "--cargo-arg=--user-config-file=none"]
            for argument in ("--fail-fast", "--retries=0", "--no-tests=fail", "--ignore-default-filter",
                             "--message-format=libtest-json-plus", "--message-format-version=0.1",
                             "--status-level=fail", "--final-status-level=fail",
                             "--show-progress=none",
                             "--failure-output=never", "--success-output=never",
                             "--test-threads=" + str(test_threads)):
                command.append("--cargo-test-arg=" + argument)
        for pattern in args.file:
            command.extend(["--file", pattern])
        for pattern in args.exclude_file:
            command.extend(["--exclude", pattern])
        if args.regex:
            command.extend(["--re", args.regex])
        if diff:
            staged = output / "selection.diff"
            shutil.copyfile(diff, staged)
            report["selection_diff"] = {
                "original": str(diff),
                "sha256": file_digest(staged),
            }
            command.extend(["--in-diff", str(staged)])
        report["command"] = command
        env = {
            **os.environ,
            "TMPDIR": str(scratch),
            "TMP": str(scratch),
            "TEMP": str(scratch),
            "CARGO_INCREMENTAL": "0",
            "CARGO_PROFILE_DEV_DEBUG": "0",
            "CARGO_PROFILE_TEST_DEBUG": "0",
            "CARGO_TERM_COLOR": "never",
        }
        env["RUST_TEST_THREADS"] = "1"
        if args.test_tool == "nextest":
            env["NEXTEST_DOUBLE_SPAWN"] = "0"
            env["NEXTEST_EXPERIMENTAL_LIBTEST_JSON"] = "1"
        report["execution_environment"] = {
            key: env.get(key)
            for key in (
                "RUST_TEST_THREADS",
                "CARGO_BUILD_TARGET",
                "CARGO_BUILD_JOBS",
                "RUSTUP_TOOLCHAIN",
                "CARGO_INCREMENTAL",
                "CARGO_PROFILE_DEV_DEBUG",
                "CARGO_PROFILE_TEST_DEBUG",
                "NEXTEST_DOUBLE_SPAWN",
                "NEXTEST_EXPERIMENTAL_LIBTEST_JSON",
            )
        }
        (output / "audit.json").write_text(
            json.dumps(report, indent=2) + "\n", encoding="utf-8"
        )

        def interrupt(*_):
            raise KeyboardInterrupt

        signal.signal(signal.SIGTERM, interrupt)
        probe_root = output / "tool-version"
        probe_root.mkdir()
        probe_env = {
            **env,
            "TMPDIR": str(probe_root),
            "TMP": str(probe_root),
            "TEMP": str(probe_root),
        }
        probe, _ = guarded_phase(
            ["cargo", "mutants", "--version"],
            probe_root,
            "version",
            disk_limit_mib=4,
            timeout=10,
            log_limit_mib=1,
            env=probe_env,
        )
        report["tool_version"] = probe
        probe["expected"] = "cargo-mutants " + version
        cleanup_errors.extend(
            "tool version: " + error for error in probe["cleanup_errors"]
        )
        if probe["cleanup_errors"]:
            report["retained_phase_roots"] = [str(probe_root)]
        if probe["returncode"] == 130:
            status = 130
        elif (
            probe["returncode"] != 0
            or probe["tool_returncode"] != 0
            or probe["refusal"] is not None
        ):
            report["tool_version_error"] = (
                "Pinned tool probe failed (exit "
                + str(probe["tool_returncode"])
                + ", refusal "
                + str(probe["refusal"])
                + "); inspect "
                + probe["log_path"]
            )
        else:
            actual = Path(probe["log_path"]).read_text(encoding="utf-8").strip()
            probe["actual_preview"] = actual[:512]
            if actual != probe["expected"]:
                report["tool_version_error"] = (
                    "Expected "
                    + probe["expected"]
                    + "; observed "
                    + json.dumps(actual[:512])
                    + "; install the pinned tool: cargo install cargo-mutants --version "
                    + version
                    + " --locked; bounded probe log: "
                    + probe["log_path"]
                )
            else:
                started = time.monotonic()
                if args.test_tool == "nextest":
                    nextest_root = output / "nextest-version"
                    nextest_root.mkdir()
                    nextest, _ = guarded_phase(["cargo", "nextest", "--version"], nextest_root,
                                                "version", disk_limit_mib=4, timeout=10,
                                                log_limit_mib=1,
                                                env={**env, "TMPDIR": str(nextest_root), "TMP": str(nextest_root), "TEMP": str(nextest_root)})
                    report["nextest_version"] = nextest
                    expected = config.tools()["tools"]["cargo-nextest"]["version"]
                    cleanup_errors.extend(nextest["cleanup_errors"])
                    if nextest["cleanup_errors"]:
                        report.setdefault("retained_phase_roots", []).append(str(nextest_root))
                    if nextest["returncode"] == 130:
                        raise KeyboardInterrupt
                    text = Path(nextest["log_path"]).read_text(encoding="utf-8") if not nextest["returncode"] else ""
                    if (nextest["returncode"] or nextest["cleanup_errors"]
                        or not install.version_matches("cargo-nextest", text)):
                        raise ValueError(
                            "Pinned Nextest version refused; install the pinned tool: cargo install cargo-nextest --version "
                            + expected + " --locked; retained probe evidence: " + str(nextest_root)
                        )
                    report["nextest_version"]["release"] = expected
                    started = time.monotonic()
                    report["doctest_baseline"] = doctest_baseline(workspace, output, before, args, env, started)
                    if report["doctest_baseline"]["returncode"] == 130:
                        raise KeyboardInterrupt
                    if report["doctest_baseline"]["returncode"] or report["doctest_baseline"]["cleanup_errors"]:
                        cleanup_errors.extend(report["doctest_baseline"]["cleanup_errors"])
                        raise ValueError("same-source doctest baseline refused; retained " + str(output / "doctest-baseline"))
                remaining = args.timeout - (time.monotonic() - started)
                if remaining <= 0:
                    raise ValueError("campaign budget expired before mutation execution")
                report["campaign_started"] = True
                phase, resources = guarded_phase(
                    command,
                    output,
                    "mutation",
                    disk_limit_mib=args.disk_limit_mib,
                    timeout=remaining,
                    log_limit_mib=8,
                    env=env,
                )
                report.update({key: phase.get(key) for key in ("tool_returncode",)})
                if "execution_error" in phase:
                    report["execution_error"] = phase["execution_error"]
                cleanup_errors.extend(phase["cleanup_errors"])
                if phase["cleanup_errors"]:
                    report.setdefault("retained_phase_roots", []).append(str(output))
                status = phase["returncode"]
    except KeyboardInterrupt:
        status = 130
    except (OSError, ValueError, subprocess.SubprocessError) as error:
        report["execution_error"] = str(error)
    if cleanup_errors and status != 130:
        status = 125
    if args.test_tool == "nextest" and report.get("campaign_started") and status not in (0, 2, 3):
        report["descendant_cleanup_uncertain"] = True
    if resources is not None and status in (0, 2, 3):
        try:
            if (
                resources.get("refusal") is not None
                or resources["returncode"] != status
            ):
                raise ValueError("resource owner and mutation status disagree")
            report.update(complete_outcomes(output, status, version, args.test_tool,
                                            report.get("doctest_baseline", {}).get("tests")))
            report["execution_complete"] = True
        except (OSError, ValueError, KeyError, TypeError) as error:
            report["evidence_error"] = str(error)
            if args.test_tool == "nextest":
                report["descendant_cleanup_uncertain"] = True
            status = 125
    try:
        after = source_identity(workspace)
        report["source_after"] = after
        report["source_stable"] = before == after
        if before != after:
            report["changed_source_files"] = [
                name
                for name in sorted(set(before["files"]) | set(after["files"]))
                if before["files"].get(name) != after["files"].get(name)
            ]
            status = 125
    except (OSError, ValueError, subprocess.SubprocessError) as error:
        report["source_error"] = str(error)
        report["source_stable"] = False
        status = 125
    if not report.get("source_stable") or status == 125:
        report["execution_complete"] = False
    if not cleanup_errors and not report.get("descendant_cleanup_uncertain"):
        try:
            shutil.rmtree(scratch)
        except OSError as error:
            cleanup_errors.append("owned scratch removal failed: " + str(error))
            status = 125
            report["execution_complete"] = False
    if cleanup_errors or report.get("descendant_cleanup_uncertain"):
        report["cleanup_errors"] = cleanup_errors
        report["retained_scratch"] = str(scratch)
    report["returncode"] = status
    (output / "audit.json").write_text(
        json.dumps(report, indent=2) + "\n", encoding="utf-8"
    )
    if report.get("tool_version_error"):
        print(report["tool_version_error"], file=sys.stderr)
    if report.get("execution_error"):
        print(report["execution_error"], file=sys.stderr)
    print("Mutation evidence: " + str(output), file=sys.stderr)
    return status


if __name__ == "__main__":
    sys.exit(main())
