#!/usr/bin/env python3
"""Bind completed mutation evidence to the exact source and selected planner inventory."""
import argparse
import hashlib
import json
from pathlib import Path
import re

MAX_JSON = 64 * 1024 * 1024


def closed_object(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError("Duplicate evidence object key")
        result[key] = value
    return result


def read(path):
    path = Path(path)
    if path.is_symlink() or not path.is_file():
        raise ValueError(f"Expected regular evidence file: {path}")
    with path.open("rb") as stream:
        raw = stream.read(MAX_JSON + 1)
    if len(raw) > MAX_JSON:
        raise ValueError("Mutation evidence exceeds its byte bound")
    return raw, json.loads(raw, object_pairs_hook=closed_object)


def identities(mutants):
    if not isinstance(mutants, list):
        raise ValueError("Mutant inventory must be an array")
    values = []
    for mutant in mutants:
        if not isinstance(mutant, dict):
            raise ValueError("Invalid mutant identity")
        value = {key: mutant[key] for key in
                 ("package", "file", "span", "replacement", "genre", "name")}
        values.append(json.dumps(value, sort_keys=True, separators=(",", ":")))
    if len(values) != len(set(values)):
        raise ValueError("Duplicate mutant identity")
    return set(values)


def record_plan(plan, source, destination):
    if not re.fullmatch(r"[0-9a-f]{40}", source):
        raise ValueError("Expected exact source commit")
    raw, planned = read(plan)
    count = len(identities(planned))
    Path(destination).write_text(json.dumps(dict(source_commit=source, count=count,
        sha256=hashlib.sha256(raw).hexdigest()), indent=2) + "\n")


def verify(plan, metadata, source, shard_plan, root):
    if not re.fullmatch(r"[0-9a-f]{40}", source):
        raise ValueError("Expected exact source commit")
    raw, planned = read(plan)
    _, binding = read(metadata)
    wanted = identities(planned)
    if type(binding.get("count")) is not int or binding != dict(source_commit=source, count=len(wanted), sha256=hashlib.sha256(raw).hexdigest()):
        raise ValueError("Mutation plan source/hash/count binding differs")
    shards = json.loads(shard_plan, object_pairs_hook=closed_object)
    if not isinstance(shards, list) or bool(shards) != bool(wanted):
        raise ValueError("Empty/nonempty mutation plan and shards disagree")
    root = Path(root)
    expected_names = {s["artifact_name"] for s in shards}
    if len(expected_names) != len(shards):
        raise ValueError("Duplicate shard artifact")
    canonical = {f"cargo-mutants-shard-{i}-of-{len(shards)}" for i in range(len(shards))}
    if expected_names != canonical or any(s["selector"] != s["artifact_name"].replace("cargo-mutants-shard-", "").replace("-of-", "/") for s in shards):
        raise ValueError("Shard indices/selectors differ from complete canonical plan")
    actual = {p.name for p in root.iterdir()} if root.exists() else set()
    if actual != expected_names:
        raise ValueError("Missing or unexpected shard artifacts")
    seen = set()
    counts = {"CaughtMutant": "caught", "Unviable": "unviable", "MissedMutant": "missed", "Timeout": "timeout"}
    for shard in shards:
        name = shard["artifact_name"]
        if not re.fullmatch(r"cargo-mutants-shard-[0-9]+-of-[1-9][0-9]*", name):
            raise ValueError("Unsafe shard artifact name")
        folder = root / name / "mutants.out"
        with (folder / "source-commit.txt").open() as stream:
            if stream.read(42).strip() != source:
                raise ValueError("Shard source differs from planner source")
        _, inventory = read(folder / "mutants.json")
        _, outcomes = read(folder / "outcomes.json")
        selected = identities(inventory)
        rows = outcomes["outcomes"]
        baselines = [r for r in rows if r["scenario"] == "Baseline"]
        if len(baselines) != 1 or baselines[0]["summary"] != "Success":
            raise ValueError("Shard lacks exactly one successful baseline")
        phases = baselines[0]["phase_results"]
        if len(phases) != 2 or {p["phase"] for p in phases} != {"Build", "Test"} or any(p["process_status"] != "Success" for p in phases):
            raise ValueError("Shard baseline build/test phases did not succeed")
        completed = [r for r in rows if r["scenario"] != "Baseline"]
        tested = identities([r["scenario"]["Mutant"] for r in completed])
        if selected != tested or seen & tested:
            raise ValueError("Shard planned/tested mismatch or cross-shard duplicate")
        totals = {key: 0 for key in counts.values()}
        for result in completed:
            totals[counts[result["summary"]]] += 1
        if not isinstance(outcomes.get("end_time"), str) or not outcomes["end_time"] or type(outcomes["total_mutants"]) is not int or outcomes["total_mutants"] != len(tested):
            raise ValueError("Shard incomplete or total differs")
        if any(type(outcomes[key]) is not int or outcomes[key] != count for key, count in totals.items()):
            raise ValueError("Shard counters differ from completed outcomes")
        if totals["missed"] or totals["timeout"]:
            raise ValueError("Missed or timed-out mutants remain")
        seen |= tested
    if seen != wanted:
        raise ValueError("Global tested identities differ from selected planner inventory")
    return len(seen)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--plan", required=True)
    parser.add_argument("--metadata", required=True)
    parser.add_argument("--source-sha", required=True)
    parser.add_argument("--record-plan", action="store_true")
    parser.add_argument("--shards")
    parser.add_argument("--artifacts")
    args = parser.parse_args()
    if args.record_plan:
        record_plan(args.plan, args.source_sha, args.metadata)
    else:
        if not args.shards or not args.artifacts:
            parser.error("Verification requires --shards and --artifacts")
        print(f"Verified {verify(args.plan, args.metadata, args.source_sha, args.shards, args.artifacts)} exact mutant identities.")


if __name__ == "__main__":
    main()
