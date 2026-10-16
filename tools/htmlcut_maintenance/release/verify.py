# SPDX-License-Identifier: MPL-2.0
"""Verify immutable provider metadata and anonymously downloaded native bytes."""

import argparse
import hashlib
import os
from pathlib import Path
import subprocess
import tempfile
import time
import urllib.request
from .. import config, process
from . import assets, notes
from .source import release_tag
from .accept import accept


def download(url, path, maximum):
    command = [
        "curl",
        "--disable",
        "--fail",
        "--location",
        "--proto",
        "=https",
        "--proto-redir",
        "=https",
        "--max-time",
        "120",
        "--max-filesize",
        str(maximum),
        "--output",
        str(path),
        url,
    ]
    result = process.run(
        command,
        path.parent,
        disk_limit_mib=256,
        timeout=130,
        log_limit_mib=1,
        label="download-" + path.name.replace(".", "-"),
    )
    if (
        process.status(result)
        or not path.is_file()
        or not 0 < path.stat().st_size <= maximum
    ):
        raise ValueError("public download failed or exceeded its bound")


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("tag")
    p.add_argument("--target", required=True)
    p.add_argument("--output", type=Path, required=True)
    a = p.parse_args()
    sha, version = release_tag(config.ROOT, a.tag, os.environ.get("RELEASE_SOURCE_SHA"))
    target = next((row for row in config.targets() if row["triple"] == a.target), None)
    if target is None:
        raise ValueError("unsupported native target")
    with tempfile.TemporaryDirectory(prefix="htmlcut-verify-notes-") as temporary:
        path = Path(temporary) / "notes.md"
        path.write_text(notes.from_ref(config.ROOT, sha, version))
        assets.main(["--tag", a.tag, "--version", version, "--notes", str(path)])
    a.output.mkdir(parents=True, exist_ok=False)
    manifest = f"htmlcut-{version}-checksums.txt"
    package = f"htmlcut-{version}-{a.target}.{target['archive']}"
    repository = "resoltico/HTMLCut"
    base = f"https://github.com/{repository}/releases/download/{a.tag}/"
    download(base + manifest, a.output / manifest, 65536)
    expected = assets.parse_checksums((a.output / manifest).read_bytes())
    if set(expected) != set(config.asset_names(version)) - {manifest}:
        raise ValueError("public checksum inventory differs")
    download(base + package, a.output / package, assets.MAX_ASSET)
    if assets.digest(a.output / package)[1] != expected[package]:
        raise ValueError("public package checksum differs")
    with (a.output / "attestation.json").open("xb") as proof:
        subprocess.run(
            [
                "gh",
                "attestation",
                "verify",
                str(a.output / package),
                "--repo",
                repository,
                "--signer-workflow",
                repository + "/.github/workflows/release.yml",
                "--source-digest",
                sha,
                "--format",
                "json",
            ],
            stdout=proof,
            check=True,
            timeout=120,
        )
    print(accept(a.output / package, a.target, version))
    return 0
