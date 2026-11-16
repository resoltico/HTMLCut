#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-only
"""Add checksum-bound installers to a fresh copy of cargo-dist release assets."""
import argparse
import hashlib
import importlib.util
from pathlib import Path
import re


def prepare(directory: Path, revision: str) -> None:
    directory = directory.resolve(strict=True)
    manifest = directory / "sha256.sum"
    archives = sorted(directory.glob("htmlcut-*.tar.gz")) + sorted(directory.glob("htmlcut-*.zip"))
    if not archives:
        raise ValueError("directory contains no HTMLCut release archives")
    if manifest.is_symlink() or (manifest.exists() and not manifest.is_file()):
        raise ValueError("sha256.sum must be a regular release manifest")
    if manifest.exists():
        original = manifest.read_text(encoding="ascii")
    else:
        # cargo-dist local builds emit one original checksum beside each archive.
        sidecars = [Path(str(archive) + ".sha256") for archive in archives]
        if any(path.is_symlink() or not path.is_file() for path in sidecars):
            raise ValueError("archives need original per-archive checksums or sha256.sum")
        original = "".join(path.read_text(encoding="ascii").rstrip() + "\n" for path in sidecars)
    entries = {}
    for line in original.splitlines():
        if not line:
            continue
        match = re.fullmatch(r"([0-9a-fA-F]{64}) [ *]([^/\\]+)", line)
        if not match or match[2] in entries:
            raise ValueError("release manifest has a malformed or duplicate entry")
        entries[match[2]] = match[1].lower()
    for archive in archives:
        if archive.is_symlink() or not archive.is_file():
            raise ValueError(f"archive is not a regular file: {archive.name}")
        if hashlib.sha256(archive.read_bytes()).hexdigest() != entries.get(archive.name):
            raise ValueError(f"archive does not match the original release manifest: {archive.name}")
    for name in ("install.sh", "install.ps1"):
        if name in entries or (directory / name).exists() or (directory / name).is_symlink():
            raise ValueError(f"staging refuses an existing installer or hash: {name}")
    source = Path(__file__).resolve().parent
    spec = importlib.util.spec_from_file_location("installer_staging", source / "prepare-installers.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    module.prepare(source, directory, revision)
    additions = []
    for name in ("install.sh", "install.ps1"):
        digest = hashlib.sha256((directory / name).read_bytes()).hexdigest()
        additions.append(f"{digest}  {name}\n")
    with manifest.open("w", encoding="ascii", newline="\n") as stream:
        stream.write(original.rstrip() + "\n" + "".join(additions))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("directory", type=Path)
    parser.add_argument("--revision", default="main")
    args = parser.parse_args()
    try:
        prepare(args.directory, args.revision)
    except (OSError, ValueError) as error:
        parser.exit(1, f"Cannot prepare test release: {error}\n")


if __name__ == "__main__":
    main()
