# SPDX-License-Identifier: MPL-2.0
"""Pin/feature protocol and fresh-root ownership; real compiler runs are separate evidence."""

import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
from tools.htmlcut_maintenance import config, install


class InstallTests(unittest.TestCase):
    def test_locked_pins_features_and_owned_builds_ignore_inherited_owner_paths(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            owner = root / "owner"
            owner.mkdir()
            (owner / "keep").write_text("owner")
            calls = []

            def run(command, build, **options):
                build = Path(build)
                calls.append((command, build, options))
                label = options["label"]
                if label == "install":
                    self.assertIn("--locked", command)
                    self.assertEqual(command[1], "+" + config.toolchain())
                    self.assertEqual(
                        command[command.index("--version") + 1],
                        config.tools()["tools"]["cargo-about"]["version"],
                    )
                    self.assertEqual(command[command.index("--features") + 1], "cli")
                    self.assertNotEqual(options["env"]["CARGO_TARGET_DIR"], str(owner))
                    Path(options["env"]["CARGO_TARGET_DIR"]).mkdir()
                    Path(options["env"]["CARGO_BUILD_BUILD_DIR"]).mkdir()
                    (build / "install.log").write_text("build")
                    destination = root / "tool/bin"
                    destination.mkdir(parents=True)
                    (
                        destination
                        / ("cargo-about.exe" if os.name == "nt" else "cargo-about")
                    ).touch()
                else:
                    self.assertFalse((build / "target").exists())
                    self.assertFalse((build / "build").exists())
                    self.assertEqual(options["disk_limit_mib"], 4)
                    self.assertEqual(options["timeout"], 10)
                    (build / "version.log").write_text(
                        "cargo-about "
                        + config.tools()["tools"]["cargo-about"]["version"]
                        + "\n"
                    )
                (build / (label + ".resources.json")).write_text("{}")
                return dict(refusal=None, returncode=0, cleanup_errors=[])

            with (
                patch.object(install.process, "run", side_effect=run),
                patch.dict(
                    os.environ,
                    {
                        "CARGO_TARGET_DIR": str(owner),
                        "CARGO_BUILD_BUILD_DIR": str(owner),
                    },
                ),
            ):
                executable = install.install("cargo-about", root / "tool")
            self.assertTrue(executable.is_file())
            self.assertEqual((owner / "keep").read_text(), "owner")
            self.assertEqual(
                [call[2]["label"] for call in calls], ["install", "version"]
            )
            self.assertFalse(calls[0][1].exists())
            self.assertTrue((root / "tool/install.resources.json").exists())

    def test_existing_roots_and_unknown_tools_refuse_without_starting_processes(self):
        with (
            tempfile.TemporaryDirectory() as temporary,
            patch.object(install.process, "run") as launched,
        ):
            with self.assertRaises(ValueError):
                install.install("cargo-deny", Path(temporary))
            with self.assertRaises(ValueError):
                install.install("unknown", Path(temporary) / "new")
            launched.assert_not_called()

    def test_failed_or_wrong_version_never_becomes_success(self):
        for phase in ["install", "version"]:
            with self.subTest(phase=phase), tempfile.TemporaryDirectory() as temporary:
                builds = []

                def run(command, build, **options):
                    build = Path(build)
                    builds.append(build)
                    label = options["label"]
                    (build / (label + ".log")).write_text(
                        "wrong version" if label == "version" else "failed build"
                    )
                    return dict(
                        refusal="wall-time-exceeded" if label == phase else None,
                        returncode=125 if label == phase else 0,
                        cleanup_errors=["uncertain"] if label == phase else [],
                    )

                with patch.object(install.process, "run", side_effect=run):
                    with self.assertRaises(ValueError):
                        install.install("cargo-deny", Path(temporary) / "tool")
                self.assertTrue(builds[0].exists())
                # The test owns these fake products and no subprocess was launched.
                __import__("shutil").rmtree(builds[0])
