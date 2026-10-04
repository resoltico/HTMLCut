# SPDX-License-Identifier: MPL-2.0
"""An advancing default branch cannot substitute its bytes/version for a requested release ref."""
import subprocess
from pathlib import Path
import tempfile
import unittest
import zipfile

ROOT = Path(__file__).resolve().parents[1]

class SourceTest(unittest.TestCase):
    def test_tag_syntax_is_closed_before_git_or_shell_expansion(self):
        script = ROOT / "scripts/release-tag.sh"
        for tag in ["main", "v01.0.0", "v15.0.0-rc1", "v15.0.0+meta", "v15.$(false).0", ""]:
            result = subprocess.run(["bash", str(script), tag], cwd=ROOT,
                env={**__import__('os').environ, "RELEASE_TAG": "", "GITHUB_REF_NAME": ""}, capture_output=True)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn(b"canonical stable", result.stderr)

    def test_moving_main_differs_from_tag_but_archives_and_notes_use_tag(self):
        with tempfile.TemporaryDirectory() as directory:
            repo = Path(directory)
            def git(*args):
                return subprocess.check_output(['git', '-C', str(repo), *args], text=True, stderr=subprocess.DEVNULL).strip()
            git('init', '-b', 'main')
            git('config', 'user.name', 'Release fixture')
            git('config', 'user.email', 'fixture@example.invalid')
            (repo / 'Cargo.toml').write_text('[workspace.package]\nversion = "15.0.0"\n')
            (repo / 'changelog.md').write_text('## [Unreleased]\nfuture\n## [15.0.0]\ncorrect notes\n## [14.0.0]\nold\n')
            (repo / 'payload.txt').write_text('tag source')
            git('add', '.')
            git('commit', '-m', 'release source')
            git('tag', '-a', 'v15.0.0', '-m', 'fixture tag')
            intended = git('rev-parse', 'HEAD')
            shell = f'source "{ROOT / "scripts/common.sh"}"; source "{ROOT / "scripts/release-tag.sh"}"; htmlcut_release_version_for_tag "{ROOT / "scripts"}" "$PWD" v15.0.0'
            bound = subprocess.run(['bash', '-c', shell], cwd=repo,
                env={**__import__('os').environ, 'RELEASE_SOURCE_SHA': '0'*40}, capture_output=True)
            self.assertNotEqual(bound.returncode, 0)
            self.assertIn(b"resolved source commit", bound.stderr)
            (repo / 'Cargo.toml').write_text('[workspace.package]\nversion = "99.0.0"\n')
            (repo / 'payload.txt').write_text('moving main')
            git('commit', '-am', 'advance main')
            self.assertNotEqual(intended, git('rev-parse', 'HEAD'))
            output = repo / 'archives'
            subprocess.run(['bash', str(ROOT / 'scripts/build-source-archives.sh'), 'v15.0.0', str(output)], cwd=repo, check=True, capture_output=True)
            archive = output / 'htmlcut-source-15.0.0.zip'
            with zipfile.ZipFile(archive) as bundle:
                self.assertEqual(bundle.read('htmlcut-source-15.0.0/payload.txt'), b'tag source')
                self.assertIn(b'15.0.0', bundle.read('htmlcut-source-15.0.0/Cargo.toml'))
            self.assertEqual((output / 'htmlcut-source-15.0.0.source-commit').read_text().strip(), intended)
            notes = subprocess.check_output(['python3', str(ROOT / 'scripts/release-notes.py'), '--ref', 'v15.0.0', '--version', '15.0.0'], cwd=repo, text=True)
            self.assertEqual(notes, 'correct notes\n')

if __name__ == '__main__':
    unittest.main()
