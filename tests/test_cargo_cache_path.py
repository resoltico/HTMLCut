# SPDX-License-Identifier: MPL-2.0
"""The action receives the real Cargo directory without forbidden dot segments."""
import importlib.util
from pathlib import Path
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("cargo_cache_path", ROOT / "scripts/cargo-build-cache-path.py")
cache = importlib.util.module_from_spec(spec)
spec.loader.exec_module(cache)


class CargoCachePathTests(unittest.TestCase):
    def test_declared_relative_directory_is_resolved_before_glob_consumption(self):
        with tempfile.TemporaryDirectory() as directory:
            canonical = Path(directory).resolve()
            root = canonical / "checkout"
            (root / ".cargo").mkdir(parents=True)
            (root / ".cargo/config.toml").write_text('[build]\nbuild-dir = "../artifacts/./build"\n')
            actual = cache.build_directory(root)
            self.assertEqual(actual, canonical / "artifacts/build")
            self.assertTrue(actual.is_absolute())
            self.assertNotIn("..", actual.parts)
            self.assertNotIn(".", actual.parts)

    def test_missing_declared_build_directory_fails_instead_of_guessing(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / ".cargo").mkdir()
            (root / ".cargo/config.toml").write_text("[build]\n")
            with self.assertRaises(KeyError):
                cache.build_directory(root)


if __name__ == "__main__":
    unittest.main()
