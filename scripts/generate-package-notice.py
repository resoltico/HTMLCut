#!/usr/bin/env python3
# SPDX-License-Identifier: MPL-2.0
"""Assemble native attribution from the locked target graph and original notices."""
import argparse
import hashlib
from html.parser import HTMLParser
import io
import json
import os
import re
from pathlib import Path
import subprocess
import tempfile
import tarfile
import tomllib
from urllib.request import Request, urlopen


class CopyrightText(HTMLParser):
    def __init__(self):
        super().__init__(convert_charrefs=True)
        self.parts = []

    def handle_data(self, text):
        self.parts.append(text)


def runtime_notice(root, target):
    compiler = subprocess.check_output(["rustc", "-Vv"], text=True)
    metadata = dict(line.split(": ", 1) for line in compiler.splitlines()[1:] if ": " in line)
    expected = tomllib.loads((root / "rust-toolchain.toml").read_text())["toolchain"]["channel"]
    if metadata["release"] != expected or not re.fullmatch("[0-9a-f]{40}", metadata["commit-hash"]):
        raise ValueError("Native runtime notice requires the pinned release compiler")
    source = metadata["commit-hash"]
    sysroot = Path(subprocess.check_output(["rustc", "--print", "sysroot"], text=True).strip())
    copyright_bytes = (sysroot / "share/doc/rust/COPYRIGHT-library.html").read_bytes()
    copyright = copyright_bytes.decode("utf-8")
    if "Copyright notices for The Rust Standard Library" not in copyright or "Permission is hereby granted" not in copyright:
        raise ValueError("Pinned compiler standard-library attribution is missing")
    parser = CopyrightText()
    parser.feed(copyright)
    sections = [f"Rust {expected} standard-library attribution\nSource: https://github.com/rust-lang/rust/tree/{source}\n"
                f"Compiler copyright notice SHA-256: {hashlib.sha256(copyright_bytes).hexdigest()}",
                "".join(parser.parts)]
    llvm_url = f"https://api.github.com/repos/rust-lang/rust/contents/src/llvm-project?ref={source}"
    headers = {"Authorization": "Bearer " + token} if (token := os.environ.get("GH_TOKEN")) else {}
    with urlopen(Request(llvm_url, headers=headers), timeout=60) as response:
        llvm = json.load(response)
    if llvm.get("submodule_git_url") != "https://github.com/rust-lang/llvm-project.git" or not re.fullmatch("[0-9a-f]{40}", llvm["sha"]):
        raise ValueError("Pinned Rust LLVM source differs")
    for component in ["compiler-rt", *(["libunwind"] if "musl" in target else [])]:
        url = f"https://raw.githubusercontent.com/rust-lang/llvm-project/{llvm['sha']}/{component}/LICENSE.TXT"
        with urlopen(url, timeout=60) as response:
            license = response.read().decode("utf-8")
        if not license.strip():
            raise ValueError("LLVM runtime permission text is missing")
        sections.append(f"LLVM {component} runtime attribution\nSource: {url}\n{license}")
    if "musl" in target:
        script_url = f"https://raw.githubusercontent.com/rust-lang/rust/{source}/src/ci/docker/scripts/musl.sh"
        with urlopen(script_url, timeout=60) as response:
            script = response.read().decode("utf-8")
        version = re.search(r"^MUSL=musl-([0-9]+\.[0-9]+\.[0-9]+)$", script, re.MULTILINE)
        if version is None:
            raise ValueError("Pinned Rust musl source version is missing")
        url = f"https://www.musl-libc.org/releases/musl-{version[1]}.tar.gz"
        with urlopen(url, timeout=60) as response:
            archive = response.read()
        with tarfile.open(fileobj=io.BytesIO(archive)) as archive:
            license = archive.extractfile(f"musl-{version[1]}/COPYRIGHT").read().decode("utf-8")
        if "Permission is hereby granted" not in license:
            raise ValueError("musl permission text is missing")
        sections.append(f"musl {version[1]} runtime attribution\nSource: {url}\nRust build and applied patches: {script_url}\n{license}")
    return "\n\n".join(sections).rstrip() + "\n"


def bash_program():
    for key in ("ProgramW6432", "ProgramFiles", "ProgramFiles(x86)"):
        if root := os.environ.get(key):
            candidate = Path(root) / "Git/bin/bash.exe"
            if candidate.is_file():
                return str(candidate)
    return "bash"


def shell_inventory(root):
    # Python on Windows starts native programs directly. Prefer Git Bash over the WSL launcher;
    # forward-slash drive paths are understood by Git Bash, including paths containing spaces.
    return subprocess.check_output([bash_program(), "-c",
        'cd "$1"; source ./scripts/contributor-rust-tools.sh; htmlcut_contributor_cargo_tool_inventory',
        "inventory", root.as_posix()], text=True)


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
    inventory = shell_inventory(root)
    tools = [line.split() for line in inventory.splitlines()]
    expected = next(version for name, version, binary in tools if name == "cargo-about")
    actual = subprocess.check_output(["cargo", "about", "--version"], text=True).strip()
    if actual != f"cargo-about {expected}":
        raise ValueError(f"Install pinned cargo-about {expected} before packaging")
    targets = subprocess.check_output([bash_program(), (root / "scripts/release-targets.sh").as_posix(), "triples"], text=True).splitlines()
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
    notice += "\n" + runtime_notice(root, args.target)
    args.output.write_text(notice, encoding="utf-8")


if __name__ == "__main__":
    main()
