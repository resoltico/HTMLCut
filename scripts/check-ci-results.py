#!/usr/bin/env python3
# SPDX-License-Identifier: MPL-2.0
"""Reject missing, failed, cancelled or skipped mandatory jobs."""
import json, os, sys
REQUIRED = ("release-target-matrix", "linux-maintainer", "focused-smoke", "release-target-smoke", "semver")
def failures(jobs):
    return [name for name in REQUIRED if jobs.get(name, {}).get("result") != "success"]
if __name__ == "__main__":
    errors = failures(json.loads(os.environ["JOB_RESULTS"]))
    if errors:
        print("Mandatory CI jobs did not succeed: " + ", ".join(errors), file=sys.stderr)
        sys.exit(1)
    print("Every mandatory CI job succeeded.")
