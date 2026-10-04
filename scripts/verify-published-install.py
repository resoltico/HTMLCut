#!/usr/bin/env python3
# SPDX-License-Identifier: MPL-2.0
"""Execute the exact native install example from the current published API baseline tag."""
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
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[1]
    baseline = tomllib.loads((root / "semver-baseline/htmlcut-core/BASELINE.toml").read_text())
    version = baseline["package_version"]
    workspace = tomllib.loads((root / "Cargo.toml").read_text())["workspace"]["package"]["version"]
    if version != workspace:
        args.output.write_text(json.dumps(dict(applicable=False,
            reason="Workspace version is not the current published baseline")) + "\n")
        return
    tag = baseline["source_git_ref"]
    if tag != "v" + version or not re.fullmatch(r"v[0-9]+\.[0-9]+\.[0-9]+", tag):
        raise ValueError("Published install provenance is not an exact stable tag")
    source = subprocess.check_output(["git", "rev-parse", tag + "^{commit}"], cwd=root, text=True).strip()
    document = subprocess.check_output(["git", "show", tag + ":docs/getting-started.md"], cwd=root, text=True)
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
