#!/usr/bin/env python3
# SPDX-License-Identifier: MPL-2.0
"""Execute the exact native install example from an immutable published source commit."""
import argparse
import hashlib
import json
from pathlib import Path
import platform
import re
import subprocess
import tempfile
import tomllib


def install_block(document, windows):
    language = "powershell" if windows else "sh"
    match = re.search(r"```" + language + r"\n(.*?)\n```", document, re.S)
    if match is None:
        raise ValueError("Published native install example is missing")
    return match[1] + "\n"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source-sha", required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[1]
    source = args.source_sha
    if not re.fullmatch(r"[0-9a-f]{40}", source):
        parser.error("Expected immutable published source commit")
    manifest = subprocess.check_output(["git", "show", source + ":Cargo.toml"], cwd=root, text=True)
    version = tomllib.loads(manifest)["workspace"]["package"]["version"]
    tag = "v" + version
    tagged_source = subprocess.check_output(["git", "rev-parse", tag + "^{commit}"], cwd=root, text=True).strip()
    if tagged_source != source:
        raise ValueError("Published version tag differs from immutable source")
    document = subprocess.check_output(["git", "show", source + ":docs/getting-started.md"], cwd=root, text=True)
    windows = platform.system() == "Windows"
    code = install_block(document, windows)
    if f'"{version}"' not in code and f"VERSION={version}\n" not in code:
        raise ValueError("Published install example version differs")
    with tempfile.TemporaryDirectory(prefix="htmlcut-published-install-") as temporary:
        directory = Path(temporary)
        script = directory / ("install.ps1" if windows else "install.sh")
        script.write_text(code, encoding="utf-8")
        command = (["powershell.exe", "-NoProfile", "-ExecutionPolicy", "Bypass", "-File", str(script)]
                   if windows else ["bash", str(script)])
        result = subprocess.run(command, cwd=directory, capture_output=True, text=True, timeout=180)
        if result.returncode != 0 or f"htmlcut {version}" not in result.stdout:
            raise ValueError("Published install example failed:\n" + result.stdout + result.stderr)
        binaries = list(directory.glob("htmlcut-*/htmlcut.exe" if windows else "htmlcut-*/htmlcut"))
        if len(binaries) != 1:
            raise ValueError("Published install example did not produce exactly one executable")
        digest = hashlib.sha256(binaries[0].read_bytes()).hexdigest()
    args.output.write_text(json.dumps(dict(applicable=True, passed=True, published_tag=tag,
        source_commit=source, runner_system=platform.system(), runner_machine=platform.machine(),
        document_sha256=hashlib.sha256(document.encode()).hexdigest(),
        script_sha256=hashlib.sha256(code.encode()).hexdigest(), binary_sha256=digest,
        exact_published_example_executed=True), indent=2) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
