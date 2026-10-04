#!/usr/bin/env python3
# SPDX-License-Identifier: MPL-2.0
"""Assemble native attribution from the locked target graph and original notices."""
import argparse
import json
from pathlib import Path
import subprocess
import tempfile
import tomllib


def package_key(package):
    return package["id"]


def render_notice(report, project_notice, source, version, target):
    packages = {package_key(row["package"]): row["package"] for row in report["crates"]}
    covered = {package_key(row["crate"]) for license in report["licenses"] for row in license["used_by"]}
    if not packages or covered != packages.keys():
        raise ValueError("Dependency attribution omits packages from its graph")
    sections = [project_notice.rstrip(),
        f"\nNative dependency attribution: HTMLCut {version}, {target}",
        f"Project and local dependency source: https://github.com/resoltico/HTMLCut/tree/{source}",
        f"Matching source archive: https://github.com/resoltico/HTMLCut/releases/download/v{version}/htmlcut-source-{version}.tar.gz"]
    for license in report["licenses"]:
        if not license["text"].strip() or not license["used_by"]:
            raise ValueError("Dependency license text or attribution is missing")
        sections.append(f"\n{'=' * 72}\n{license['name']} ({license['id']})")
        for row in license["used_by"]:
            package = row["crate"]
            origin = (f"https://crates.io/api/v1/crates/{package['name']}/{package['version']}/download"
                      if package["source"] else f"https://github.com/resoltico/HTMLCut/tree/{source}")
            sections.append(f"{package['name']} {package['version']}\nSource: {origin}")
        sections.append(license["text"])
    for package in sorted(packages.values(), key=lambda item: item["id"]):
        directory = Path(package["manifest_path"]).parent
        for path in sorted(directory.iterdir()):
            if path.is_file() and path.name.upper().startswith(("NOTICE", "COPYRIGHT", "AUTHORS")):
                sections.append(f"\nAdditional notice: {package['name']} {package['version']} / {path.name}\n{path.read_text(encoding='utf-8')}")
    return "\n\n".join(sections).rstrip() + "\n"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--target", required=True)
    parser.add_argument("--source", required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[1]
    inventory = subprocess.check_output(["bash", "-c",
        'source "$1"; htmlcut_contributor_cargo_tool_inventory', "inventory",
        str(root / "scripts/contributor-rust-tools.sh")], text=True)
    tools = [line.split() for line in inventory.splitlines()]
    expected = next(version for name, version, binary in tools if name == "cargo-about")
    actual = subprocess.check_output(["cargo", "about", "--version"], text=True).strip()
    if actual != f"cargo-about {expected}":
        raise ValueError(f"Install pinned cargo-about {expected} before packaging")
    targets = subprocess.check_output(["bash", str(root / "scripts/release-targets.sh"), "triples"], text=True).splitlines()
    if args.target not in targets:
        raise ValueError("Unsupported native target")
    if len(args.source) != 40 or any(c not in "0123456789abcdef" for c in args.source):
        raise ValueError("Expected exact source commit")
    policy = tomllib.loads((root / "deny.toml").read_text())
    version = tomllib.loads((root / "Cargo.toml").read_text())["workspace"]["package"]["version"]
    with tempfile.TemporaryDirectory(prefix="htmlcut-license-") as directory:
        config = Path(directory) / "about.toml"
        config.write_text("accepted = " + json.dumps(policy["licenses"]["allow"]) +
                          "\ntargets = " + json.dumps([args.target]) +
                          "\nignore-dev-dependencies = true\n[private]\nignore = false\n")
        result = subprocess.check_output(["cargo", "about", "generate", "--locked", "--fail",
            "--threshold", str(policy["licenses"]["confidence-threshold"]), "--manifest-path",
            str(root / "crates/htmlcut-cli/Cargo.toml"), "--config", str(config), "--format", "json"], cwd=root)
    notice = render_notice(json.loads(result), (root / "NOTICE").read_text(), args.source, version, args.target)
    args.output.write_text(notice, encoding="utf-8")


if __name__ == "__main__":
    main()
