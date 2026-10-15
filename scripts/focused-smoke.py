#!/usr/bin/env python3
# SPDX-License-Identifier: MPL-2.0
"""Own finite fuzz builds, parallel harnesses and their retained evidence."""
import argparse
import json
import os
from pathlib import Path
import re
import runpy
import shutil
import signal
import stat
import subprocess
import sys
import tempfile
import time
import tomllib

ROOT = Path(__file__).resolve().parents[1]
WATCHDOG = ROOT / "scripts/run-resource-bounded.py"
MAXIMUM_SEEDS = 10000
MAXIMUM_INPUT = 4096


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


def read_seed(path, remaining=MAXIMUM_INPUT):
    metadata = path.stat()
    if not stat.S_ISREG(metadata.st_mode):
        raise Refusal(f"Fuzz seed {path} must be an ordinary regular file")
    if metadata.st_size > remaining:
        raise Refusal(f"Fuzz seed {path} exceeds the smoke input bound of 4096 bytes")
    with path.open("rb") as source:
        opened = os.fstat(source.fileno())
        if not stat.S_ISREG(opened.st_mode):
            raise Refusal(f"Fuzz seed {path} must be an ordinary regular file")
        value = source.read(remaining + 1)
    if len(value) > remaining:
        raise Refusal(f"Fuzz seed {path} exceeds the smoke input bound of 4096 bytes")
    return value


def seed_paths(directory, documents=False):
    paths = []
    for path in directory.iterdir():
        if documents and path.is_dir():
            continue
        if len(paths) >= MAXIMUM_SEEDS:
            raise Refusal(f"{directory} exceeds the bounded initial corpus of 10000 seeds")
        paths.append(path)
    return sorted(paths)


def prepare_seeds(output, targets):
    inventory = {"contract_bytes": [], "extract": []}
    if set(targets) != inventory.keys():
        raise Refusal("Fuzz target manifest and corpus formats disagree")
    cases = seed_paths(ROOT / "conformance/contract")
    documents = seed_paths(ROOT / "fuzz/seeds/documents", documents=True)
    if len(cases) + len(documents) > MAXIMUM_SEEDS:
        raise Refusal("extract exceeds the bounded initial corpus of 10000 seeds")
    for target in targets:
        (output / target / "corpus").mkdir(parents=True)
        (output / target / "crashes").mkdir()

    def write_seed(target, name, value):
        with (output / target / "corpus" / name).open("xb") as destination:
            destination.write(value)
        inventory[target].append({"name": name, "bytes": len(value)})

    for case in cases:
        contract = read_seed(case / "contract.json", MAXIMUM_INPUT - 2)
        page = read_seed(case / "page.html", MAXIMUM_INPUT - 2 - len(contract))
        write_seed("contract_bytes", case.name, contract)
        write_seed("extract", case.name, len(contract).to_bytes(2, "little") + contract + page)
    contract = b'{"select":":scope"}'
    for path in documents:
        page = read_seed(path, MAXIMUM_INPUT - 2 - len(contract))
        write_seed("extract", "document-" + path.name,
                   len(contract).to_bytes(2, "little") + contract + page)
    (output / "seeds.json").write_text(json.dumps(inventory, indent=2) + "\n")


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


