---
afad: "4.0"
version: "15.0.0"
domain: ARCHITECTURE
updated: "2026-10-01"
route:
  keywords: [architecture, surfaces, htmlcut-cli, htmlcut-core, extraction contract, ownership boundary, discovery model]
  questions: ["what are the maintained HTMLCut surfaces?", "when should I reuse compiled plans and prepared documents?", "what does HTMLCut own versus downstream consumers?"]
---

# Architecture

HTMLCut has one supported Rust product API, `htmlcut-core`, and a binary CLI. The core accepts immutable UTF-8 snapshots, validates and compiles one extraction plan, lazily prepares a bounded DOM only when needed, and returns requested strings or one typed error family.

The CLI owns files, stdin, bounded GET acquisition, strict decoding, saved runs, output framing and atomic file publication. The core performs no filesystem, network, environment, clock, process or terminal I/O. Parser and DOM types remain private. Compiled plans and prepared documents are reusable; each execution gets fresh shared work counters. Preparation failures are cached too.

`dom_text` is literal parsed descendant text, including hidden and template/script/style content. `document_text` is explicit structural formatting with links, image alt, lists and tables, excluding only script/style/template payloads. Exclusions are explicit and immutable. Guards read the original DOM before exclusions or transforms. Default selection requires exactly one candidate.

Source slicing is a separate parse-free strategy with half-open UTF-8 byte ranges. DOM HTML serialization is not original-source preservation. URL resolution is explicit and uses caller/final-response base metadata; HTML base elements are ignored.

Compact results contain identities, complete counts and requested values. JSON has one framing LF; raw emits one exact value without an added LF. Semantic failures publish no values. Logical budgets are not OS isolation, and extracted data is not sanitized or prompt-injection protected.

See [Core](core.md), [CLI](cli.md), [Schemas](schema.md), and the [implementation authority](extraction-contract-spec.md).
