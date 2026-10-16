# SPDX-License-Identifier: MPL-2.0
"""Bounded pinned installations into caller-selected fresh roots."""

import argparse
import os
from pathlib import Path
import re
import shutil
import stat
import sys
import tempfile
from . import config, process


def confirmed_exit(report):
    """Only known child exit and explicit cleanup confirmation permit deletion."""
    return (
        isinstance(report, dict)
        and report.get("cleanup_errors") == []
        and type(report.get("returncode")) is int
    )


def remove_products(build):
    for products in (build / "target", build / "build"):
        if products.exists():
            shutil.rmtree(products)


def version_matches(name, actual):
    version = config.tool_pin(name)["version"]
    actual = actual.strip()
    if name != "cargo-nextest":
        return actual == f"{name} {version}"
    lines = actual.splitlines()
    if len(lines) not in (3, 5):
        return False
    fields = {}
    for line in lines[1:]:
        key, separator, value = line.partition(": ")
        if not separator or key in fields:
            return False
        fields[key] = value
    if len(lines) == 3:
        return (
            lines[0] == f"cargo-nextest {version}"
            and set(fields) == {"release", "host"}
            and fields["release"] == version
            and re.fullmatch(r"[A-Za-z0-9_.-]+", fields["host"]) is not None
        )
    header = re.fullmatch(
        rf"cargo-nextest {re.escape(version)} \(([0-9a-f]{{7,40}}) (\d{{4}}-\d{{2}}-\d{{2}})\)",
        lines[0],
    )
    if header is None:
        return False
    return (
        set(fields) == {"release", "commit-hash", "commit-date", "host"}
        and fields["release"] == version
        and re.fullmatch(r"[0-9a-f]{40}", fields["commit-hash"]) is not None
        and fields["commit-hash"].startswith(header[1])
        and fields["commit-date"] == header[2]
        and re.fullmatch(r"[A-Za-z0-9_.-]+", fields["host"]) is not None
    )


def _probe(name, executable, evidence, env=None):
    config.tool_pin(name)
    executable = Path(executable).absolute()
    if not stat.S_ISREG(executable.lstat().st_mode) or not os.access(
        executable, os.X_OK
    ):
        raise ValueError("tool verification requires an ordinary executable")
    arguments = ["mutants", "--version"] if name == "cargo-mutants" else ["--version"]
    report = process.run(
        [str(executable), *arguments],
        evidence,
        disk_limit_mib=4,
        timeout=10,
        log_limit_mib=1,
        poll_seconds=0.05,
        label="version",
        env=env,
        cwd=evidence,
    )
    return report, (Path(evidence) / "version.log").read_text().strip()


def verify(name, executable, evidence):
    """Probe an explicit restored executable; the caller owns cache trust and inputs."""
    config.prerequisites()
    report, actual = _probe(name, executable, evidence)
    if (
        not confirmed_exit(report)
        or process.status(report)
        or not version_matches(name, actual)
    ):
        raise ValueError(
            f"pinned version refused: {actual[:512]!r}; evidence {evidence}"
        )
    return Path(executable)


def install(name, root):
    config.prerequisites()
    pin = config.tool_pin(name)
    root = Path(root).resolve()
    if root.exists():
        raise ValueError(
            "installation root must be new; existing tool roots are never restored or trusted"
        )
    root.mkdir(parents=True)
    build = Path(
        tempfile.mkdtemp(
            prefix="htmlcut-tool-build-", dir=os.environ.get("RUNNER_TEMP")
        )
    )
    reports = []
    attempted_phases = 0
    try:
        env = {
            **os.environ,
            "CARGO_TARGET_DIR": str(build / "target"),
            "CARGO_BUILD_BUILD_DIR": str(build / "build"),
        }
        command = [
            "cargo",
            "+" + config.toolchain(),
            "install",
            name,
            "--version",
            pin["version"],
            "--locked",
            "--root",
            str(root),
        ]
        if pin.get("features"):
            command += ["--features", ",".join(pin["features"])]
        attempted_phases += 1
        report = process.run(
            command,
            build,
            disk_limit_mib=4096,
            timeout=900,
            log_limit_mib=4,
            label="install",
            env=env,
            cwd=build,
        )
        reports.append(report)
        if not confirmed_exit(report) or process.status(report):
            raise ValueError(
                f"pinned installation failed; retained evidence at {build}"
            )
        executable = root / "bin" / (name + (".exe" if os.name == "nt" else ""))
        remove_products(build)
        attempted_phases += 1
        report, actual = _probe(name, executable, build, env)
        reports.append(report)
        if (
            not confirmed_exit(report)
            or process.status(report)
            or not version_matches(name, actual)
        ):
            raise ValueError(
                f"pinned version refused: {actual[:512]!r}; evidence {build}"
            )
        for path in build.glob("*.log"):
            shutil.copy2(path, root / path.name)
        for path in build.glob("*.json"):
            shutil.copy2(path, root / path.name)
    except BaseException:
        if len(reports) == attempted_phases and all(map(confirmed_exit, reports)):
            try:
                remove_products(build)
            except OSError as error:
                print(
                    f"Confirmed children stopped, but owned product cleanup failed: {error}",
                    file=sys.stderr,
                )
        else:
            print(
                f"Compiler products retained because child cleanup is unconfirmed: {build}",
                file=sys.stderr,
            )
        print(f"Installation evidence retained at {build}", file=sys.stderr)
        raise
    else:
        shutil.rmtree(build)
    return executable


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("tool")
    parser.add_argument("--root", type=Path, required=True)
    args = parser.parse_args()
    print(install(args.tool, args.root))
    return 0