def stop_children(children):
    """Only this thread owns these Popen objects; never signal a saved numeric PID."""
    uncertain = False
    for process in list(children):
        try:
            if os.name == "nt":
                if process.poll() is None:
                    # TerminateProcess would bypass the watchdog's handler.
                    # Its retained Popen handle keeps this process identity
                    # owned while taskkill stops the still-attached tree.
                    uncertain = True
                    subprocess.run(["taskkill", "/PID", str(process.pid), "/T", "/F"],
                                   stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
                                   check=True, timeout=5)
            else:
                # Popen checks its owned child's state before sending a signal.
                process.terminate()
        except (OSError, subprocess.SubprocessError) as error:
            print(f"Cannot request fuzz watchdog cleanup: {error}", file=sys.stderr)
            uncertain = True
    for deadline in [time.monotonic() + 15, None]:
        if deadline is None:
            uncertain = True
            print("Fuzz watchdog cleanup exceeded 15 seconds; reports are unconfirmed", file=sys.stderr)
            for process in list(children):
                try:
                    process.kill()
                except OSError as error:
                    print(f"Fuzz watchdog leader cleanup unconfirmed: {error}", file=sys.stderr)
            deadline = time.monotonic() + 2
        while children and time.monotonic() < deadline:
            for process in list(children):
                try:
                    completed = process.poll() is not None
                except OSError as error:
                    print(f"Cannot confirm fuzz watchdog termination: {error}", file=sys.stderr)
                    uncertain = True
                    completed = True
                if completed:
                    children.remove(process)
            if children:
                time.sleep(0.01)
        if not children:
            break
    if children:
        print("Fuzz watchdog leaders remain unconfirmed after termination", file=sys.stderr)
        children.clear()
    return uncertain


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
        if any(not harness_path(release, target).resolve().is_relative_to(owned) for target in targets):
            raise Refusal("Fuzz harness paths must remain in the owned build directory")
        if not all(os.access(harness_path(release, target), os.X_OK) for target in targets):
            raise Refusal("Fuzz harness files must be executable")
        candidates.append(release)
    if len(candidates) != 1:
        raise Refusal("Expected exactly one owned release directory containing every fuzz harness")
    return candidates[0]


