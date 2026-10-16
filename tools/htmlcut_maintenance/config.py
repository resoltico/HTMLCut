# SPDX-License-Identifier: MPL-2.0
"""Authoritative source configuration and derived release metadata."""

from pathlib import Path
import os
import re
import sys
import tomllib

ROOT = Path(__file__).resolve().parents[2]


def read(path):
    with (ROOT / path).open("rb") as source:
        return tomllib.load(source)


def toolchain():
    return read("rust-toolchain.toml")["toolchain"]["channel"]


def tools():
    return read("tools/contributor-tools.toml")


def tool_pin(name):
    pin = tools()["tools"].get(name)
    if pin is None:
        raise ValueError(f"unknown contributor tool {name}")
    return pin


def cargo_download_paths(environment=None):
    environment = os.environ if environment is None else environment
    cargo_home = Path(environment.get("CARGO_HOME") or Path.home() / ".cargo")
    return [
        str(cargo_home / "registry" / kind / "index.crates.io-*")
        for kind in ("index", "cache")
    ]


def ci_tool_paths(name, environment=None):
    """Select a fresh hosted-runner root only for controlled installation inputs."""
    tool_pin(name)
    environment = os.environ if environment is None else environment
    required = ("RUNNER_TEMP", "RUNNER_OS", "RUNNER_ARCH", "ImageOS", "ImageVersion")
    if any(not environment.get(key, "").strip() for key in required):
        raise ValueError(
            "tool caching requires hosted runner image and platform identity"
        )
    platform = "-".join(environment[key] for key in required[1:])
    if not re.fullmatch(r"[A-Za-z0-9_.-]+", platform):
        raise ValueError("hosted runner image identity contains unsupported characters")
    fixed = {
        "RUSTFLAGS",
        "CARGO_ENCODED_RUSTFLAGS",
        "RUSTC",
        "RUSTC_WRAPPER",
        "RUSTC_WORKSPACE_WRAPPER",
        "RUSTC_BOOTSTRAP",
        "CARGO_BUILD_TARGET",
        "CARGO_INCREMENTAL",
        "MACOSX_DEPLOYMENT_TARGET",
        "CC",
        "CXX",
        "AR",
        "CFLAGS",
        "CXXFLAGS",
        "LDFLAGS",
        "CFG_OMIT_COMMIT_HASH",
    }
    prefixes = (
        "CARGO_TARGET_",
        "CARGO_PROFILE_",
        "CARGO_REGISTRY_",
        "CARGO_REGISTRIES_",
        "CARGO_SOURCE_",
        "CC_",
        "CXX_",
        "AR_",
        "CFLAGS_",
        "CXXFLAGS_",
    )
    forbidden = sorted(
        key
        for key, value in environment.items()
        if (value or key == "CFG_OMIT_COMMIT_HASH")
        and (
            key in fixed
            or key.startswith(prefixes)
            or key.startswith("CARGO_BUILD_")
            and key
            not in {
                "CARGO_BUILD_TARGET_DIR",
                "CARGO_BUILD_BUILD_DIR",
            }
        )
    )
    if forbidden:
        raise ValueError(
            "tool caching refuses build overrides: " + ", ".join(forbidden)
        )
    cargo_home = Path(environment.get("CARGO_HOME") or Path.home() / ".cargo")
    if not cargo_home.is_absolute():
        raise ValueError("tool caching requires an absolute Cargo home")
    if any(
        (cargo_home / name).exists() or (cargo_home / name).is_symlink()
        for name in ("config", "config.toml")
    ):
        # Cargo install still reads this configuration from an owned build cwd.
        # Do not inspect or fingerprint a file that may contain owner secrets.
        raise ValueError("tool caching refuses Cargo-home configuration")
    parent = Path(environment["RUNNER_TEMP"]).resolve(strict=True)
    if not parent.is_dir():
        raise ValueError("RUNNER_TEMP must be an existing owned job directory")
    root = parent / "htmlcut-cargo-tools" / name
    if root.exists() or root.is_symlink():
        raise ValueError("CI tool restore root must be new")
    return {
        "root": str(root),
        "binary": str(
            root
            / "bin"
            / (name + (".exe" if environment["RUNNER_OS"] == "Windows" else ""))
        ),
        "platform": platform,
    }


def targets():
    rows = read("tools/release-targets.toml")["targets"]
    if (
        len(rows) != 4
        or len({row["triple"] for row in rows}) != len(rows)
        or len({row["id"] for row in rows}) != len(rows)
    ):
        raise ValueError(
            "release target inventory must contain four distinct native targets"
        )
    return rows


def matrix():
    return {
        "include": [
            dict(
                id=row["id"],
                runs_on=row["runner"],
                target_triple=row["triple"],
                artifact_bundle_name="standalone-" + row["id"],
                needs_musl_tools="musl" in row["triple"],
            )
            for row in targets()
        ]
    }


def prerequisites():
    minimum = (3, 13) if sys.platform == "darwin" else (3, 11)
    if sys.version_info < minimum:
        raise ValueError(
            "maintenance requires Python "
            + ".".join(map(str, minimum))
            + "+ on this platform; invoke a suitable python3"
        )
    from .process import check_process_ownership

    check_process_ownership()


def asset_names(version):
    return [
        f"htmlcut-source-{version}.zip",
        f"htmlcut-source-{version}.tar.gz",
        *[f"htmlcut-{version}-{row['triple']}.{row['archive']}" for row in targets()],
        f"htmlcut-{version}-checksums.txt",
    ]


def mutation_matrix():
    rows = [
        row
        for row in matrix()["include"]
        if row["needs_musl_tools"] or "windows" in row["id"]
    ]
    if len(rows) != 2:
        raise ValueError(
            "mutation selection must contain complementary Linux and Windows lanes"
        )
    return {"include": rows}
