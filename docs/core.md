---
afad: "4.0"
version: "17.0.0"
domain: CORE
updated: "2026-10-03"
route:
  keywords: [core, immutable snapshots, compiled plans, prepared documents, projections, guards, bounded discovery, identities]
  questions: ["what is the maintained htmlcut-core surface?", "what does the core schema registry cover?", "how should a Rust caller embed htmlcut-core?"]
---

# Core API

Supply exact UTF-8 bytes and explicit metadata. `SourceSnapshot` preserves source spelling/BOM/CRLF; the parsed DOM follows HTML rules. Compile plans once and reuse them. `PreparedDocument` parses lazily at most once, including cached preparation failures. Every execution receives fresh shared counters. Parser/DOM types are private.

```rust
use htmlcut_core::{CompiledPlan, ExtractionPlan, PreparationLimits, PreparedDocument,
    Projection, ValueProjection, SnapshotMetadata, SourceSnapshot};
let document = PreparedDocument::new(
    SourceSnapshot::new("<p id='amount'>EUR 180</p><a href='next'>Link</a>", SnapshotMetadata::default())?,
    PreparationLimits::default())?;
let amount = CompiledPlan::compile(&ExtractionPlan::css("#amount")?)?;
let mut link = ExtractionPlan::css("a")?;
link.projection = Projection::Value(ValueProjection::Attribute { name: "href".into() });
let link = CompiledPlan::compile(&link)?;
assert_eq!(document.execute(&amount)?.data.as_values().unwrap(), ["EUR 180"]);
assert_eq!(document.execute(&link)?.data.as_values().unwrap(), ["next"]);
# Ok::<(), htmlcut_core::ExtractionError>(())
```

`ExtractionResult` separates bare `ExtractionData` from `ExecutionReceipt`. Values are strings; records contain named `FieldValue` strings, explicit absence/null, or string arrays. Data serialization is only the requested array. Empty values/records both serialize as `[]`; the typed enum and receipt data kind preserve their interpretation. Record maps serialize in key order, while rows and all-valued fields retain original order.

## Records and assumptions

A CSS records projection has 1–64 ordered `RecordField` declarations with unique ASCII names (`[A-Za-z_][A-Za-z0-9_]{0,63}`). Each has a selector, scalar projection, cardinality, exclusions and transforms. No nested records, source fields, numeric/date mapping, sorting, joins or expressions occur.

Root selection is required single by default, explicit all with min/max, or positive one-based nth. Fields additionally allow optional: zero nodes produces `null`, one produces a string, more than one fails. Missing attributes on matched nodes always fail; present empty strings remain valid. All-valued fields return arrays, with zero allowed only by explicit minimum zero. Nth is positional, not semantic identity.

Field candidates are the row root or its descendants; `:scope` addresses the row. Original ancestor/sibling predicates may observe outside context under the shared work budget, but outside-row payload cannot be returned. There is no clone/reparse or nearest-card inference. Nested row matches must be made precise explicitly. Guards are conjunctive original-DOM reads before exclusions/transforms. Document guards also run for explicit empty selections; selected-row guards over zero rows are vacuous by that declared choice. A late row/field failure rejects the complete operation, rather than returning partial records.

Root exclusions/transforms apply to flat projections; records use field-owned exclusions/transforms. Normalize-whitespace is an explicit literal-text transform protecting original HTML pre ancestry. Markdown owns normalization already. URL resolution is explicit for supported attributes and Markdown destinations, using normalized absolute HTTP(S) base metadata without userinfo. HTML base elements and acquisition origin are not inferred. Raw/base inputs are capped at 8 KiB and URL processing at 32 KiB; output caps still apply.

## Representations

`dom_text` concatenates parsed descendant text literally, including hidden/script/style/template content. `inner_html`/`outer_html` serialize filtered immutable parsed DOM, not original source bytes. Source slicing is a distinct parse-free strategy of literal or bounded-regex nonoverlapping boundary pairs, with exact accepted-byte half-open ranges and explicit boundary inclusion.

