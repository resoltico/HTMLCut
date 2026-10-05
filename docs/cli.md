---
afad: "4.0"
version: "19.2.0"
domain: CLI
updated: "2026-10-05"
route:
  keywords: [cli, extract, run, inspect, describe, schema, snapshot bundles, raw output]
  questions: ["what commands does htmlcut-cli expose?", "what does htmlcut schema include?", "how do extraction and source slicing outputs work?"]
---

# CLI

Commands are `extract`, `run`, `inspect`, `outline`, `describe`, and `schema`. Inline requests compile through the same current plan contract as files and Rust constructors. No CLI Rust library, retired aliases or old-wire conversion is supported.

```sh
htmlcut describe
htmlcut describe extract
htmlcut extract --file fixture.html --css '#amount'
htmlcut extract --file fixture.html --css '#amount' --raw
htmlcut extract --file fixture.html --css 'a.item' --read attribute:href --match all
htmlcut extract --stdin --plan amount.plan.json
htmlcut inspect --file fixture.html --css p --samples 3
htmlcut outline --file fixture.html
htmlcut inspect --file fixture.html --css 'main,article,table' --identifiers
htmlcut schema htmlcut.extraction.plan
```

Exactly one file or intentional stdin supplies strict UTF-8; BOM, CRLF and NUL remain in the accepted snapshot. Invalid UTF-8 fails. No network, environment-backed source, charset conversion, HTML charset/base sniffing or browser execution occurs. `--base-url` supplies explicit absolute HTTP(S) metadata without userinfo.

Default single selection rejects zero or multiple candidates. `--match nth --index 1` is explicitly positional; `--match all --min 0` permits an empty array. `--read READ` chooses `dom_text` (default), `normalized_text`, `markdown`, `resolved_markdown`, `inner_html`, `outer_html`, `attribute:NAME`, `resolved_attribute:NAME` or `source` (slicing only). Relative URL resolution requires explicit base metadata; absolute URLs need no base. `--raw` requires one flat string, permits empty content, and adds no LF. Records/raw is rejected before consuming the source when the plan determines it. Default data JSON is identical for terminals and pipes.

Repeated `--field NAME CSS READ` declares single-valued record fields, with CSS passed as one opaque argument. Suffix `NAME` with `?` to allow zero or one match, returning `null` on absence; a matched node missing a requested attribute still fails. Repeated `--field-exclude NAME CSS` excludes matching descendants from that declared field (use the output field name without `?`). It cannot be combined with `--plan` or an attribute reading. Inline fields conflict with root `--read`. `--following-siblings N` requires fields and includes exactly N following element sibling subtrees (0–63); missing siblings or absorption of another root candidate fail. `:scope` identifies the original anchor. Repeated `--exclude CSS` removes matched descendants from a flat DOM reading; it cannot be combined with fields, source slicing, or a plan file. Repeated `--expect-text CSS TEXT` requires exactly one original-DOM match with that literal text before exclusions or transforms, and works with flat readings or records. The existing plan contract owns both mechanisms. All/nth field selection and advanced guards use plan files. A source-dependent failure rejects the entire execution before successful stdout/artifact publication. Typed errors go to stderr, with bounded safe option/I/O causes, a declared-member path for plan shape failures, and applicable numeric row/field positions rather than user text or mandatory digest envelopes.

`--receipt FILE` publishes the fixed complete execution receipt separately; `--bundle FILE` saves a self-contained snapshot/plan/manifest archive. They are mutually exclusive. `run BUNDLE` accepts delivery options only, recomputes from bundled bytes/configuration and compares its full receipt. No original files, network, environment substitution, nested run, archive unpacking or source/plan overrides occur. Bundle contents are opt-in data and can contain source or explicit metadata supplied by the caller.

`inspect` requires `--css`; it establishes a complete match count and defaults to three samples (range 1–10). The default result is unchanged: bounded tags, at most eight exact attribute names and 160 Unicode scalar values of whitespace-collapsed literal text, with separate completeness flags. `--identifiers` instead returns each sampled tag's exact `id` (up to 128 UTF-8 bytes), up to eight distinct class tokens in lexical order (up to 64 bytes each), and 160 Unicode scalar values of whitespace-collapsed text with HTML block and break boundaries. Oversized identifiers are omitted and `identifiers_complete` becomes false; values are never shortened into misleading selector fragments. The mode reports only `id` and class values, not arbitrary attributes. Both modes retain a complete selector count and a 16 KiB encoded response cap; exhaustion fails rather than returning partial success. Identifier values are raw data, not escaped or proven-unique CSS selectors. Use an explicit scoped selector such as `main,article,table` and verify the resulting selector with `inspect` or `extract`. Neither mode infers browser visibility or boilerplate.

`outline` surveys repeated original-DOM HTML siblings without a selector by default. It considers tag groups and narrower complete class-signature groups; these can overlap. `--within CSS` requires exactly one scope element. The complete group count is established, while the largest four groups are shown by default (`--limit` range 1–16); `groups_complete` labels any omission. Every group has an exact direct-sibling count, two bounded text samples, and parent/item identifiers. A selector is emitted only when it matches exactly those nodes on this source snapshot; `null` means no safe hint was proved. Table-row groups also report literal parsed header text, header uniqueness, direct data-cell count ranges, and span presence. They do not supply a guessed field mapping; pair manually selected columns with `--expect-text` guards when a header order matters. The result carries the accepted-source SHA-256. Discovery omits head, script, style, template, pre, code, noscript and foreign-namespace subtrees. It has a 10 million-unit work bound, 1,024 signatures per parent and a 16 KiB encoded output cap; exhaustion fails without partial success. Neither candidate selectors nor table headings guarantee meaning or stability on future snapshots.

`--output FILE` publishes data instead of stdout; replacement requires `--overwrite`. Input/destination collisions and destination symlinks/special files fail. Source, plan and bundle paths validate the opened regular handle; intentional streams use stdin. Regular input symlinks remain supported. Files use bounded staging and atomic native create/replace. All requested artifacts are staged before any commit; receipt/bundle commits precede data delivery. Several files/stdout are not a transaction. A later failure may leave an execution sidecar or bytes already delivered to a pipe and returns nonzero. File-only or empty-raw output does not require an unused stdout descriptor. No untested power-loss durability is promised.

Exit classes: 0 success/help; 2 invalid configuration/schema/selector/regex/bundle structure; 3 unmet selection/guard/attribute/representation or replay expectation; 4 resource exhaustion; 5 real acquisition/UTF-8/delivery I/O; 6 internal invariants. Known malformed archive bytes are distinct from a failing underlying read/seek.
