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


class MutantsProtocolTests(unittest.TestCase):
    def test_mutants_exact_protocol_and_rejected_versions(self):
        for version_text, status in (
            ("cargo-mutants 27.1.0", 0),
            ("cargo-mutants 0.0.0", 0),
            ("version command failed", 2),
        ):
            with (
                self.subTest(version=version_text, status=status),
                tempfile.TemporaryDirectory() as temporary,
            ):
                root = Path(temporary)
                seen = []
                builds = []

                def run(command, build, **options):
                    build = Path(build)
                    builds.append(build)
                    seen.append((command, options))
                    label = options["label"]
                    (build / (label + ".log")).write_text(
                        "installed" if label == "install" else version_text + "\n"
                    )
                    (build / (label + ".resources.json")).write_text("{}")
                    if label == "version":
                        self.assertEqual(command[1:], ["mutants", "--version"])
                        self.assertEqual(options["timeout"], 10)
                        self.assertEqual(options["log_limit_mib"], 1)
                        self.assertEqual(options["disk_limit_mib"], 4)
                    return dict(
                        returncode=status if label == "version" else 0,
                        refusal=None,
                        cleanup_errors=[],
                    )

                with patch.object(install.process, "run", side_effect=run):
                    if status == 0 and version_text == "cargo-mutants 27.1.0":
                        install.install("cargo-mutants", root / "tool")
                    else:
                        with self.assertRaisesRegex(
                            ValueError, "pinned version refused"
                        ):
                            install.install("cargo-mutants", root / "tool")
                self.assertEqual(
                    [options["label"] for _, options in seen], ["install", "version"]
                )
                if builds[0].exists():
                    __import__("shutil").rmtree(builds[0])

    def test_installed_pinned_mutants_binary_uses_its_real_version_protocol(self):
        import shutil

        actual = shutil.which("cargo-mutants")
        if actual is None:
            self.skipTest("optional cargo-mutants binary is not installed")
        with tempfile.TemporaryDirectory(
            prefix="htmlcut-mutants-protocol-"
        ) as temporary:
            root = Path(temporary)
            real_run = install.process.run
            observed = []

            def run(command, build, **options):
                label = options["label"]
                build = Path(build)
                if label == "install":
                    destination = root / "tool root/bin"
                    destination.mkdir(parents=True)
                    shutil.copy2(
                        actual,
                        destination
                        / ("cargo-mutants.exe" if os.name == "nt" else "cargo-mutants"),
                    )
                    (build / "install.log").write_text(
                        "Installation phase substituted with an existing binary; version phase is real.\n"
                    )
                    (build / "install.resources.json").write_text("{}")
                    return dict(returncode=0, refusal=None, cleanup_errors=[])
                observed.append(command)
                return real_run(command, build, **options)

            with patch.object(install.process, "run", side_effect=run):
                executable = install.install("cargo-mutants", root / "tool root")
            self.assertEqual(observed, [[str(executable), "mutants", "--version"]])
            self.assertEqual(
                (root / "tool root/version.log").read_text().strip(),
                "cargo-mutants " + config.tools()["tools"]["cargo-mutants"]["version"],
            )


class FailureProductTests(unittest.TestCase):
    def test_only_confirmed_failed_phases_remove_owned_compiler_products(self):
        for phase in ("install", "version"):
            for confirmed in (True, False):
                with (
                    self.subTest(phase=phase, confirmed=confirmed),
                    tempfile.TemporaryDirectory() as temporary,
                ):
                    root = Path(temporary)
                    owner = root / "owner cache"
                    owner.mkdir()
                    (owner / "marker").write_text("keep")
                    builds = []

                    def run(command, build, **options):
                        build = Path(build)
                        builds.append(build)
                        label = options["label"]
                        for name in ("target", "build"):
                            (build / name).mkdir(exist_ok=True)
                            (build / name / "product").write_text("owned")
                        (build / (label + ".log")).write_text(
                            "cargo-mutants 0.0.0\n"
                            if label == "version"
                            else "compiler failed\n"
                        )
                        (build / (label + ".resources.json")).write_text("{}")
                        failing = label == phase
                        return dict(
                            returncode=2
                            if failing and confirmed
                            else None
                            if failing
                            else 0,
                            refusal="wall-time-exceeded"
                            if failing and not confirmed
                            else None,
                            cleanup_errors=[]
                            if not failing or confirmed
                            else ["child exit is unconfirmed"],
                        )

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
                        with self.assertRaises(ValueError):
                            install.install("cargo-mutants", root / "tool")
                    build = builds[0]
                    self.assertTrue(build.exists())
                    self.assertTrue((build / (phase + ".log")).exists())
                    self.assertTrue((build / (phase + ".resources.json")).exists())
                    for name in ("target", "build"):
                        self.assertEqual((build / name).exists(), not confirmed)
                    self.assertEqual((owner / "marker").read_text(), "keep")
                    __import__("shutil").rmtree(build)

    def test_an_exception_without_the_attempted_phase_report_retains_products(self):
        with tempfile.TemporaryDirectory() as temporary:
            builds = []

            def run(command, build, **options):
                build = Path(build)
                builds.append(build)
                (build / "target").mkdir(exist_ok=True)
                (build / "target/product").write_text("possibly active")
                if options["label"] == "install":
                    (build / "install.log").write_text("install complete")
                    return dict(returncode=0, refusal=None, cleanup_errors=[])
                raise OSError("version cleanup report unavailable")

            with patch.object(install.process, "run", side_effect=run):
                with self.assertRaises(OSError):
                    install.install("cargo-mutants", Path(temporary) / "tool")
            self.assertTrue((builds[0] / "target/product").exists())
            __import__("shutil").rmtree(builds[0])
