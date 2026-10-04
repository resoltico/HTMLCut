# SPDX-License-Identifier: MPL-2.0
"""Native attribution keeps original notices and rejects incomplete graph coverage."""
import importlib.util
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

MODULE = Path(__file__).resolve().parents[1] / "scripts/generate-package-notice.py"
spec = importlib.util.spec_from_file_location("package_notice", MODULE)
notice = importlib.util.module_from_spec(spec)
spec.loader.exec_module(notice)


class PackageNoticeTests(unittest.TestCase):
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
