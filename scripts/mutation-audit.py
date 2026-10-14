#!/usr/bin/env python3
# SPDX-License-Identifier: MPL-2.0
"""Run cargo-mutants in disposable build roots, retaining bounded audit evidence."""
import argparse
import json
import os
from pathlib import Path
import shutil
import signal
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True, help="new directory outside the checkout")
    parser.add_argument("--dir", type=Path, default=ROOT)
    parser.add_argument("--jobs", type=int, default=min(2, os.cpu_count() or 1))
    parser.add_argument("--disk-limit-mib", type=int, default=4096)
    parser.add_argument("--timeout", type=int, default=1800, help="total wall time, including baseline")
    parser.add_argument("--file", action="append", default=[])
    parser.add_argument("--regex")
    parser.add_argument("--in-diff", type=Path)
    args = parser.parse_args()
    if sys.version_info < (3, 11):
        parser.error("requires Python 3.11 or newer")
    source = args.dir.resolve(strict=True)
    output = args.output.resolve()
    if output.exists():
        parser.error("output must be a new directory; existing evidence is preserved")
    if output == source or source in output.parents:
        parser.error("output must be outside the source checkout")
    if args.jobs < 1 or args.jobs > min(2, os.cpu_count() or 1):
        parser.error("jobs must be between 1 and min(2, available CPUs)")
    if args.disk_limit_mib <= 0 or args.timeout <= 0:
        parser.error("resource limits must be positive")
    # Do not override a shared build root silently: it could corrupt owner caches.
    overrides = [key for key in ("CARGO_TARGET_DIR", "CARGO_BUILD_TARGET_DIR", "CARGO_BUILD_BUILD_DIR")
                 if os.environ.get(key)]
    if overrides:
        parser.error("unset shared Cargo build overrides: " + ", ".join(overrides))
    inventory_env = {**os.environ, "PATH": str(Path(sys.executable).resolve().parent) + os.pathsep + os.environ.get("PATH", "")}
    version = subprocess.run(["bash", "-c",
        'set -euo pipefail; source ./scripts/contributor-rust-tools.sh; htmlcut_contributor_cargo_tool_inventory | awk \'$1 == "cargo-mutants" {print $2}\''],
        cwd=ROOT, env=inventory_env, check=True, capture_output=True, text=True).stdout.strip()
    tool = subprocess.run(["cargo", "mutants", "--version"], capture_output=True, text=True)
    if tool.returncode != 0:
        parser.error("install cargo-mutants with cargo install cargo-mutants --version " + version + " --locked")
    actual = tool.stdout.strip()
    if actual != "cargo-mutants " + version:
        parser.error("install cargo-mutants " + version + " with --locked; got " + actual)
    if not output.parent.is_dir():
        parser.error("output parent must be an existing directory")
    if shutil.disk_usage(output.parent).free < args.disk_limit_mib * 1024 * 1024:
        parser.error("insufficient free space for the selected disk budget")
    output.mkdir()  # Never reuse or rotate an owner's results.
    scratch = output / "scratch"
    scratch.mkdir()
    env = {**os.environ, "TMPDIR": str(scratch), "TMP": str(scratch), "TEMP": str(scratch),
           "CARGO_INCREMENTAL": "0", "CARGO_PROFILE_DEV_DEBUG": "0", "CARGO_PROFILE_TEST_DEBUG": "0",
           "CARGO_TERM_COLOR": "never"}
    command = ["cargo", "mutants", "--dir", str(source), "--workspace", "--all-features",
               "--test-workspace", "true", "--no-config", "--copy-target", "false", "--copy-vcs", "false",
               "--jobs", str(args.jobs), "--jobserver-tasks", str(os.cpu_count() or 1),
               "--baseline", "run", "--cargo-arg=--locked", "--cargo-arg=--offline",
               "--cargo-arg=--target-dir=target", '--cargo-arg=--config=build.build-dir="target"',
               "--build-timeout", "300", "--timeout", "60", "--output", str(output)]
    for pattern in args.file:
        command.extend(["--file", pattern])
    if args.regex:
        command.extend(["--re", args.regex])
    if args.in_diff:
        command.extend(["--in-diff", str(args.in_diff.resolve(strict=True))])
    report = {"scope": "focused" if args.file or args.regex or args.in_diff else "full workspace",
              "full_review_complete": False, "jobs": args.jobs,
              "disk_limit_mib": args.disk_limit_mib, "command": command}
    (output / "audit.json").write_text(json.dumps(report, indent=2) + "\n")
    def interrupt(*_):
        raise KeyboardInterrupt

    signal.signal(signal.SIGTERM, interrupt)
    process = None
    try:
        process = subprocess.Popen([sys.executable, str(ROOT / "scripts/run-resource-bounded.py"),
            "--root", str(output), "--disk-limit-mib", str(args.disk_limit_mib),
            "--timeout", str(args.timeout), "--poll-seconds", "0.2", "--label", "mutation", "--", *command], env=env)
        status = process.wait()
    except KeyboardInterrupt:
        if process:
            process.send_signal(signal.SIGTERM)
            process.wait()
        status = 130
    finally:
        # Cargo-mutants owns only these TMP roots; results remain outside them.
        shutil.rmtree(scratch)
    if status == 0:
        try:
            outcomes = json.loads((output / "mutants.out/outcomes.json").read_text())
            mutants = [item for item in outcomes["outcomes"] if isinstance(item["scenario"], dict)]
            baseline = [item for item in outcomes["outcomes"] if item["scenario"] == "Baseline"]
            if outcomes["total_mutants"] < 1 or len(mutants) != outcomes["total_mutants"]:
                raise ValueError("empty or incomplete mutant outcomes")
            if len(baseline) != 1 or baseline[0]["summary"] != "Success":
                raise ValueError("missing or unsuccessful unmutated baseline")
            report["tested_mutants"] = len(mutants)
        except (OSError, ValueError, KeyError, TypeError) as error:
            report["evidence_error"] = str(error)
            status = 125
    report["returncode"] = status
    # An exit code does not establish review: survivors/timeouts need examination.
    report["review_required"] = True
    (output / "audit.json").write_text(json.dumps(report, indent=2) + "\n")
    print("Mutation evidence: " + str(output), file=sys.stderr)
    return status


if __name__ == "__main__":
    sys.exit(main())
