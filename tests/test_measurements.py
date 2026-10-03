"""Operational measurements reject wrong answers and ambiguous/untyped RSS records."""
import json
from pathlib import Path
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'evaluation'))
from measurements import paired, peak_rss_bytes, warm


class MeasurementControls(unittest.TestCase):
    def test_host_rss_units_are_bytes_and_ambiguity_is_rejected(self):
        self.assertEqual(peak_rss_bytes(' 1064960 maximum resident set size\n', 'Darwin'), 1064960)
        self.assertEqual(peak_rss_bytes('Maximum resident set size (kbytes): 1024\n', 'Linux'), 1048576)
        for text, system in [('', 'Darwin'), ('-1 maximum resident set size', 'Darwin'),
                             ('1 maximum resident set size\n2 maximum resident set size\n', 'Darwin'),
                             ('1 maximum resident set size', 'Windows')]:
            with self.assertRaises(ValueError):
                peak_rss_bytes(text, system)

    def test_fresh_launches_check_complete_answers_and_keep_each_sample(self):
        commands = {'python': [sys.executable, '-c', 'print("[1,2]")']}
        timing, outputs = paired(commands, [1, 2], lambda name, value: json.loads(value), warmups=1, repeats=2)
        self.assertEqual(len(timing['python']['samples_ns']), 2)
        self.assertEqual(json.loads(outputs['python']), [1, 2])
        with self.assertRaises(ValueError):
            paired(commands, [1], lambda name, value: json.loads(value), warmups=0, repeats=1)

    def test_warm_measurement_checks_every_answer(self):
        self.assertEqual(len(warm(lambda: ['value'], ['value'], warmups=1, repeats=2)['samples_ns']), 2)
        with self.assertRaises(ValueError):
            warm(lambda: [], ['value'], warmups=0, repeats=1)


if __name__ == '__main__':
    unittest.main()
