# SPDX-License-Identifier: MPL-2.0
"""Bind one annotated tag to source, manifest version and successful main CI."""

import argparse
import json
import os
from pathlib import Path
import time
from .. import config
from .source import release_tag, git
from .assets import gh_stream, MAX_METADATA
from .admission import require_success


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("tag")
    p.add_argument("--require-ci", action="store_true")
    p.add_argument("--output", type=Path)
    a = p.parse_args()
    sha, version = release_tag(config.ROOT, a.tag, os.environ.get("RELEASE_SOURCE_SHA"))
    if git(config.ROOT, "rev-parse", "HEAD") != sha:
        raise ValueError("checkout differs from resolved release source")
    facts = dict(source_sha=sha, release_tag=a.tag, version=version)
    if a.require_ci:
        _, _, raw = gh_stream(
            [
                "api",
                "--method",
                "GET",
                "repos/"
                + os.environ["GITHUB_REPOSITORY"]
                + "/actions/workflows/ci.yml/runs",
                "-f",
                "head_sha=" + sha,
                "-f",
                "branch=main",
                "-f",
                "per_page=100",
            ],
            MAX_METADATA,
            time.monotonic() + 60,
            collect=True,
        )
        facts["ci_run"] = require_success(
            json.loads(raw), sha, ".github/workflows/ci.yml", {"push"}
        )
    if a.output:
        a.output.write_text(json.dumps(facts, indent=2) + "\n")
    if os.environ.get("GITHUB_OUTPUT"):
        with open(os.environ["GITHUB_OUTPUT"], "a") as output:
            for key, value in facts.items():
                output.write(f"{key}={value}\n")
    print(json.dumps(facts))
    return 0
