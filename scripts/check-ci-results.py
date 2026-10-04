#!/usr/bin/env python3
# SPDX-License-Identifier: MPL-2.0
"""Fail closed on missing, cancelled, failed or unexpectedly skipped mandatory CI jobs."""
import json
import os
import sys

REQUIRED = ("release-target-matrix", "devcontainer-changes", "linux-maintainer", "cross-platform-rust-gate", "release-target-smoke")

def failures(jobs):
    errors = [name for name in REQUIRED if jobs.get(name, {}).get("result") != "success"]
    environment = jobs.get("contributor-devcontainer", {})
    changed = jobs.get("devcontainer-changes", {}).get("outputs", {}).get("changed")
    allowed = ("success", "skipped") if changed == "false" else ("success",)
    if environment.get("result") not in allowed:
        errors.append("contributor-devcontainer")
    if changed not in ("true", "false"):
        errors.append("devcontainer-change-output")
    return errors

if __name__ == "__main__":
    try:
        errors = failures(json.loads(os.environ["JOB_RESULTS"]))
    except (KeyError, ValueError, TypeError):
        errors = ["invalid job results"]
    if errors:
        print("Mandatory CI jobs did not succeed: " + ", ".join(errors), file=sys.stderr)
        sys.exit(1)
    print("Every mandatory CI job succeeded; optional skips match the explicit policy.")
