#!/usr/bin/env python3
# SPDX-License-Identifier: MPL-2.0
"""Resolve Cargo's declared build directory for the cache action's absolute glob."""
from pathlib import Path
import tomllib


def build_directory(root):
    config = tomllib.loads((root / ".cargo/config.toml").read_text())
    return (root / config["build"]["build-dir"]).resolve()


if __name__ == "__main__":
    print("build=" + str(build_directory(Path(__file__).resolve().parents[1])))
