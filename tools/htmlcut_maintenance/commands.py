# SPDX-License-Identifier: MPL-2.0
"""Standard checks and production package dependency boundaries."""

import json
import os
import shutil
import subprocess
import sys
import unittest
from . import config


def dependency_boundary(metadata):
    packages = {package["name"]: package for package in metadata["packages"]}
    for name in ("htmlcut-core", "htmlcut-runtime", "htmlcut-cli"):
        if name not in packages:
            raise ValueError(f"missing product package {name}")
    production = lambda name: {
        d["name"] for d in packages[name]["dependencies"] if d.get("kind") != "dev"
    }
    if production("htmlcut-cli") != {"clap", "htmlcut-runtime"}:
        raise ValueError(
            "CLI production dependencies must be exactly clap and htmlcut-runtime"
        )
    if "htmlcut-core" not in production("htmlcut-runtime"):
        raise ValueError("runtime must use the engine")
    if production("htmlcut-core") & {
        "htmlcut-runtime",
        "htmlcut-cli",
        "clap",
    } or production("htmlcut-runtime") & {"htmlcut-cli", "clap"}:
        raise ValueError("product dependency direction is reversed")


def check():
    config.prerequisites()
    for program in ("cargo", "rustc"):
        if not shutil.which(program):
            raise ValueError(
                f"{program} is required; install the compiler declared in rust-toolchain.toml"
            )
    if not shutil.which("cargo-deny"):
        raise ValueError(
            "cargo-deny is required; run python3 -m tools.htmlcut_maintenance install cargo-deny --root PATH"
        )
    metadata = subprocess.run(
        ["cargo", "metadata", "--locked", "--no-deps", "--format-version", "1"],
        cwd=config.ROOT,
        check=True,
        capture_output=True,
    )
    metadata = json.loads(metadata.stdout)
    dependency_boundary(metadata)
    suite = unittest.defaultTestLoader.discover(
        str(config.ROOT / "tools/htmlcut_maintenance/tests"),
        top_level_dir=str(config.ROOT),
    )
    if suite.countTestCases() == 0:
        raise ValueError("maintenance test discovery is empty")
    commands = [
        ["cargo", "fmt", "--all", "--", "--check"],
        [
            "cargo",
            "clippy",
            "--locked",
            "--workspace",
            "--all-targets",
            "--all-features",
            "--",
            "-D",
            "warnings",
        ],
        ["cargo", "test", "--locked", "--workspace", "--all-features"],
        ["cargo", "doc", "--locked", "--workspace", "--all-features", "--no-deps"],
        [
            sys.executable,
            "-m",
            "unittest",
            "discover",
            "-s",
            "tools/htmlcut_maintenance/tests",
            "-t",
            ".",
            "-v",
        ],
        ["cargo", "deny", "--locked", "check"],
    ]
    for command in commands:
        env = dict(os.environ)
        if command[1] == "doc":
            env["RUSTDOCFLAGS"] = "-Dwarnings"
        if command[0] == sys.executable:
            target = os.environ.get("CARGO_BUILD_TARGET")
            binary = __import__("pathlib").Path(metadata["target_directory"])
            if target:
                binary = binary / target
            env["HTMLCUT_TEST_BINARY"] = str(
                binary / "debug" / ("htmlcut.exe" if os.name == "nt" else "htmlcut")
            )
        subprocess.run(command, cwd=config.ROOT, env=env, check=True)
    return 0
