# SPDX-License-Identifier: MPL-2.0
"""The actual frozen-corpus command must reject edits and deletion, allowing additions."""
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]

class FrozenCorpusTest(unittest.TestCase):
    def test_byte_changes_and_missing_files_are_rejected_against_an_isolated_tag(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            (root/'scripts').mkdir()
            shutil.copy2(ROOT/'scripts/check-conformance-frozen.sh',root/'scripts/check-conformance-frozen.sh')
            (root/'Cargo.toml').write_text('[workspace.package]\nversion="22.0.0"\n')
            case=root/'conformance/contract/case'
            case.mkdir(parents=True)
            page=case/'page.html';page.write_bytes(b'\xff\r\n')
            expected=case/'expected.json';expected.write_bytes(b'"value"\n')
            def git(*args):
                return subprocess.run(['git',*args],cwd=root,check=True,capture_output=True)
            def check():
                return subprocess.run(['bash','scripts/check-conformance-frozen.sh'],cwd=root,capture_output=True,text=True)
            git('init','-b','main');git('config','user.name','Corpus fixture');git('config','user.email','fixture@example.invalid')
            git('add','.');git('commit','-m','Corpus baseline')
            self.assertEqual(check().returncode,0)
            git('tag','v22.0.0')
            self.assertEqual(check().returncode,0)
            (case/'additional.json').write_text('{}')
            self.assertEqual(check().returncode,0)
            expected.write_bytes(b'"wrong"\n')
            refused=check();self.assertNotEqual(refused.returncode,0);self.assertIn('expected.json',refused.stderr)
            expected.write_bytes(b'"value"\n');page.unlink()
            refused=check();self.assertNotEqual(refused.returncode,0);self.assertIn('page.html',refused.stderr)
            page.write_bytes(b'\xff\n')
            self.assertNotEqual(check().returncode,0)
            page.write_bytes(b'\xff\r\n');self.assertEqual(check().returncode,0)
            git('tag','-d','v22.0.0')
