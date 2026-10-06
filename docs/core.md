---
afad: "4.0"
version: "20.0.0"
domain: CORE
updated: "2026-10-05"
route:
  keywords: [core, immutable snapshots, compiled plans, prepared documents, projections, guards, bounded discovery, identities]
  questions: ["what is the maintained htmlcut-core surface?", "what does the core schema registry cover?", "how should a Rust caller embed htmlcut-core?"]
---

# Core API

`SourceSnapshot` accepts immutable UTF-8 source plus explicit metadata and preserves full accepted bytes, including BOM/CRLF/NUL. `PreparedDocument` binds a validated preparation policy and lazily prepares one original DOM, caching success or failure. `CompiledPlan` validates current queries once and retains their bounded normalized JSON; each execution receives fresh shared work/cell/byte allowances. Parser types are private.

```rust
use htmlcut_core::{CompiledPlan, ExtractionPlan, Reading, PreparationLimits,
    PreparedDocument, SnapshotMetadata, SourceSnapshot};
let document = PreparedDocument::new(
    SourceSnapshot::new("<p id='amount'>EUR 180</p><a href='next'>Link</a>", SnapshotMetadata::default())?,
    PreparationLimits::default())?;
let amount = CompiledPlan::compile(&ExtractionPlan::css("#amount")?)?;
let mut link = ExtractionPlan::css("a")?;
link.read = Some(Reading::Attribute("href".into()));
let link = CompiledPlan::compile(&link)?;
assert_eq!(document.execute(&amount)?.data().as_values().unwrap(), ["EUR 180"]);
assert_eq!(document.execute(&link)?.data().as_values().unwrap(), ["next"]);
# Ok::<(), htmlcut_core::ExtractionError>(())
```

`ExtractionResult::data()` and `payload()` borrow immutable values/canonical JSON without framing LF. `into_data()` consumes the result and drops cached encoding/evidence. Empty Values and Records both encode as `[]`; `data_kind()` and receipts retain their distinction. Fields serialize in lexical name order; rows and many-valued strings retain original order. Results capture actual source/query/metadata/preparation and complete counts. Receipt methods accept no replacement document or query.

## Records and expectations

Root `Match` permits one/all/nth. `FieldMatch` also permits optional. `RecordField::css` constructs a required text field; fields are a map keyed by validated ASCII names of at most 64 bytes. There are 1–64 fields, no nested records or source-slicing fields. Required means exactly one node, optional means null on zero/string on one/failure on more, all means an array with default minimum one, and nth means one positive one-based position. A matched node lacking a requested attribute fails even under optional. Empty attributes/text are values. All/min=0 deliberately permits empty arrays.

A record forest contains its selected anchor and exactly `following_siblings` additional element-sibling subtrees (default 0, maximum 63). Text/comments do not count. Missing siblings fail; absorbing any root candidate, even an unselected nth candidate, fails. Nested candidates already within the anchor retain their supported behavior. `:scope` denotes the original anchor. Payload is restricted to the forest while selector predicates can inspect original context under the shared budget. Rows are never cloned/reparsed.

Expectations use document or selected scope, count bounds (default 1/1), and at most one whole-value `equals` or bounded Rust regex-search `pattern`. Inline flags/anchors control regex matching. Count-only expectations reject a supplied reading and do not project values; use CSS `[attr]` for presence. Predicate readings permit text/literal/attr and default to text. They inspect original content before exclusions. Document expectations run over explicit empty root selections; selected scope is vacuous over no roots. Every matched predicate node is checked after complete count validation. Any late failure rejects the whole operation.

## Readings

`Reading::Text` is static structural text. It collapses Unicode White_Space outside original HTML pre ancestry, retains non-ASCII and zero-width characters, and inserts boundaries at HTML blocks/breaks including details, summary and address. Parsed pre characters/edges/line endings are retained. HTML script/style/template and SVG script/style payloads at/below selection are omitted, including an inert selected root. A directly selected non-inert template descendant is readable. Hidden/CSS-hidden/noscript content remains; foreign names do not acquire HTML roles, while integration-point HTML does.

`Reading::Literal` concatenates parsed descendant text, including inert/hidden content, with no invented separators. `InnerHtml`/`OuterHtml` use filtered immutable DOM serialization. These representations can normalize source spelling and are not exact selected-byte replacements. Boundary cutting, boundary regex/literal constructors and source-range receipts are removed; caller byte/string code owns genuine byte-cutting jobs. Full accepted source remains available from the snapshot and in replay.

Attribute readings validate supported names and fail on absence. URL readings resolve supported attribute positions using explicit normalized absolute HTTP(S) base metadata without userinfo. Relative URLs require that metadata; HTML base elements and acquisition origin are not inferred. Raw/base inputs are capped at 8 KiB and URL processing at 32 KiB; value caps still apply. There are no transform arrays or arbitrary pipelines.

Markdown is a conventional CommonMark reading representation, not source/DOM/visual roundtrip or sanitization. It normalizes ASCII prose whitespace and preserves non-ASCII characters, protected pre characters, meaningful blocks, selected link/image metadata, source-derived ordinals and code. It does not guess hidden visibility or boilerplate. HTML script/style/template and supported SVG script/style payloads are excluded; foreign names do not invent HTML roles.

