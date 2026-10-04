# SPDX-License-Identifier: MPL-2.0
"""Published setup verification executes the selected platform's original code block."""
import importlib.util
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("published_install", ROOT / "scripts/verify-published-install.py")
install = importlib.util.module_from_spec(spec)
spec.loader.exec_module(install)


class PublishedInstallTests(unittest.TestCase):
    def test_platform_block_is_preserved_exactly(self):
        document = '```sh\nVERSION=18.0.0\necho "$VERSION"\n```\n```powershell\n$Version = "18.0.0"\n& ".\\htmlcut.exe" --version\n```'
        self.assertEqual(install.install_block(document, False), 'VERSION=18.0.0\necho "$VERSION"\n')
        self.assertEqual(install.install_block(document, True), '$Version = "18.0.0"\n& ".\\htmlcut.exe" --version\n')
        with self.assertRaises(ValueError):
            install.install_block("```sh\ntrue\n```", True)

    def test_native_workflow_discovers_the_install_execution_and_retains_it(self):
        workflow = (ROOT / ".github/workflows/ci.yml").read_text()
        self.assertEqual(workflow.count("python3 scripts/verify-published-install.py --output dist/published-install-example.json"), 1)
        self.assertIn("path: dist/", workflow)


if __name__ == "__main__":
    unittest.main()
