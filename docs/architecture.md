---
afad: "4.0"
version: "20.0.0"
domain: ARCHITECTURE
updated: "2026-10-05"
route:
  keywords: [architecture, surfaces, htmlcut-cli, htmlcut-core, extraction contract, ownership boundary, discovery model]
  questions: ["what are the maintained HTMLCut surfaces?", "when should I reuse compiled plans and prepared documents?", "what does HTMLCut own versus downstream consumers?"]
---

# Architecture

HTMLCut has one supported pure Rust API, `htmlcut-core`, and one native binary, `htmlcut`. The core accepts immutable caller-owned UTF-8 snapshots with explicit metadata/preparation, compiles a closed current query, prepares the original bounded DOM lazily, and executes with fresh shared counters. Parser types stay private. No filesystem/network/browser/environment/clock/process acquisition occurs in the core.

One direct typed compiler serves JSON, inline CLI and Rust construction. Query version 6 contains select/read/match, named fields, expectations and execution limits. Root and field cardinality differ explicitly; no retired contract translation or compatibility wrapper exists. Canonical normalized bytes are validated and retained once, independent of serde feature choices.

Field payload stays in the original selected row forest; selector predicates retain original context. Expectations inspect original content before exclusions. One shared text traversal supplies full values, predicates, bounded previews and complete table headers. Literal, Markdown and parsed HTML are separate requested readings. Exact selected-byte slicing is removed; full accepted source remains intact.

Execution constructs private typed values and one bounded canonical JSON payload. The result owns immutable execution provenance/counts and the actual residual work allowance. Source/query/configuration hashes and receipt encoding are lazy; receipt success/failure is cached. Hash-cache hits do not replenish logical work. Holding values plus encoded bytes is bounded duplication, not a measured memory reduction.

The CLI owns intentional file/stdin input, UTF-8 decoding, standard native help, coherent inspection, JSON/raw framing and native staged publication. It reuses core payload bytes. Requested evidence comes from the result alone. Artifacts stage before the first file commit; single-file atomicity does not make several files/stdout a transaction.

Replay retains a closed uncompressed three-member USTAR container containing manifest, normalized query and exact source. It validates bounded declared sizes, exact member/header/padding/footer rules and actual I/O failures without unpacking. Recomputed evidence must agree with the bundle. Transport family remains 1; embedded query/receipt/semantics are current version 6.

Maintained parser/selector/serialization and provenance forks protect retained pure-library properties. Logical counters do not provide OS isolation; hashes do not authenticate origin or business meaning. Acquisition, rendering, normalization of business values, joins and ranking belong to callers.

See [Core](core.md), [CLI](cli.md), [Schemas](schema.md) and [Extraction Contract](extraction-contract-spec.md).
