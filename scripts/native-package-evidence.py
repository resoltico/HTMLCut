#!/usr/bin/env python3
"""Record bounded native package verification evidence without publishing a release."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import subprocess


def command(*args):
    return subprocess.run(args, check=True, capture_output=True, text=True, timeout=10).stdout.strip()


def package_digest(path):
    size = path.stat().st_size
    if size == 0 or size > 128 * 1024 * 1024:
        raise ValueError("Native package is empty or exceeds the artifact budget")
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(64 * 1024), b""):
            digest.update(chunk)
    return size, digest.hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--package", type=Path, required=True)
    parser.add_argument("--target", required=True)
    parser.add_argument("--version", required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--smoke-log", type=Path, required=True)
    args = parser.parse_args()
    if command("git", "status", "--porcelain"):
        raise ValueError("Native source evidence requires a clean checkout")
    with args.smoke_log.open("wb") as log:
        subprocess.run(["bash", "./scripts/smoke-release-artifact.sh", args.target],
                       check=True, stdout=log, stderr=subprocess.STDOUT, timeout=120)
    if args.smoke_log.stat().st_size > 2 * 1024 * 1024:
        raise ValueError("Native smoke log exceeds its evidence budget")
    size, digest = package_digest(args.package)
    evidence = {
        "schema": "htmlcut.native-package-evidence", "version": 1,
        "source_commit": command("git", "rev-parse", "HEAD"),
        "source_tree": command("git", "rev-parse", "HEAD^{tree}"),
        "product_version": args.version, "target": args.target,
        "runner_system": platform.system(), "runner_machine": platform.machine(),
        "rustc": command("rustc", "-Vv"),
        "package": args.package.name, "package_bytes": size, "package_sha256": digest,
        "smoke_command": ["./scripts/smoke-release-artifact.sh", args.target],
        "smoke_exit_status": 0,
        "smoke_log_sha256": hashlib.sha256(args.smoke_log.read_bytes()).hexdigest(),
        "github_run_id": os.environ.get("GITHUB_RUN_ID"),
        "github_run_attempt": os.environ.get("GITHUB_RUN_ATTEMPT"),
    }
    args.output.write_text(json.dumps(evidence, indent=2) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
