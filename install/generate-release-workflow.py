#!/usr/bin/env python3
# SPDX-License-Identifier: MPL-2.0
"""Generate cargo-dist's workflow with the required pre-upload qualification."""
import argparse
from pathlib import Path
import os
import re
import shutil
import subprocess
import tempfile


QUALIFICATION = '''      - name: Qualify the exact installer release artifacts
        shell: bash
        run: python3 install/qualify-release.py target/distrib
'''


def overlay(text: str) -> str:
    start = text.find("  build-global-artifacts:\n")
    if start < 0 or text.count("  build-global-artifacts:\n") != 1:
        raise ValueError("Expected one cargo-dist global artifact job")
    end = text.find("\n  # Determines if we should publish/announce", start)
    if end < 0:
        raise ValueError("Cannot locate the end of cargo-dist's global artifact job")
    job = text[start:end]
    marker = '      - name: "Upload artifacts"\n'
    if job.count(marker) != 1 or QUALIFICATION in text:
        raise ValueError("Expected one unmodified global artifact upload step")
    job = job.replace(marker, QUALIFICATION + marker, 1)
    return text[:start] + job + text[end:]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    root = Path(__file__).resolve().parent.parent
    versions = re.findall(r'^cargo-dist-version = "([^"\n]+)"$', (root / "dist-workspace.toml").read_text(), re.MULTILINE)
    if len(versions) != 1:
        parser.exit(1, "Expected one configured cargo-dist-version in dist-workspace.toml\n")
    configured = versions[0]
    actual = subprocess.check_output(["dist", "--version"], cwd=root, text=True).strip()
    if actual != "cargo-dist " + configured:
        parser.exit(1, f"Use configured cargo-dist {configured}; found {actual}\n")
    workflow = root / ".github/workflows/release.yml"
    if args.check:
        original = workflow.read_bytes()
        # Generate from the intended working source in an owned disposable copy;
        # the file being checked is never overwritten, even temporarily.
        listing = subprocess.check_output(
            ["git", "ls-files", "-z", "--cached", "--others", "--exclude-standard"], cwd=root
        )
        with tempfile.TemporaryDirectory(prefix="htmlcut-workflow-source-") as tmp:
            isolated = Path(tmp)
            for raw in sorted(set(listing.split(b"\0")) - {b""}):
                name = os.fsdecode(raw)
                if name.split("/", 1)[0] in {".git", "target"}:
                    continue
                source = root / name
                if not source.is_file():
                    continue
                destination = isolated / name
                destination.parent.mkdir(parents=True, exist_ok=True)
                shutil.copy2(source, destination)
            environment = dict(os.environ, CARGO_TARGET_DIR=str(isolated / "target"))
            subprocess.run(["dist", "generate"], cwd=isolated, env=environment, check=True)
            expected = overlay((isolated / ".github/workflows/release.yml").read_text()).encode("utf-8")
        if original != expected:
            parser.exit(1, "Release workflow is stale; run python3 install/generate-release-workflow.py\n")
    else:
        subprocess.run(["dist", "generate"], cwd=root, check=True)
        workflow.write_bytes(overlay(workflow.read_text()).encode("utf-8"))


if __name__ == "__main__":
    main()
