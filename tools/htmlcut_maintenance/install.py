# SPDX-License-Identifier: MPL-2.0
"""Bounded pinned installations into caller-selected fresh roots."""

import argparse
import os
from pathlib import Path
import shutil
import tempfile
from . import config, process


def install(name, root):
    config.prerequisites()
    pin = config.tools()["tools"].get(name)
    if pin is None:
        raise ValueError(f"unknown contributor tool {name}")
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
        report = process.run(
            command,
            build,
            disk_limit_mib=4096,
            timeout=900,
            log_limit_mib=4,
            label="install",
            env=env,
        )
        reports.append(report)
        if process.status(report):
            raise ValueError(
                f"pinned installation failed; retained evidence at {build}"
            )
        executable = root / "bin" / (name + (".exe" if os.name == "nt" else ""))
        for products in (build / "target", build / "build"):
            if products.exists():
                shutil.rmtree(products)
        report = process.run(
            [str(executable), "--version"],
            build,
            disk_limit_mib=4,
            timeout=10,
            log_limit_mib=1,
            label="version",
            env=env,
        )
        reports.append(report)
        actual = (build / "version.log").read_text().strip()
        if process.status(report) or actual != f"{name} {pin['version']}":
            raise ValueError(f"pinned version refused: {actual!r}; evidence {build}")
        for path in build.glob("*.log"):
            shutil.copy2(path, root / path.name)
        for path in build.glob("*.json"):
            shutil.copy2(path, root / path.name)
    except BaseException:
        print(
            f"Installation evidence retained at {build}", file=__import__("sys").stderr
        )
        raise
    else:
        if all(
            report["cleanup_errors"] == [] and type(report["returncode"]) is int
            for report in reports
        ):
            shutil.rmtree(build)
    return executable


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("tool")
    parser.add_argument("--root", type=Path, required=True)
    args = parser.parse_args()
    print(install(args.tool, args.root))
    return 0
