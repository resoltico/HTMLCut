#!/usr/bin/env python3
# SPDX-License-Identifier: MPL-2.0
"""Own finite fuzz builds, sequential harnesses and their retained evidence."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import signal
import stat
import subprocess
import sys
import tempfile
import threading
from tools.htmlcut_maintenance import config, process
from tools.htmlcut_maintenance.mutation import source_identity
import time
import tomllib

ROOT = Path(__file__).resolve().parents[2]
MAXIMUM_SEEDS = 10000
MAXIMUM_INPUT = 65536
EXTRACT_FRAMING = "mode:u8, contract_bytes:u16le, contract:utf8, document:bytes"


class Refusal(Exception):
    def __init__(self, message, status=1):
        super().__init__(message)
        self.status = status


def run_count(value):
    if not re.fullmatch(r"[1-9][0-9]{0,4}", value) or int(value) > 10000:
        raise argparse.ArgumentTypeError("--runs must be an integer from 1 to 10000")
    return int(value)


def fuzz_targets():
    with (ROOT / "fuzz/Cargo.toml").open("rb") as source:
        return [target["name"] for target in tomllib.load(source)["bin"]]


def seed_size(path, remaining=MAXIMUM_INPUT):
    metadata = path.stat()
    if not stat.S_ISREG(metadata.st_mode):
        raise Refusal(f"Fuzz seed {path} must be an ordinary regular file")
    if metadata.st_size > remaining:
        raise Refusal(
            f"Fuzz seed {path} exceeds the smoke input bound of {MAXIMUM_INPUT} bytes"
        )
    return metadata.st_size


def read_seed(path, remaining=MAXIMUM_INPUT):
    seed_size(path, remaining)
    with path.open("rb") as source:
        opened = os.fstat(source.fileno())
        if not stat.S_ISREG(opened.st_mode):
            raise Refusal(f"Fuzz seed {path} must be an ordinary regular file")
        value = source.read(remaining + 1)
    if len(value) > remaining:
        raise Refusal(
            f"Fuzz seed {path} exceeds the smoke input bound of {MAXIMUM_INPUT} bytes"
        )
    return value


def seed_paths(directory, documents=False):
    paths = []
    for path in directory.iterdir():
        if documents and path.is_dir():
            continue
        if len(paths) >= MAXIMUM_SEEDS:
            raise Refusal(
                f"{directory} exceeds the bounded initial corpus of 10000 seeds"
            )
        paths.append(path)
    return sorted(paths)


def seed_sources():
    cases = seed_paths(ROOT / "conformance/contract")
    if not cases:
        raise Refusal("The conformance seed corpus is empty")
    documents = seed_paths(ROOT / "fuzz/seeds/documents", documents=True)
    if len(cases) + len(documents) > MAXIMUM_SEEDS:
        raise Refusal("extract exceeds the bounded initial corpus of 10000 seeds")
    # Reject nonregular/oversized sources before source fingerprinting opens them.
    for case in cases:
        contract_bytes = seed_size(case / "contract.json", MAXIMUM_INPUT - 3)
        seed_size(case / "page.html", MAXIMUM_INPUT - 3 - contract_bytes)
    for path in documents:
        seed_size(path, MAXIMUM_INPUT - 3 - len(b'{"select":":scope"}'))
    return cases, documents


def prepare_seeds(output, targets, sources=None):
    inventory = {"contract_bytes": [], "extract": []}
    identities = {target: {} for target in inventory}
    if set(targets) != inventory.keys():
        raise Refusal("Fuzz target manifest and corpus formats disagree")
    cases, documents = seed_sources() if sources is None else sources
    for target in targets:
        (output / target / "corpus").mkdir(parents=True)
        (output / target / "crashes").mkdir()

    def write_seed(target, name, value, source=None):
        source = name if source is None else source
        digest = hashlib.sha256(value).hexdigest()
        if digest in identities[target]:
            identities[target][digest]["sources"].append(source)
            return
        with (output / target / "corpus" / name).open("xb") as destination:
            destination.write(value)
        record = {
            "name": name,
            "bytes": len(value),
            "sha256": digest,
            "sources": [source],
        }
        inventory[target].append(record)
        identities[target][digest] = record

    for case in cases:
        contract = read_seed(case / "contract.json", MAXIMUM_INPUT - 3)
        page = read_seed(case / "page.html", MAXIMUM_INPUT - 3 - len(contract))
        write_seed("contract_bytes", case.name, contract)
        write_seed(
            "extract",
            case.name,
            b"\0" + len(contract).to_bytes(2, "little") + contract + page,
        )
    # Exercise the small-reading semantic policy during initialization as well
    # as through mutations, without changing any authored source bytes.
    first = cases[0]
    contract = read_seed(first / "contract.json", MAXIMUM_INPUT - 3)
    page = read_seed(first / "page.html", MAXIMUM_INPUT - 3 - len(contract))
    write_seed(
        "extract",
        "small-reading-" + first.name,
        b"\2" + len(contract).to_bytes(2, "little") + contract + page,
        source=first.name,
    )
    contract = b'{"select":":scope"}'
    for index, path in enumerate(documents):
        page = read_seed(path, MAXIMUM_INPUT - 3 - len(contract))
        write_seed(
            "extract",
            "document-" + path.name,
            bytes([1 | ((index % 33) << 2)])
            + len(contract).to_bytes(2, "little")
            + contract
            + page,
        )
    (output / "seeds.json").write_text(json.dumps(inventory, indent=2) + "\n")
    return inventory


def cleanup_confirmed(report):
    try:
        result = json.loads(report.read_text())
        errors = result["cleanup_errors"]
        if type(result["returncode"]) is not int:
            raise ValueError("child returncode is not confirmed")
        if not isinstance(errors, list) or errors:
            raise ValueError(f"cleanup_errors={errors!r}")
    except (OSError, ValueError, KeyError, TypeError) as error:
        print(f"Unconfirmed process cleanup in {report}: {error}", file=sys.stderr)
        return False
    return True


def harness_path(directory, target):
    return directory / (target + (".exe" if os.name == "nt" else ""))


def harness_directory(build, targets):
    owned = (build / "target").resolve()
    if not owned.is_relative_to(build.resolve(strict=True)):
        raise Refusal("Fuzz target directory must remain in the owned build directory")
    candidates = []
    for release in (build / "target").glob("*/release"):
        if not all(harness_path(release, target).is_file() for target in targets):
            continue
        if any(
            not harness_path(release, target).resolve().is_relative_to(owned)
            for target in targets
        ):
            raise Refusal("Fuzz harness paths must remain in the owned build directory")
        if not all(
            os.access(harness_path(release, target), os.X_OK) for target in targets
        ):
            raise Refusal("Fuzz harness files must be executable")
        candidates.append(release)
    if len(candidates) != 1:
        raise Refusal(
            "Expected exactly one owned release directory containing every fuzz harness"
        )
    return candidates[0]


def verify_progress(output, targets, runs, require_mutations):
    progress = {}
    for target in targets:
        text = (output / target / "run.log").read_text(encoding="utf-8")
        completed = re.search(r"Done (\d+) runs", text)
        if completed is None or int(completed[1]) < runs:
            raise Refusal(f"{target} did not complete all {runs} iterations")
        initialized = re.search(r"^#(\d+)\s+INITED\b", text, re.MULTILINE)
        if require_mutations:
            if initialized is None or int(completed[1]) <= int(initialized[1]):
                raise Refusal(
                    f"{target} completed no mutation after seed initialization; increase --runs explicitly"
                )
        progress[target] = {
            "initialized": int(initialized[1]) if initialized else None,
            "completed": int(completed[1]),
            "post_initialization": (
                max(0, int(completed[1]) - int(initialized[1])) if initialized else None
            ),
        }
    return progress


def verify_source(expected, workspace):
    observed = source_identity(workspace)
    if observed != expected:
        raise Refusal(
            "Fuzz source/lock inputs changed; retained evidence does not establish a stable-source run"
        )
    return {"git_commit": observed["git_commit"], "sha256": observed["sha256"]}


def source_state(workspace, identity):
    if identity["git_commit"] is None:
        return {"kind": "standalone"}
    status = process.capture(
        ["git", "status", "--porcelain=v1", "--untracked-files=all"],
        cwd=workspace,
        maximum_mib=1,
        timeout=10,
    )
    return {"kind": "git", "dirty": bool(status.strip())}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--runs", type=run_count, default=10000)
    parser.add_argument("--require-mutations", action="store_true")
    parser.add_argument("output_directory", nargs="?", type=Path)
    args = parser.parse_args()
    try:
        # Custom/ignored SIGCHLD could asynchronously reap a live Popen child.
        # The shared watchdog has the stronger nonreaping group-ownership check.
        config.prerequisites()
        toolchain = config.tools()["nightly"]
        targets = fuzz_targets()
        scratch_parent = Path(
            os.environ.get("TMPDIR") or tempfile.gettempdir()
        ).resolve()
        if scratch_parent.is_relative_to(ROOT):
            raise ValueError("Fuzz scratch must be outside the checkout")
        parent = (
            args.output_directory.resolve() if args.output_directory else scratch_parent
        )
        if parent.is_relative_to(ROOT):
            raise ValueError("Fuzz output must be outside the checkout")
        parent.mkdir(parents=True, exist_ok=True)
    except (OSError, ValueError) as error:
        print(str(error), file=sys.stderr)
        return 2

    output = Path(
        tempfile.mkdtemp(
            prefix="run." if args.output_directory else "htmlcut-fuzz.", dir=parent
        )
    )
    try:
        build = Path(tempfile.mkdtemp(prefix="htmlcut-fuzz-build.", dir=scratch_parent))
    except OSError as error:
        try:
            output.rmdir()
        except OSError as cleanup_error:
            print(
                f"Cannot remove unused fuzz output {output}: {cleanup_error}",
                file=sys.stderr,
            )
        print(f"Cannot create owned fuzz build directory: {error}", file=sys.stderr)
        return 1
    build_started = False
    workers_started = []
    cancelled = 0
    cancellation = threading.Event()

    def cancel(signum, _):
        nonlocal cancelled
        cancelled = 128 + signum
        cancellation.set()

    signal.signal(signal.SIGINT, cancel)
    signal.signal(signal.SIGTERM, cancel)

    def start(root, disk, timeout, label, command, env):
        report = process.run(
            command,
            root,
            disk_limit_mib=disk,
            timeout=timeout,
            log_limit_mib=4,
            label=label,
            env=env,
            cwd=ROOT,
            cancelled=cancellation,
        )
        status = process.status(report)
        if status:
            raise Refusal("Bounded fuzz phase failed", cancelled or status)

    status = 125
    evidence = {
        "extract_framing": EXTRACT_FRAMING,
        "requested_runs": args.runs,
        "require_mutations": args.require_mutations or args.runs == 10000,
        "commands": {},
        "scope": "full_smoke" if args.runs == 10000 else "short",
        "complete": False,
        "exit_status": None,
    }

    def preserve_run():
        nonlocal status
        evidence["complete"] = status == 0
        evidence["exit_status"] = status
        try:
            (output / "run.json").write_text(json.dumps(evidence, indent=2) + "\n")
        except OSError as error:
            print(f"Cannot preserve fuzz run evidence {output}: {error}", file=sys.stderr)
            if status == 0:
                status = 125

    try:
        sources = seed_sources()
        evidence["source"] = source_identity(ROOT)
        evidence["source_state"] = source_state(ROOT, evidence["source"])
        prepare_seeds(output, targets, sources)
        evidence["source_after_seeds"] = verify_source(evidence["source"], ROOT)
        (output / "run.json").write_text(json.dumps(evidence, indent=2) + "\n")
        env = {
            **os.environ,
            "CARGO_INCREMENTAL": "0",
            "CARGO_TARGET_DIR": str(build / "target"),
            "CARGO_BUILD_TARGET_DIR": str(build / "target"),
            "CARGO_BUILD_BUILD_DIR": str(build / "target"),
        }
        if cancelled:
            raise Refusal("Fuzz check interrupted", cancelled)
        build_started = True
        build_command = [
            "cargo",
            "+" + toolchain,
            "fuzz",
            "build",
            "--target-dir",
            str(build / "target"),
            "--codegen-units",
            "16",
        ]
        evidence["commands"]["build"] = build_command
        start(
            build,
            1024,
            600,
            "build",
            build_command,
            env,
        )
        evidence["source_after_build"] = verify_source(evidence["source"], ROOT)

        release = harness_directory(build, targets)
        for target in targets:
            if cancelled:
                raise Refusal("Fuzz check interrupted", cancelled)
            workers_started.append(target)
            command = [
                str(harness_path(release, target)),
                str(output / target / "corpus"),
                f"-runs={args.runs}",
                "-reload=0",
                f"-max_len={MAXIMUM_INPUT}",
                "-timeout=10",
                "-rss_limit_mb=2048",
                "-seed=1008",
                f"-artifact_prefix={output / target / 'crashes'}/",
            ]
            evidence["commands"][target] = command
            start(
                output / target,
                64,
                180,
                "run",
                command,
                env,
            )

        evidence["progress"] = verify_progress(
            output, targets, args.runs, args.require_mutations or args.runs == 10000
        )
        evidence["source_after_run"] = verify_source(evidence["source"], ROOT)
        for target, progress in evidence["progress"].items():
            print(
                f"{target}: {progress['completed']} executions; "
                f"{progress['initialized']} initialized; "
                f"{progress['post_initialization']} after initialization."
            )
        if args.runs != 10000:
            print(
                f"Short test only: {args.runs} runs per target; required smoke is 10000."
            )
        status = 0
    except Refusal as error:
        print(str(error), file=sys.stderr)
        status = error.status
    except (OSError, ValueError, subprocess.SubprocessError, StopIteration) as error:
        print(f"Fuzz check failed: {error}", file=sys.stderr)
        status = 1
    finally:
        uncertain = False
        filesystem_failed = False
        try:
            build_evidence = list(build.glob("build.*"))
        except OSError as error:
            print(
                f"Cannot inspect fuzz build evidence {build}: {error}", file=sys.stderr
            )
            filesystem_failed = True
            build_evidence = []
        for path in build_evidence:
            if not (output / path.name).exists():
                try:
                    shutil.copy2(path, output / path.name)
                except OSError as error:
                    print(
                        f"Cannot preserve fuzz build evidence {path}: {error}",
                        file=sys.stderr,
                    )
                    filesystem_failed = True
        build_safe = not build_started or cleanup_confirmed(
            build / "build.resources.json"
        )
        if build_safe and not uncertain and not filesystem_failed:
            try:
                shutil.rmtree(build)
            except OSError as error:
                print(
                    f"Cannot remove confirmed fuzz build {build}: {error}",
                    file=sys.stderr,
                )
                filesystem_failed = True
        else:
            if build_safe and not uncertain:
                print(
                    f"Fuzz build retained because evidence preservation failed: {build}",
                    file=sys.stderr,
                )
            else:
                print(
                    f"Fuzz build retained because process cleanup is unconfirmed: {build}",
                    file=sys.stderr,
                )
                uncertain = True
        for target in workers_started:
            if not cleanup_confirmed(output / target / "run.resources.json"):
                print(
                    f"Fuzz target retained because process cleanup is unconfirmed: {output / target}",
                    file=sys.stderr,
                )
                uncertain = True
        if (uncertain or filesystem_failed) and status == 0:
            status = 125
        if status == 0 and not args.output_directory:
            try:
                shutil.rmtree(output)
            except BaseException as error:
                status = 125
                preserve_run()
                if not isinstance(error, OSError):
                    raise
                print(
                    f"Cannot remove confirmed fuzz output {output}: {error}",
                    file=sys.stderr,
                )
                print(f"Fuzz evidence retained at {output}", file=sys.stderr)
        else:
            preserve_run()
            print(f"Fuzz evidence retained at {output}", file=sys.stderr)
    return status


if __name__ == "__main__":
    sys.exit(main())
