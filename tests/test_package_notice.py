# SPDX-License-Identifier: MPL-2.0
"""Native attribution keeps original notices and rejects incomplete graph coverage."""
import importlib.util
import io
import json
from pathlib import Path
import tempfile
import tarfile
import unittest
from unittest.mock import patch

MODULE = Path(__file__).resolve().parents[1] / "scripts/generate-package-notice.py"
spec = importlib.util.spec_from_file_location("package_notice", MODULE)
notice = importlib.util.module_from_spec(spec)
spec.loader.exec_module(notice)


class PackageNoticeTests(unittest.TestCase):
    def test_runtime_terms_source_pins_and_musl_patches_are_retained(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "rust-toolchain.toml").write_text('[toolchain]\nchannel = "2.0.0"\n')
            copyright = root / "share/doc/rust/COPYRIGHT-library.html"
            copyright.parent.mkdir(parents=True)
            copyright.write_text('<h1>Copyright notices for The Rust Standard Library</h1>'
                                 '<pre>Copyright &lt;Fixture&gt;\nPermission is hereby granted. &amp;</pre>')
            metadata = "rustc 2.0.0\nrelease: 2.0.0\ncommit-hash: " + "a" * 40 + "\n"
            original = "Copyright Native Authors\nPermission is hereby granted verbatim.\n"
            archive = io.BytesIO()
            with tarfile.open(fileobj=archive, mode="w:gz") as tar:
                entry = tarfile.TarInfo("musl-1.2.5/COPYRIGHT")
                entry.size = len(original.encode())
                tar.addfile(entry, io.BytesIO(original.encode()))

            def fetch(url, timeout):
                if "api.github.com" in url:
                    return io.BytesIO(json.dumps(dict(sha="b" * 40,
                        submodule_git_url="https://github.com/rust-lang/llvm-project.git")).encode())
                if url.endswith("musl.sh"):
                    return io.BytesIO(b"MUSL=musl-1.2.5\n# Applied upstream patches\n")
                if url.endswith(".tar.gz"):
                    return io.BytesIO(archive.getvalue())
                return io.BytesIO(original.encode())

            with patch.object(notice.subprocess, "check_output", side_effect=[metadata, str(root)]), \
                    patch.object(notice, "urlopen", side_effect=fetch):
                text = notice.runtime_notice(root, "x86_64-unknown-linux-musl")
            self.assertIn("Copyright <Fixture>\nPermission is hereby granted. &", text)
            self.assertEqual(text.count(original.rstrip()), 3)
            self.assertIn("/tree/" + "a" * 40, text)
            self.assertIn("/" + "b" * 40 + "/compiler-rt/LICENSE.TXT", text)
            self.assertIn("/" + "b" * 40 + "/libunwind/LICENSE.TXT", text)
            self.assertIn("musl-1.2.5.tar.gz", text)
            self.assertIn("Rust build and applied patches:", text)

    def test_runtime_notice_refuses_wrong_compiler_and_missing_copyright_material(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "rust-toolchain.toml").write_text('[toolchain]\nchannel = "2.0.0"\n')
            for release in ["1.0.0", "2.0.0"]:
                metadata = f"rustc {release}\nrelease: {release}\ncommit-hash: " + "a" * 40 + "\n"
                with patch.object(notice.subprocess, "check_output", side_effect=[metadata, str(root)]):
                    with self.assertRaises((ValueError, FileNotFoundError)):
                        notice.runtime_notice(root, "aarch64-apple-darwin")

    def test_inventory_runs_the_authoritative_script(self):
        rows = [line.split() for line in notice.shell_inventory(MODULE.parents[1]).splitlines()]
        self.assertEqual(len([row for row in rows if row[0] == "cargo-about"]), 1)
        self.assertTrue(all(len(row) == 3 for row in rows))

    def test_git_bash_is_preferred_to_the_windows_wsl_launcher(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory) / "Program Files"
            binary = root / "Git/bin/bash.exe"
            binary.parent.mkdir(parents=True)
            binary.touch()
            with patch.dict(notice.os.environ, {"ProgramW6432": str(root)}, clear=True):
                self.assertEqual(notice.bash_program(), str(binary))
            with patch.dict(notice.os.environ, {}, clear=True):
                self.assertEqual(notice.bash_program(), "bash")

    def report(self, directory):
        package = {"id": "fixture@1.0.0", "name": "fixture", "version": "1.0.0",
                   "source": "registry", "manifest_path": str(directory / "Cargo.toml")}
        return {"crates": [{"package": package}], "licenses": [{"name": "MIT License", "id": "MIT",
                "text": "Copyright Fixture Authors\nPermission notice retained verbatim.",
                "used_by": [{"crate": package}]}]}

    def test_original_terms_copyright_and_source_locations_reach_native_notice(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            copyright = "Additional copyright and attribution\nSecond author.\n"
            (root / "COPYRIGHT").write_text(copyright)
            report = self.report(root)
            output = notice.render_notice(report, "Project scope", "a" * 40, "18.0.0", "target")
            self.assertIn(report["licenses"][0]["text"], output)
            self.assertIn(copyright, output)
            self.assertIn("https://crates.io/api/v1/crates/fixture/1.0.0/download", output)
            self.assertIn("/tree/" + "a" * 40, output)
            self.assertIn("htmlcut-source-18.0.0.tar.gz", output)

    def test_missing_package_attribution_or_permission_text_is_refused(self):
        with tempfile.TemporaryDirectory() as directory:
            for missing in ("package", "text"):
                report = self.report(Path(directory))
                if missing == "package":
                    report["licenses"][0]["used_by"] = []
                else:
                    report["licenses"][0]["text"] = " "
                with self.assertRaises(ValueError):
                    notice.render_notice(report, "Project scope", "a" * 40, "18.0.0", "target")


if __name__ == "__main__":
    unittest.main()
