# SPDX-License-Identifier: MPL-2.0
"""Documented accepted selector examples must have executable conformance evidence."""
import json
from pathlib import Path
import re
import unittest
ROOT = Path(__file__).resolve().parents[1]


def selectors(node):
    if isinstance(node, str):
        yield node
    elif isinstance(node, dict):
        for key in ("select", "until"):
            if key in node:
                yield node[key]
        yield from node.get("exclude", [])
        for key in ("where", "expect"):
            for predicate in node.get(key, []):
                if isinstance(predicate, str):
                    yield predicate
                elif "select" in predicate:
                    yield predicate["select"]
        for field in node.get("fields", {}).values():
            yield from selectors(field)


class ContractDialectsTest(unittest.TestCase):
    def test_each_documented_accepted_example_is_in_the_dialect_corpus(self):
        text = (ROOT / "docs/contract.md").read_text()
        accepted = text.split("**Accepted selectors.**", 1)[1].split("**Case.**", 1)[0]
        examples = set(re.findall(r"`([^`]+)`", accepted))
        self.assertTrue(examples)
        tested = set()
        for contract in (ROOT / "conformance/contract").glob("1*/contract.json"):
            tested.update(selectors(json.loads(contract.read_text())))
        self.assertFalse(examples - tested, f"No conformance evidence for {examples - tested}")
