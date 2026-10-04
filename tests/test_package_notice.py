# SPDX-License-Identifier: MPL-2.0
"""Native attribution keeps original notices and rejects incomplete graph coverage."""
import importlib.util
from pathlib import Path
import tempfile
import unittest

MODULE = Path(__file__).resolve().parents[1] / "scripts/generate-package-notice.py"
spec = importlib.util.spec_from_file_location("package_notice", MODULE)
notice = importlib.util.module_from_spec(spec)
spec.loader.exec_module(notice)


class PackageNoticeTests(unittest.TestCase):
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
