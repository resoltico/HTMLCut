# SPDX-License-Identifier: MPL-2.0
"""The owned overlay must qualify global artifacts before their upload."""
import importlib.util
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location("workflow_generator", Path(__file__).with_name("generate-release-workflow.py"))
generator = importlib.util.module_from_spec(spec)
spec.loader.exec_module(generator)

BASE = '''jobs:
  build-local-artifacts:
      - name: "Upload artifacts"
  build-global-artifacts:
      - name: Build global artifacts
        run: dist build --artifacts=global
      - name: "Upload artifacts"
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
        with self.assertRaisesRegex(ValueError, "unmodified"):
            generator.overlay(output)

    def test_missing_or_ambiguous_placement_is_refused(self):
        for text in [BASE.replace('      - name: "Upload artifacts"\n', ''), BASE.replace('      - name: "Upload artifacts"\n', '      - name: "Upload artifacts"\n' * 2), BASE.replace('  build-global-artifacts:\n', '')]:
            with self.assertRaises(ValueError):
                generator.overlay(text)


if __name__ == "__main__":
    unittest.main()
