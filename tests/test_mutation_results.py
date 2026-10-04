# SPDX-License-Identifier: MPL-2.0
"""Equal counters cannot substitute for complete, source-bound mutant identities."""
import copy
import importlib.util
import json
import shutil
from pathlib import Path
import tempfile
import unittest

spec = importlib.util.spec_from_file_location("mutation_results", Path(__file__).resolve().parents[1] / "scripts/verify-mutation-results.py")
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class MutationResultsTest(unittest.TestCase):
    def fixture(self, root):
        mutant = dict(package="htmlcut-core", file="crates/htmlcut-core/src/execution.rs",
            span=dict(start=dict(line=1, column=1), end=dict(line=1, column=2)),
            replacement="false", genre="FnValue", name="fixture mutant")
        plan = root / "plan.json"
        metadata = root / "source.json"
        plan.write_text(json.dumps([mutant]))
        module.record_plan(plan, "a" * 40, metadata)
        folder = root / "artifacts/cargo-mutants-shard-0-of-1/mutants.out"
        folder.mkdir(parents=True)
        (folder / "mutants.json").write_text(json.dumps([mutant]))
        (folder / "source-commit.txt").write_text("a" * 40 + "\n")
        outcome = dict(outcomes=[dict(scenario="Baseline", summary="Success",
            phase_results=[dict(phase=phase, process_status="Success") for phase in ["Build", "Test"]]),
            dict(scenario=dict(Mutant=mutant), summary="CaughtMutant")],
            total_mutants=1, caught=1, unviable=0, missed=0, timeout=0, end_time="complete")
        (folder / "outcomes.json").write_text(json.dumps(outcome))
        args = [plan, metadata, "a" * 40,
            json.dumps([dict(artifact_name="cargo-mutants-shard-0-of-1", selector="0/1")]), root / "artifacts"]
        return args, folder, outcome

    def test_complete_evidence_and_each_false_green(self):
        with tempfile.TemporaryDirectory() as directory:
            args, folder, original = self.fixture(Path(directory))
            self.assertEqual(module.verify(*args), 1)
            variants = []
            changed = copy.deepcopy(original); changed["outcomes"][1]["scenario"]["Mutant"]["replacement"] = "true"; variants.append(changed)
            changed = copy.deepcopy(original); changed["outcomes"].append(changed["outcomes"][1]); variants.append(changed)
            changed = copy.deepcopy(original); changed["outcomes"][0]["summary"] = "Failure"; variants.append(changed)
            changed = copy.deepcopy(original); changed["outcomes"][0]["phase_results"][1]["process_status"] = "Failure"; variants.append(changed)
            changed = copy.deepcopy(original); changed["outcomes"][1]["summary"] = "MissedMutant"; changed.update(caught=0, missed=1); variants.append(changed)
            changed = copy.deepcopy(original); changed["outcomes"][1]["summary"] = "Timeout"; changed.update(caught=0, timeout=1); variants.append(changed)
            changed = copy.deepcopy(original); changed["caught"] = 0; variants.append(changed)
            changed = copy.deepcopy(original); changed["end_time"] = None; variants.append(changed)
            for changed in variants:
                (folder / "outcomes.json").write_text(json.dumps(changed))
                with self.assertRaises(ValueError): module.verify(*args)
            (folder / "outcomes.json").write_text(json.dumps(original))
            (folder / "source-commit.txt").write_text("b" * 40)
            with self.assertRaises(ValueError): module.verify(*args)
            (folder / "source-commit.txt").write_text("a" * 40)
            args[2] = "b" * 40
            with self.assertRaises(ValueError): module.verify(*args)

    def test_empty_plan_is_explicit_and_bound(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory); plan = root / "plan.json"; metadata = root / "source.json"
            plan.write_text("[]"); module.record_plan(plan, "a" * 40, metadata)
            self.assertEqual(module.verify(plan, metadata, "a" * 40, "[]", root / "missing"), 0)
            plan.write_text('[{"invalid":true}]')
            with self.assertRaises((ValueError, KeyError)): module.verify(plan, metadata, "a" * 40, "[]", root / "missing")

    def test_duplicate_json_keys_are_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "evidence.json"
            path.write_text('{"source":1,"source":2}')
            with self.assertRaises(ValueError): module.read(path)

    def test_equal_global_count_cannot_hide_cross_shard_duplicate(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory); args, folder, outcomes = self.fixture(root)
            original = outcomes["outcomes"][1]["scenario"]["Mutant"]
            second = copy.deepcopy(original); second["name"] = "second identity"
            args[0].write_text(json.dumps([original, second]))
            module.record_plan(args[0], args[2], args[1])
            first = root / "artifacts/cargo-mutants-shard-0-of-2"
            folder.parent.rename(first)
            shutil.copytree(first, root / "artifacts/cargo-mutants-shard-1-of-2")
            args[3] = json.dumps([dict(artifact_name=f"cargo-mutants-shard-{i}-of-2", selector=f"{i}/2") for i in range(2)])
            with self.assertRaisesRegex(ValueError, "cross-shard"):
                module.verify(*args)
