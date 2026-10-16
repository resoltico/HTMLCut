# SPDX-License-Identifier: MPL-2.0
"""Immutable source resolution and Git-owned source archives."""

import argparse
from pathlib import Path
import re
import subprocess
import tomllib
from .. import config

STABLE = re.compile(r"(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)")


def git(root, *args):
    return (
        subprocess.check_output(["git", "-C", str(root), *args], timeout=30)
        .decode()
        .strip()
    )


def resolve(root, ref):
    sha = git(root, "rev-parse", "--verify", ref + "^{commit}")
    if not re.fullmatch("[0-9a-f]{40}", sha):
        raise ValueError("source must resolve to an exact commit")
    version = tomllib.loads(git(root, "show", sha + ":Cargo.toml"))["workspace"][
        "package"
    ]["version"]
    if not STABLE.fullmatch(version):
        raise ValueError("source must declare a canonical stable workspace version")
    return sha, version


def release_tag(root, tag, expected_sha=None, main="origin/main"):
    if not tag.startswith("v") or not STABLE.fullmatch(tag[1:]):
        raise ValueError("release tag must be canonical stable vMAJOR.MINOR.PATCH")
    if git(root, "cat-file", "-t", "refs/tags/" + tag) != "tag":
        raise ValueError("release tag must be annotated")
    sha, version = resolve(root, "refs/tags/" + tag)
    if tag != "v" + version:
        raise ValueError("release tag differs from tagged workspace version")
    if expected_sha is not None and sha != expected_sha:
        raise ValueError("release tag differs from its resolved source commit")
    subprocess.run(
        ["git", "-C", str(root), "merge-base", "--is-ancestor", sha, main],
        check=True,
        timeout=30,
    )
    return sha, version


def archives(root, ref, output):
    root = Path(root)
    output = Path(output)
    sha, version = resolve(root, ref)
    output.mkdir(parents=True, exist_ok=True)
    basename = "htmlcut-source-" + version
    for form in ("zip", "tar.gz"):
        path = output / (basename + "." + form)
        if path.exists():
            raise ValueError(f"source archive already exists: {path}")
        subprocess.run(
            [
                "git",
                "-C",
                str(root),
                "archive",
                "--format=" + form,
                "--prefix=" + basename + "/",
                "-o",
                str(path.resolve()),
                sha,
            ],
            check=True,
            timeout=60,
        )
    (output / (basename + ".source-commit")).write_text(sha + "\n", encoding="utf-8")
    return sha, version


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("ref")
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--root", type=Path, default=config.ROOT)
    args = parser.parse_args()
    print(archives(args.root, args.ref, args.output))
    return 0
