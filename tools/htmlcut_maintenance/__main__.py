# SPDX-License-Identifier: MPL-2.0
"""HTMLCut contributor and release commands, using the invoking Python."""

import sys

if sys.version_info < ((3, 13) if sys.platform == "darwin" else (3, 11)):
    raise SystemExit(
        "maintenance requires Python 3.13+ on macOS, Python 3.11+ on Linux/Windows; invoke a suitable python3"
    )
import argparse
import importlib
import json
from . import config


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "command", choices=["check", "qa", "install", "release", "matrix", "process"]
    )
    args = parser.parse_args(sys.argv[1:2])
    rest = sys.argv[2:]
    if args.command == "matrix":
        print(json.dumps(config.matrix()))
        return 0
    if args.command == "check":
        from .commands import check

        return check()
    if args.command == "qa":
        if rest == ["--help"]:
            print("QA operations: fuzz, mutation")
            return 0
        if not rest or rest[0] not in ("fuzz", "mutation"):
            parser.error("qa requires fuzz or mutation")
        module, rest = rest[0], rest[1:]
    elif args.command == "release":
        operations = (
            "identity",
            "source",
            "package",
            "accept",
            "checksums",
            "notes",
            "admission",
            "signature",
            "notice",
            "assets",
            "publish",
            "verify",
        )
        if rest == ["--help"]:
            print("Release operations: " + ", ".join(operations))
            return 0
        if not rest or rest[0] not in operations:
            parser.error("release requires one of: " + ", ".join(operations))
        module, rest = "release." + rest[0], rest[1:]
    else:
        module = args.command
    sys.argv = [f"python3 -m tools.htmlcut_maintenance {args.command}", *rest]
    return importlib.import_module("." + module, __package__).main()


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (ValueError, OSError) as error:
        print(str(error), file=sys.stderr)
        sys.exit(2)
