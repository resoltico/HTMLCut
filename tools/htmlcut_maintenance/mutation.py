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
import shutil
import signal
import stat
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[2]
from tools.htmlcut_maintenance import config, process

VERDICTS = {
    "CaughtMutant": "caught",
    "MissedMutant": "missed",
    "Timeout": "timeout",
    "Unviable": "unviable",
}


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


def validate_phases(outcome, baseline=False):
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
    failure = lambda value: (
        isinstance(value, dict)
        and set(value) == {"Failure"}
        and type(value["Failure"]) is int
        and value["Failure"] != 0
    )
    valid = actual == [("Build", "Success"), ("Test", "Success")] if baseline else False
    if not baseline:
        valid = (
            summary == "CaughtMutant"
            and len(actual) == 2
            and actual[0] == ("Build", "Success")
            and actual[1][0] == "Test"
            and failure(actual[1][1])
            or summary == "MissedMutant"
            and actual == [("Build", "Success"), ("Test", "Success")]
            or summary == "Unviable"
            and len(actual) == 1
            and actual[0][0] == "Build"
            and failure(actual[0][1])
            or summary == "Timeout"
            and actual
            in [[("Build", "Timeout")], [("Build", "Success"), ("Test", "Timeout")]]
        )
    if not valid or baseline and summary != "Success":
        raise ValueError("inconsistent scenario phases: " + str(summary))


def complete_outcomes(output, tool_status, version):
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
            validate_phases(item, baseline=True)
        elif isinstance(scenario, dict) and set(scenario) == {"Mutant"}:
            mutant = scenario["Mutant"]
            name = mutant["name"]
            if name in tested or planned.get(name) != mutant:
                raise ValueError("duplicate, unplanned or changed mutant: " + str(name))
            if item["summary"] not in VERDICTS:
                raise ValueError("invalid mutant verdict: " + str(item["summary"]))
            validate_phases(item)
            evidence_file(root, item["diff_path"])
            tested[name] = item
            counts[VERDICTS[item["summary"]]] += 1
        else:
            raise ValueError("invalid mutation scenario")
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
            poll_seconds=0.2,
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
        refusal=refusal,
        cleanup_errors=errors,
        returncode=status,
    )
    return details, resources


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
    ):
        parser.error(
            "positive resource limits and jobs no greater than available CPUs are required"
        )
    overrides = [
        key
        for key in (
            "CARGO_TARGET_DIR",
            "CARGO_BUILD_TARGET_DIR",
            "CARGO_BUILD_BUILD_DIR",
        )
        if os.environ.get(key)
    ]
    if overrides:
        parser.error("unset shared Cargo build overrides: " + ", ".join(overrides))
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
        workspace = Path(json.loads(metadata.stdout)["workspace_root"]).resolve(
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
            "60",
            "--output",
            str(output),
        ]
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
        env.setdefault("RUST_TEST_THREADS", "1")
        report["execution_environment"] = {
            key: env.get(key)
            for key in (
                "RUST_TEST_THREADS",
                "CARGO_BUILD_TARGET",
                "RUSTUP_TOOLCHAIN",
                "CARGO_INCREMENTAL",
                "CARGO_PROFILE_DEV_DEBUG",
                "CARGO_PROFILE_TEST_DEBUG",
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
                phase, resources = guarded_phase(
                    command,
                    output,
                    "mutation",
                    disk_limit_mib=args.disk_limit_mib,
                    timeout=args.timeout,
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
    finally:
        if not cleanup_errors:
            try:
                shutil.rmtree(scratch)
            except OSError as error:
                cleanup_errors.append("owned scratch removal failed: " + str(error))
        if cleanup_errors:
            report["cleanup_errors"] = cleanup_errors
            report["retained_scratch"] = str(scratch)
    if cleanup_errors and status != 130:
        status = 125
    if resources is not None and status in (0, 2, 3):
        try:
            if (
                resources.get("refusal") is not None
                or resources["returncode"] != status
            ):
                raise ValueError("resource owner and mutation status disagree")
            report.update(complete_outcomes(output, status, version))
            report["execution_complete"] = True
        except (OSError, ValueError, KeyError, TypeError) as error:
            report["evidence_error"] = str(error)
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
    report["returncode"] = status
    (output / "audit.json").write_text(
        json.dumps(report, indent=2) + "\n", encoding="utf-8"
    )
    if report.get("tool_version_error"):
        print(report["tool_version_error"], file=sys.stderr)
    print("Mutation evidence: " + str(output), file=sys.stderr)
    return status


if __name__ == "__main__":
    sys.exit(main())
