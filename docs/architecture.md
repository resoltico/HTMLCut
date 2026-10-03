---
afad: "4.0"
version: "17.0.0"
domain: ARCHITECTURE
updated: "2026-10-03"
route:
  keywords: [architecture, surfaces, htmlcut-cli, htmlcut-core, extraction contract, ownership boundary, discovery model]
  questions: ["what are the maintained HTMLCut surfaces?", "when should I reuse compiled plans and prepared documents?", "what does HTMLCut own versus downstream consumers?"]
---

# Architecture

HTMLCut has one supported pure Rust API, `htmlcut-core`, and the native binary `htmlcut`. The core accepts exact immutable UTF-8 snapshots plus explicit metadata/preparation policy, validates and compiles closed plans, prepares the original bounded DOM lazily, and executes with fresh shared budgets. Parser/DOM types remain private.

The CLI owns regular files, intentional stdin, strict UTF-8 acceptance, one closed replay container, JSON/raw framing and staged atomic single-file publication. It performs no HTTP acquisition or browser/charset workflow. The core performs no filesystem/network/environment/clock/process/terminal acquisition; bounded caller-owned serialization is explicit.

One plan selects roots once. A scalar projection returns strings; a records projection evaluates named scalar fields within each original row and returns typed records. Field payload candidates remain inside the row while ancestor/sibling predicates retain original-DOM context under the shared budget. Guards read original content before field exclusions/transforms. Required single, optional node presence, all and nth have explicit shapes and failure rules. No field HTML reparsing or nested record/expression language occurs.

`ExtractionResult` separates `ExtractionData` from `ExecutionReceipt`. Bare data serialization contains only the requested array; receipts bind source/plan/extraction/data identities and counts without copied values or paths. Prepared documents cache one parsing result or failure; compiled plans contain no spent execution counters.

Markdown reading, literal DOM text, parsed HTML serialization and exact source slices are distinct contracts. Markdown preserves source ordinals as literal bullet labels and table cells as nested lists, avoiding custom frames and renderer-dependent renumbering or discarded pipe-table cells. Source slices are parse-free accepted-byte substrings.

Replay bundles contain source bytes, normalized plan, explicit metadata/preparation and recorded receipt. The CLI reads a bounded three-member USTAR subset without unpacking, re-executes, and rejects integrity/expectation disagreements. Hashes do not authenticate origin; logical resource counters do not provide OS isolation. Data/destinations are untrusted and are not sanitized.

See [Core](core.md), [CLI](cli.md), [Schemas](schema.md), and [Extraction Contract](extraction-contract-spec.md).