```rust
use htmlcut_core::{Boundary, CompiledPlan, ExtractionPlan, PreparationLimits,
    PreparedDocument, SnapshotMetadata, SourceSnapshot};
let document = PreparedDocument::new(
    SourceSnapshot::new("STARTéEND", SnapshotMetadata::default())?, PreparationLimits::default())?;
let plan = ExtractionPlan::slice(Boundary::Literal { value: "START".into() },
    Boundary::Literal { value: "END".into() })?;
let result = document.execute(&CompiledPlan::compile(&plan)?)?;
assert_eq!(result.data.as_values().unwrap(), ["é"]);
assert_eq!(result.receipt.ranges.as_ref().unwrap()[0].start, 5);
# Ok::<(), htmlcut_core::ExtractionError>(())
```

Markdown is a conventional CommonMark reading representation, not source/DOM/visual roundtrip or sanitization. It normalizes ASCII prose whitespace and preserves non-ASCII characters, meaningful blocks, selected link/image metadata, source-derived ordinals and code. It does not guess hidden visibility or boilerplate. HTML script/style/template and supported SVG script/style payloads are excluded; foreign names do not invent HTML roles.

Headings use ATX syntax. Paragraphs, definitions, details/summary and address have distinct block boundaries. Lists use bullets; ordered labels are source-derived literal text (for example `- 8\. item`), preserving start/reversed/value resets and exclusion gaps. Renderers disregard later ordinary ordered markers, so they are not used to encode these ordinals.

All tables use nested lists: captions are preceding paragraphs, rows are outer items and cells are inner items, with header text bold. Empty/ragged/nested/block/code cells are retained once. No grid inference, span repetition, pipe-table cell dropping, or source span-layout roundtrip is promised. Use records/HTML/attributes for exact layout information. Selected fragments carry only required original list/pre/table context, not surrounding payload.

Preformatted content uses a fence longer than conflicting literal backtick runs. Parsed text is emitted literally, including trailing LFs, followed by exactly one framing LF before the closing fence. CommonMark code events therefore contain payload plus that one structural LF. Link/image metadata inside code is annotated after the fence, keeping program text intact. Block-containing anchors keep block content and receive a following conventional destination link. Destinations containing LF/CR/NUL fail with `invalid_representation`; raw attributes/HTML/source retain those values. Literal syntax is escaped where it could forge Markdown structure.

## Bounds and identities

Preparation defaults: 50 MiB source, 250,000 elements, 1,000,000 nodes, depth 2,048, and 10,000,000 parser work units. Execution defaults: shared work 1,000,000, 100,000 candidates per pass, 10,000 selected roots, 100,000 cells, 8 MiB per value and 64 MiB aggregate leaf payload. A field slot costs one cell; all-valued strings each cost another; a flat string costs one. Empty/null fields cannot evade it.

Plans are capped at 256 KiB, patterns at 8 KiB, syntax depth at 64, fields at 64 and aggregate guards/exclusion selectors at 32 each. All configured regex program/DFA allowances share 8 MiB, divided equally across the known regex count. Complete data JSON, including escaping/keys, is capped at 64 MiB excluding its delivery LF; receipts at 4 MiB. Exhaustion is failure, never truncated successful data. These are logical bounds, not exact allocator/RSS or OS isolation guarantees.

Matching scratch is scoped to one immutable document/pass. Inner traversal/predicates, guards, field passes, exclusions, formatting and serialization share execution accounting; cached outcomes do not grant free work. A mismatched document/scope is an invariant error. All/nth counts are complete, even when the selected values are few.

Source digest is SHA-256 of accepted bytes. Data digest is SHA-256 of compact canonical payload without framing LF. Plan and execution identities use domain-separated eight-byte length framing, current domains `htmlcut.plan/3` and `htmlcut.extraction/3`. Execution binds source/plan identities, normalized metadata, actual preparation policy and semantics. Preparation uses `htmlcut.prepared/3`. Canonical object ordering survives downstream serde feature unification. Digests prove identity/integrity, not authenticity or business correctness.

Receipts have current wire/semantics, data kind, four digests, complete root counts, declaration-ordered numeric field aggregates and optional source ranges. They contain no values, source, paths or raw metadata and prove execution rather than successful later delivery. [CLI](cli.md) and [Schemas](schema.md) define adapter framing, inspection and self-contained replay.
