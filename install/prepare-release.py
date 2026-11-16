#!/usr/bin/env python3
# SPDX-License-Identifier: MPL-2.0
"""Produce revision-stamped standalone installers and their checksum manifest."""
import hashlib
import importlib.util
from pathlib import Path
import subprocess


ARTIFACTS = ("install.sh", "install.ps1", "installers.sha256")


def prepare(root: Path, revision: str) -> None:
    source = root / "install"
    output = root / "target/installer-release"
    spec = importlib.util.spec_from_file_location("installer_staging", source / "prepare-installers.py")
    staging = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(staging)
    staging.prepare(source, output, revision)
    lines = []
    for name in ARTIFACTS[:2]:
        digest = hashlib.sha256((output / name).read_bytes()).hexdigest()
        lines.append(f"{digest}  {name}\n")
    with (output / "installers.sha256").open("w", encoding="ascii", newline="\n") as stream:
        stream.write("".join(lines))


def main():
    root = Path(__file__).resolve().parent.parent
    revision = subprocess.check_output(
        ["git", "rev-parse", "--verify", "HEAD"], cwd=root, text=True
    ).strip()
    prepare(root, revision)


if __name__ == "__main__":
    main()
