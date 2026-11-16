# SPDX-License-Identifier: AGPL-3.0-only
"""The owned overlay must qualify global artifacts before their upload."""
import importlib.util
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location("workflow_generator", Path(__file__).with_name("generate-release-workflow.py"))
generator = importlib.util.module_from_spec(spec)
spec.loader.exec_module(generator)

BASE = '''# Copyright 2022-2024, axodotdev
# SPDX-License-Identifier: MIT or Apache-2.0
jobs:
  build-local-artifacts:
      - name: "Upload artifacts"
        uses: actions/upload-artifact@v7
        with:
          path: target/distrib/
  build-global-artifacts:
      - name: Build global artifacts
        run: dist build --artifacts=global
      - name: "Upload artifacts"
        uses: actions/upload-artifact@v7
        with:
          path: target/distrib/
  # Determines if we should publish/announce
  host:
'''


class WorkflowGenerationTests(unittest.TestCase):
    def test_qualification_precedes_only_the_global_upload(self):
        output = generator.overlay(BASE)
        global_job = output.split("  build-global-artifacts:\n", 1)[1]
        self.assertLess(global_job.index("dist build"), global_job.index("Qualify the exact"))
        self.assertLess(global_job.index("Qualify the exact"), global_job.index('name: "Upload artifacts"'))
        self.assertNotIn("Qualify the exact", output.split("  build-global-artifacts:\n", 1)[0])
        self.assertIn("# Copyright 2022-2024, axodotdev", output)
        self.assertIn("(MIT OR Apache-2.0) AND AGPL-3.0-only", output)
        self.assertIn("HTMLCut qualification overlay: Copyright", output)
        with self.assertRaisesRegex(ValueError, "unmodified"):
            generator.overlay(output)

    def test_missing_or_ambiguous_placement_is_refused(self):
        for text in [BASE.replace('      - name: "Upload artifacts"\n', ''), BASE.replace('      - name: "Upload artifacts"\n', '      - name: "Upload artifacts"\n' * 2), BASE.replace('  build-global-artifacts:\n', '')]:
            with self.assertRaises(ValueError):
                generator.overlay(text)

    def test_missing_or_changed_upstream_license_is_refused(self):
        for text in [BASE.replace("# SPDX-License-Identifier: MIT or Apache-2.0\n", ""), BASE.replace("MIT or Apache-2.0", "Unrecognized terms")]:
            with self.assertRaisesRegex(ValueError, "upstream cargo-dist workflow license"):
                generator.overlay(text)

    def test_only_the_owned_dirty_scope_is_overridden_in_the_copy(self):
        original = '[dist]\nallow-dirty = ["ci"]\n'
        self.assertEqual(generator.upstream_configuration(original), '[dist]\nallow-dirty = []\n')
        for text in ['[dist]\n', original + 'allow-dirty = ["ci"]\n', '[dist]\nallow-dirty = ["ci", "msi"]\n', original + 'allow-dirty = []\n']:
            with self.assertRaisesRegex(ValueError, "exactly the owned"):
                generator.upstream_configuration(text)


if __name__ == "__main__":
    unittest.main()