Headings use ATX syntax. Paragraphs, definitions, details/summary and address have distinct block boundaries. Lists use bullets; ordered labels are source-derived literal text (for example `- 8\. item`), preserving start/reversed/value resets and exclusion gaps. Renderers disregard later ordinary ordered markers, so they are not used to encode these ordinals.

All tables use nested lists: captions are preceding paragraphs, rows are outer items and cells are inner items, with header text bold. Empty/ragged/nested/block/code cells are retained once. No grid inference, span repetition, pipe-table cell dropping, or source span-layout roundtrip is promised. Use records/HTML/attributes for exact layout information. Selected fragments carry only required original list/pre/table context, not surrounding payload.

HTML emphasis/strong roles, including header cells, use fixed generated `<em>`/`<strong>` inline tags. Nested identical roles are idempotent; tags carry no source attributes. Consumers disabling inline HTML will not render emphasis. Non-pre code uses a backtick span longer than every payload run, with CommonMark-safe framing; parsed line endings become spaces and other characters remain literal. Selected fragments inherit original roles. Literal ampersands in prose, image alternatives and destinations are escaped to prevent a second character-reference decode.

Preformatted content uses a fence longer than conflicting literal backtick runs. An explicit `language-*` class on pre or its sole direct HTML code child supplies a language: 1–64 ASCII characters, starting alphanumeric, then alphanumeric or `_+.-`. Malformed recognized tokens or distinct conflicting labels fail with `invalid_representation`; duplicate identical labels are accepted. No language is inferred. Parsed text is emitted literally, including trailing LFs, followed by exactly one framing LF before the closing fence. CommonMark code events therefore contain payload plus that one structural LF. Link/image metadata inside code is annotated after the fence, keeping program text intact. Block-containing anchors keep block content and receive a following conventional destination link. Destinations containing LF/CR/NUL fail with `invalid_representation`; raw attributes/HTML retain their parsed values. Literal syntax is escaped where it could forge Markdown structure.

## Discovery, bounds and evidence

`inspect(select, samples)` counts completely and returns 1–10 bounded samples containing exact id/classes, attribute names and the shared structural reading. Independent completeness labels identify identifier/name/text/sample omissions. Oversized identifiers are omitted rather than shortened into misleading selector tokens. Observations bind exact accepted-source SHA-256 and have a 16 KiB encoded cap.

`survey(within, limit)` discovers repeated direct HTML siblings under a fresh 10 million-unit budget, with 1,024 signatures per parent and at most sixteen returned groups. Optional scope must select exactly one node. Groups can overlap. Every hint uses bounded CSS escaping and must match exact original members in order; null is a valid absence of proof. Table facts include complete structural-text headers (reading `text`), uniqueness, direct cell ranges and spans, without column meaning or grid reconstruction. Incomplete headers are not usable exact guards. For field discovery, extract one representative row's outer HTML.

Preparation defaults are 50 MiB source, 250,000 elements, 1,000,000 nodes, depth 2,048 and parser work 10,000,000. Execution defaults are work 1,000,000, candidates 100,000, selected 10,000, cells 100,000, one value 8 MiB and total leaf bytes 64 MiB. Each field slot costs one cell, including null/empty arrays; each all-valued string costs another. Complete candidates are counted for all/nth. Explicit maxima remain assumptions and are never silently clamped to resource limits.

Queries admit at most 256 KiB including normalized default expansion/escaping; patterns at most 8 KiB, syntax depth 64, fields 64 and expectations/aggregate exclusion selectors 32 each. Regex programs/DFA allowances share 8 MiB. Complete encoded data, including escaping/keys, is at most 64 MiB without delivery LF; receipts at most 4 MiB. Exhaustion never returns truncated successful data. Retained parser/selector/filtered-serializer forks enforce actual logical boundaries; source size alone cannot replace them. Logical limits do not establish exact RSS or OS isolation.

Ordinary source acceptance/preparation/compilation/execution hashes no identities and serializes no receipt. The core produces one bounded canonical data payload before success and retains it beside private typed values. This is bounded duplication. Query bytes are normalized once and retained; explicit identities/evidence request hashes lazily. `receipt()` and `receipt_payload()` consume the actual remaining execution work and cache success and failure. Source/query cache hits still receive identical logical charges, so a warm-created bundle must replay cold under the same policy. New executions have fresh allowances; results cannot be cloned to duplicate unspent evidence work.

Source/data digests are SHA-256 of accepted bytes/canonical payload, excluding delivery LF. Query and execution identities use eight-byte length framing with `htmlcut.plan/6` and `htmlcut.extraction/6`; preparation retains `htmlcut.prepared/3`. Execution binds actual source/query/metadata/preparation/semantics. Canonical ordering is independent of serde_json feature unification. Receipts contain data kind, four identities, complete root counts and field-name aggregate maps (candidates/projected strings/absences), with no source ranges or copied values/paths/raw metadata. Raw output remains a transport choice; its receipt identifies canonical JSON data. Integrity/execution facts do not authenticate source or establish delivery.

[CLI](cli.md) and [Schemas](schema.md) define delivery, named shapes and closed USTAR replay.
