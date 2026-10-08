#!/usr/bin/env python3
# SPDX-License-Identifier: MPL-2.0
"""Require successful main CI verification for the tagged source."""
import argparse
import json
from pathlib import Path
import re


def require_success(payload, source, path, events):
    eligible = [run for run in payload["workflow_runs"] if run["head_sha"] == source
        and run["head_branch"] == "main" and run["event"] in events
        and run["path"].split("@", 1)[0] == path]
    if not eligible:
        raise ValueError(f"No same-source main verification: {path}")
    latest = max(eligible, key=lambda run: (run["run_number"], run["run_attempt"]))
    if latest["status"] != "completed" or latest["conclusion"] != "success":
        raise ValueError(f"Latest same-source verification did not succeed: {path}")
    return latest["id"]


def read(path):
    with Path(path).open("rb") as stream:
        raw = stream.read(1024 * 1024 + 1)
    if len(raw) > 1024 * 1024:
        raise ValueError("Verification response exceeds its byte bound")
    return json.loads(raw)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source-sha", required=True)
    parser.add_argument("--ci", required=True)
    args = parser.parse_args()
    if not re.fullmatch(r"[0-9a-f]{40}", args.source_sha):
        parser.error("Expected exact source commit")
    ci = require_success(read(args.ci), args.source_sha, ".github/workflows/ci.yml", {"push"})
    print(f"Tagged source verified by main CI run {ci}.")

if __name__ == "__main__":
    main()