def verify_progress(output, targets, runs, require_mutations):
    for target in targets:
        text = (output / target / "run.log").read_text(encoding="utf-8")
        completed = re.search(r"Done (\d+) runs", text)
        if completed is None or int(completed[1]) < runs:
            raise Refusal(f"{target} did not complete all {runs} iterations")
        if require_mutations:
            initialized = re.search(r"^#(\d+)\s+INITED\b", text, re.MULTILINE)
            if initialized is None or int(completed[1]) <= int(initialized[1]):
                raise Refusal(f"{target} completed no mutation after seed initialization; increase --runs explicitly")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--runs", type=run_count, default=10000)
    parser.add_argument("--require-mutations", action="store_true")
    parser.add_argument("output_directory", nargs="?", type=Path)
    args = parser.parse_args()
    try:
        # Custom/ignored SIGCHLD could asynchronously reap a live Popen child.
        # The shared watchdog has the stronger nonreaping group-ownership check.
        runpy.run_path(str(WATCHDOG))["check_process_ownership"]()
        toolchain = os.environ.get("HTMLCUT_CONTRIBUTOR_RUST_NIGHTLY_TOOLCHAIN")
        if not toolchain:
            raise ValueError("Run scripts/focused-smoke.sh to load the pinned contributor toolchain")
        targets = fuzz_targets()
        scratch_parent = Path(os.environ.get("TMPDIR") or tempfile.gettempdir()).resolve()
        if scratch_parent.is_relative_to(ROOT):
            raise ValueError("Fuzz scratch must be outside the checkout")
        parent = args.output_directory.resolve() if args.output_directory else scratch_parent
        if parent.is_relative_to(ROOT):
            raise ValueError("Fuzz output must be outside the checkout")
        parent.mkdir(parents=True, exist_ok=True)
    except (OSError, ValueError) as error:
        print(str(error), file=sys.stderr)
        return 2

    output = Path(tempfile.mkdtemp(prefix="run." if args.output_directory else "htmlcut-fuzz.", dir=parent))
    try:
        build = Path(tempfile.mkdtemp(prefix="htmlcut-fuzz-build.", dir=scratch_parent))
    except OSError as error:
        try:
            output.rmdir()
        except OSError as cleanup_error:
            print(f"Cannot remove unused fuzz output {output}: {cleanup_error}", file=sys.stderr)
        print(f"Cannot create owned fuzz build directory: {error}", file=sys.stderr)
        return 1
    children = []
    build_started = False
    workers_started = []
    cancelled = 0

    def cancel(signum, _):
        nonlocal cancelled
        cancelled = 128 + signum

    signal.signal(signal.SIGINT, cancel)
    signal.signal(signal.SIGTERM, cancel)

    def start(root, disk, timeout, label, command, env):
        process = subprocess.Popen([
            sys.executable, str(WATCHDOG), "--root", str(root), "--disk-limit-mib", str(disk),
            "--timeout", str(timeout), "--log-limit-mib", "4", "--label", label, "--", *command],
            cwd=ROOT, env=env, start_new_session=os.name != "nt")
        children.append(process)

    def wait_children():
        while children:
            if cancelled:
                raise Refusal("Fuzz check interrupted", cancelled)
            for process in list(children):
                status = process.poll()
                if status is not None:
                    children.remove(process)
                    if status:
                        raise Refusal("Fuzz watchdog failed", status if status > 0 else 128 - status)
            if children:
                time.sleep(0.05)

    status = 0
    try:
        prepare_seeds(output, targets)
        env = {**os.environ, "CARGO_INCREMENTAL": "0", "CARGO_TARGET_DIR": str(build / "target"),
               "CARGO_BUILD_TARGET_DIR": str(build / "target"), "CARGO_BUILD_BUILD_DIR": str(build / "target")}
        if cancelled:
            raise Refusal("Fuzz check interrupted", cancelled)
        start(build, 1024, 600, "build", ["cargo", "+" + toolchain, "fuzz", "build",
              "--target-dir", str(build / "target"), "--codegen-units", "16"], env)
        build_started = True
        wait_children()
        release = harness_directory(build, targets)
        for target in targets:
            if cancelled:
                raise Refusal("Fuzz check interrupted", cancelled)
            start(output / target, 64, 180, "run", [str(harness_path(release, target)),
                  str(output / target / "corpus"), f"-runs={args.runs}", "-reload=0", "-max_len=4096",
                  "-timeout=10", "-rss_limit_mb=2048", "-seed=1008",
                  f"-artifact_prefix={output / target / 'crashes'}/"], env)
            workers_started.append(target)
        wait_children()
        verify_progress(output, targets, args.runs, args.require_mutations)
        if args.runs != 10000:
            print(f"Short test only: {args.runs} runs per target; required smoke is 10000.")
    except Refusal as error:
        print(str(error), file=sys.stderr)
        status = error.status
    except (OSError, ValueError, subprocess.SubprocessError, StopIteration) as error:
        print(f"Fuzz check failed: {error}", file=sys.stderr)
        status = 1
    finally:
        uncertain = stop_children(children)
        filesystem_failed = False
        try:
            evidence = list(build.glob("build.*"))
        except OSError as error:
            print(f"Cannot inspect fuzz build evidence {build}: {error}", file=sys.stderr)
            filesystem_failed = True
            evidence = []
        for path in evidence:
            if not (output / path.name).exists():
                try:
                    shutil.copy2(path, output / path.name)
                except OSError as error:
                    print(f"Cannot preserve fuzz build evidence {path}: {error}", file=sys.stderr)
                    filesystem_failed = True
        build_safe = not build_started or cleanup_confirmed(build / "build.resources.json")
        if build_safe and not uncertain and not filesystem_failed:
            try:
                shutil.rmtree(build)
            except OSError as error:
                print(f"Cannot remove confirmed fuzz build {build}: {error}", file=sys.stderr)
                filesystem_failed = True
        else:
            if build_safe and not uncertain:
                print(f"Fuzz build retained because evidence preservation failed: {build}", file=sys.stderr)
            else:
                print(f"Fuzz build retained because process cleanup is unconfirmed: {build}", file=sys.stderr)
                uncertain = True
        for target in workers_started:
            if not cleanup_confirmed(output / target / "run.resources.json"):
                print(f"Fuzz target retained because process cleanup is unconfirmed: {output / target}", file=sys.stderr)
                uncertain = True
        if (uncertain or filesystem_failed) and status == 0:
            status = 125
        if status == 0 and not args.output_directory:
            try:
                shutil.rmtree(output)
            except OSError as error:
                print(f"Cannot remove confirmed fuzz output {output}: {error}", file=sys.stderr)
                print(f"Fuzz evidence retained at {output}", file=sys.stderr)
                status = 125
        else:
            print(f"Fuzz evidence retained at {output}", file=sys.stderr)
    return status


if __name__ == "__main__":
    sys.exit(main())
