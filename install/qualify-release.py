#!/usr/bin/env python3
# SPDX-License-Identifier: MPL-2.0
"""Verify and exercise the exact installer artifacts before release upload."""
import argparse
import hashlib
import importlib.util
from pathlib import Path
import shutil
import subprocess
import tempfile


def qualify(root: Path, directory: Path, revision: str) -> None:
    source = root / "install"
    native = directory / "htmlcut-x86_64-unknown-linux-musl.tar.gz"
    spec = importlib.util.spec_from_file_location("release_staging", source / "prepare-test-release.py")
    staging = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(staging)
    with tempfile.TemporaryDirectory(prefix="installer-release-check-") as tmp:
        qualified = Path(tmp)
        for path in (native, directory / "sha256.sum"):
            shutil.copyfile(path, qualified / path.name)
        # Preserve the final manifest that end-user installers consume, verify
        # its archive hash, and derive expected scripts from this source revision
        # independently of the extra-artifact build's outputs.
        staging.prepare(qualified, revision)
        lines = []
        for name in ("install.sh", "install.ps1"):
            expected = (qualified / name).read_bytes()
            if (directory / name).read_bytes() != expected:
                raise ValueError(f"Published installer differs from source revision: {name}")
            lines.append(f"{hashlib.sha256(expected).hexdigest()}  {name}\n")
        if (directory / "installers.sha256").read_bytes() != "".join(lines).encode("ascii"):
            raise ValueError("Installer checksum manifest does not match the exact published scripts")
        # The isolated copies are byte-identical to the final upload candidates.
        subprocess.run(["bash", str(source / "install_test.sh"), str(qualified)], cwd=root, check=True)
        subprocess.run(["pwsh", "-NoProfile", "-File", str(source / "install_test.ps1"), "-Dist", str(qualified)], cwd=root, check=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("directory", type=Path)
    args = parser.parse_args()
    root = Path(__file__).resolve().parent.parent
    revision = subprocess.check_output(
        ["git", "rev-parse", "--verify", "HEAD"], cwd=root, text=True
    ).strip()
    try:
        qualify(root, args.directory.resolve(strict=True), revision)
    except (OSError, ValueError, subprocess.CalledProcessError) as error:
        parser.exit(1, f"Installer release qualification failed: {error}\n")


if __name__ == "__main__":
    main()
