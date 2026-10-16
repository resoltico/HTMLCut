# SPDX-License-Identifier: MPL-2.0
"""Cache only public inputs and fail closed when native tool inputs are unknown."""

from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
from tools.htmlcut_maintenance import config


class CacheInputTests(unittest.TestCase):
    def environment(self, temporary):
        return {
            "RUNNER_TEMP": temporary,
            "RUNNER_OS": "Linux",
            "RUNNER_ARCH": "X64",
            "ImageOS": "ubuntu24",
            "ImageVersion": "20261015.1",
            "CARGO_HOME": str(Path(temporary) / "cargo home"),
        }

    def test_only_public_registry_archives_and_index_are_selected(self):
        with tempfile.TemporaryDirectory() as temporary:
            home = Path(temporary) / "owner Cargo home"
            home.mkdir()
            secret = home / "credentials.toml"
            secret.write_text("owner sentinel")
            paths = config.cargo_download_paths({"CARGO_HOME": str(home)})
            self.assertEqual(
                paths,
                [
                    str(home / "registry" / kind / "index.crates.io-*")
                    for kind in ("index", "cache")
                ],
            )
            self.assertTrue(all(str(secret) != path for path in paths))
            self.assertEqual(secret.read_text(), "owner sentinel")

    def test_hosted_platform_and_tool_select_a_new_job_owned_root(self):
        with tempfile.TemporaryDirectory() as temporary:
            environment = self.environment(temporary)
            paths = config.ci_tool_paths("cargo-nextest", environment)
            expected = Path(temporary).resolve() / "htmlcut-cargo-tools/cargo-nextest"
            self.assertEqual(Path(paths["root"]), expected)
            self.assertEqual(Path(paths["binary"]), expected / "bin/cargo-nextest")
            self.assertEqual(paths["platform"], "Linux-X64-ubuntu24-20261015.1")
            self.assertFalse(expected.exists())
            environment["RUNNER_OS"] = "Windows"
            environment["ImageOS"] = "win22"
            self.assertTrue(
                config.ci_tool_paths("cargo-nextest", environment)["binary"].endswith(
                    "cargo-nextest.exe"
                )
            )

    def test_missing_identity_unknown_tool_or_build_override_cannot_reuse_a_cache(self):
        with tempfile.TemporaryDirectory() as temporary:
            base = self.environment(temporary)
            for key in (
                "RUNNER_TEMP",
                "RUNNER_OS",
                "RUNNER_ARCH",
                "ImageOS",
                "ImageVersion",
            ):
                environment = dict(base)
                del environment[key]
                with (
                    self.subTest(missing=key),
                    self.assertRaisesRegex(ValueError, "image and platform"),
                ):
                    config.ci_tool_paths("cargo-about", environment)
            for key in (
                "RUSTFLAGS",
                "CARGO_ENCODED_RUSTFLAGS",
                "RUSTC_WRAPPER",
                "CARGO_BUILD_TARGET",
                "CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER",
                "CARGO_PROFILE_RELEASE_LTO",
                "CC",
                "MACOSX_DEPLOYMENT_TARGET",
                "CARGO_REGISTRIES_CRATES_IO_INDEX",
                "CARGO_REGISTRY_DEFAULT",
                "CARGO_SOURCE_CRATES_IO_REPLACE_WITH",
            ):
                with (
                    self.subTest(override=key),
                    self.assertRaisesRegex(ValueError, "build overrides"),
                ):
                    config.ci_tool_paths("cargo-about", {**base, key: "unsupported"})
            with self.assertRaisesRegex(ValueError, "unknown contributor"):
                config.ci_tool_paths("not-a-tool", base)
            self.assertEqual(list(Path(temporary).iterdir()), [])

    def test_cargo_home_configuration_refuses_without_opening_or_changing_it(self):
        for name in ("config", "config.toml"):
            with self.subTest(name=name), tempfile.TemporaryDirectory() as temporary:
                environment = self.environment(temporary)
                home = Path(environment["CARGO_HOME"])
                home.mkdir()
                candidate = home / name
                contents = '[build]\nrustflags = ["--cfg", "cache_input_probe"]\n'
                candidate.write_text(contents)
                original_open = Path.open

                def forbid_config_read(path, *args, **kwargs):
                    if path == candidate:
                        raise AssertionError(
                            "owner configuration must not be read or hashed"
                        )
                    return original_open(path, *args, **kwargs)

                with patch.object(Path, "open", forbid_config_read):
                    with self.assertRaisesRegex(ValueError, "Cargo-home configuration"):
                        config.ci_tool_paths("cargo-about", environment)
                self.assertEqual(candidate.read_text(), contents)
                self.assertFalse((Path(temporary) / "htmlcut-cargo-tools").exists())

    def test_relative_cargo_home_cannot_change_meaning_in_the_owned_build_cwd(self):
        with tempfile.TemporaryDirectory() as temporary:
            environment = {**self.environment(temporary), "CARGO_HOME": "relative-home"}
            with self.assertRaisesRegex(ValueError, "absolute Cargo home"):
                config.ci_tool_paths("cargo-about", environment)

    def test_nextest_commit_omission_build_override_refuses_even_when_empty(self):
        with tempfile.TemporaryDirectory() as temporary:
            for value in ("1", ""):
                with (
                    self.subTest(value=value),
                    self.assertRaisesRegex(ValueError, "CFG_OMIT_COMMIT_HASH"),
                ):
                    config.ci_tool_paths(
                        "cargo-nextest",
                        {**self.environment(temporary), "CFG_OMIT_COMMIT_HASH": value},
                    )

    def test_existing_owner_root_is_never_adopted_or_deleted(self):
        with tempfile.TemporaryDirectory() as temporary:
            paths = config.ci_tool_paths("cargo-deny", self.environment(temporary))
            root = Path(paths["root"])
            root.mkdir(parents=True)
            marker = root / "owner"
            marker.write_text("preserve")
            with self.assertRaisesRegex(ValueError, "must be new"):
                config.ci_tool_paths("cargo-deny", self.environment(temporary))
            self.assertEqual(marker.read_text(), "preserve")
