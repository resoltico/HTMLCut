# SPDX-License-Identifier: MPL-2.0
"""Immutable GitHub release convergence. Requires separate publication authorization."""

import argparse
import json
import os
from pathlib import Path
import subprocess
import tempfile
import time
from .. import config
from . import assets, notes
from .source import release_tag


def provider(tag):
    _, _, raw = assets.gh_stream(
        [
            "release",
            "view",
            tag,
            "--json",
            "tagName,name,isDraft,isPrerelease,body,assets",
        ],
        assets.MAX_METADATA,
        time.monotonic() + 30,
        collect=True,
    )
    return json.loads(raw)


def publish(tag, dist, root=config.ROOT):
    sha, version = release_tag(root, tag, os.environ.get("RELEASE_SOURCE_SHA"))
    if not os.environ.get("GH_TOKEN"):
        raise ValueError("GH_TOKEN is required for publication")
    text = notes.from_ref(root, sha, version)
    with tempfile.TemporaryDirectory(prefix="htmlcut-release-notes-") as temporary:
        path = Path(temporary) / "notes.md"
        path.write_text(text)
        try:
            release = provider(tag)
        except ValueError:
            subprocess.run(
                [
                    "gh",
                    "release",
                    "create",
                    tag,
                    "--title",
                    tag,
                    "--notes-file",
                    str(path),
                    "--draft",
                    "--verify-tag",
                ],
                cwd=root,
                check=True,
                timeout=60,
            )
            release = provider(tag)
        common = [
            "--tag",
            tag,
            "--version",
            version,
            "--dist",
            str(dist),
            "--notes",
            str(path),
        ]
        assets.main([*common, "--allow-missing"])
        # Matching published retries perform no edit or upload.
        if not release["isDraft"]:
            print(
                "Existing published release has matching immutable bytes; no edit performed."
            )
            return
        existing = {entry["name"] for entry in release["assets"]}
        for name in config.asset_names(version):
            if name not in existing:
                # No --clobber. Races must converge on exactly matching bytes.
                result = subprocess.run(
                    ["gh", "release", "upload", tag, str(Path(dist) / name)],
                    cwd=root,
                    timeout=120,
                )
                if result.returncode and name not in {
                    row["name"] for row in provider(tag)["assets"]
                }:
                    raise ValueError("release upload failed")
        assets.main(common)
        subprocess.run(
            [
                "gh",
                "release",
                "edit",
                tag,
                "--title",
                tag,
                "--draft=false",
                "--prerelease=false",
                "--latest",
                "--verify-tag",
            ],
            cwd=root,
            check=True,
            timeout=60,
        )


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("tag")
    p.add_argument("--dist", type=Path, required=True)
    a = p.parse_args()
    publish(a.tag, a.dist)
    return 0
