import importlib.util
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location('release_notes', Path(__file__).resolve().parents[1] / 'scripts/release-notes.py')
notes = importlib.util.module_from_spec(spec)
spec.loader.exec_module(notes)

class NotesTest(unittest.TestCase):
    def test_exact_section_excludes_neighbors_and_supports_explicit_preview(self):
        text = '# Changes\n## [Unreleased]\nfuture\n## [15.0.0] - date\nbody\n## [14.0.0]\nold\n'
        self.assertEqual(notes.section(text, '15.0.0'), 'body\n')
        self.assertEqual(notes.section(text, 'Unreleased'), 'future\n')
        for text in ['## [15.0.0]\n', '## [14.0.0]\nold', '## [15.0.0]\na\n## [15.0.0]\nb']:
            with self.assertRaises(ValueError):
                notes.section(text, '15.0.0')

if __name__ == '__main__':
    unittest.main()
