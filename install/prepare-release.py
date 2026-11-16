#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-only
"""Produce release installers, their hashes and vendored Corresponding Source."""
import hashlib
import importlib.util
from pathlib import Path
import subprocess
import sys


INSTALLERS = ("install.sh", "install.ps1")


def prepare_installers(root: Path, revision: str) -> None:
    source = root / "install"
    output = root / "target/installer-release"
    spec = importlib.util.spec_from_file_location("installer_staging", source / "prepare-installers.py")
    staging = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(staging)
    staging.prepare(source, output, revision)
    lines = []
    for name in INSTALLERS:
        digest = hashlib.sha256((output / name).read_bytes()).hexdigest()
        lines.append(f"{digest}  {name}\n")
    with (output / "installers.sha256").open("w", encoding="ascii", newline="\n") as stream:
        stream.write("".join(lines))


def prepare(root: Path, revision: str) -> None:
    prepare_installers(root, revision)
    source = root / "install"
    output = root / "target/installer-release"
    spec = importlib.util.spec_from_file_location("source_archive", source / "source-archive.py")
    source_archive = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(source_archive)
    archive = output / "SOURCE.tar.gz"
    source_archive.create_archive(root, archive, revision)
    digest = hashlib.sha256(archive.read_bytes()).hexdigest()
    with Path(str(archive) + ".sha256").open("w", encoding="ascii", newline="\n") as stream:
        stream.write(f"{digest}  SOURCE.tar.gz\n")


def main():
    # Local imports must not add bytecode files to the captured checkout.
    sys.dont_write_bytecode = True
    root = Path(__file__).resolve().parent.parent
    revision = subprocess.check_output(
        ["git", "rev-parse", "--verify", "HEAD"], cwd=root, text=True
    ).strip()
    prepare(root, revision)


if __name__ == "__main__":
    main()
