---
afad: "4.0"
version: "17.0.0"
domain: CLI
updated: "2026-10-03"
route:
  keywords: [cli, extract, run, inspect, describe, schema, snapshot bundles, raw output]
  questions: ["what commands does htmlcut-cli expose?", "what does htmlcut schema include?", "how do extraction and source slicing outputs work?"]
---

# CLI

Commands are `extract`, `run`, `inspect`, `describe`, and `schema`. Inline requests compile through the same current plan contract as files and Rust constructors. No CLI Rust library, retired aliases or old-wire conversion is supported.

```sh
htmlcut describe
htmlcut describe extract
htmlcut extract --file fixture.html --css '#amount'
htmlcut extract --file fixture.html --css '#amount' --raw
htmlcut extract --file fixture.html --css 'a.item' --attribute href --match all
htmlcut extract --stdin --plan amount.plan.json
htmlcut inspect --file fixture.html --css p --samples 3
htmlcut schema htmlcut.extraction.plan
```

Exactly one file or intentional stdin supplies strict UTF-8; BOM, CRLF and NUL remain in the accepted snapshot. Invalid UTF-8 fails. No network, environment-backed source, charset conversion, HTML charset/base sniffing or browser execution occurs. `--base-url` supplies explicit absolute HTTP(S) metadata without userinfo.

Default single selection rejects zero or multiple candidates. `--match nth --index 1` is explicitly positional; `--match all --min 0` permits an empty array. `--attribute NAME` chooses attribute projection directly and conflicts with another explicit projection. `--raw` requires one flat string, permits empty content, and adds no LF. Records/raw is rejected before consuming the source when the plan determines it. Default data JSON is identical for terminals and pipes.

Records, guards, exclusions and transforms use plan files. A source-dependent failure rejects the entire execution before successful stdout/artifact publication. Typed errors go to stderr, with bounded safe option/I/O causes and applicable numeric row/field positions rather than user text or mandatory digest envelopes.

`--receipt FILE` publishes the fixed complete execution receipt separately; `--bundle FILE` saves a self-contained snapshot/plan/manifest archive. They are mutually exclusive. `run BUNDLE` accepts delivery options only, recomputes from bundled bytes/configuration and compares its full receipt. No original files, network, environment substitution, nested run, archive unpacking or source/plan overrides occur. Bundle contents are opt-in data and can contain source or explicit metadata supplied by the caller.

`inspect` requires `--css`; it establishes a complete match count and defaults to three samples (range 1–10). Samples contain bounded tags, at most eight exact supported attribute names and 160 Unicode scalar values of normalized literal text, with separate completeness flags. The encoded response is capped at 16 KiB. A limit while establishing the count is failure, never an approximate successful count. There are no handles, cursors, proposals or preview-plan modes.

`--output FILE` publishes data instead of stdout; replacement requires `--overwrite`. Input/destination collisions and destination symlinks/special files fail. Source, plan and bundle paths validate the opened regular handle; intentional streams use stdin. Regular input symlinks remain supported. Files use bounded staging and atomic native create/replace. All requested artifacts are staged before any commit; receipt/bundle commits precede data delivery. Several files/stdout are not a transaction. A later failure may leave an execution sidecar or bytes already delivered to a pipe and returns nonzero. File-only or empty-raw output does not require an unused stdout descriptor. No untested power-loss durability is promised.

Exit classes: 0 success/help; 2 invalid configuration/schema/selector/regex/bundle structure; 3 unmet selection/guard/attribute/representation or replay expectation; 4 resource exhaustion; 5 real acquisition/UTF-8/delivery I/O; 6 internal invariants. Known malformed archive bytes are distinct from a failing underlying read/seek.
