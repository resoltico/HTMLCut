# SPDX-License-Identifier: MPL-2.0
"""Authoritative source configuration and derived release metadata."""

from pathlib import Path
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
