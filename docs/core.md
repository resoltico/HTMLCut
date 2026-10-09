# Core API

`SourceSnapshot` accepts immutable UTF-8 source plus explicit metadata and preserves full accepted bytes, including BOM/CRLF/NUL. `PreparedDocument` binds a validated preparation policy and lazily prepares one original DOM, caching success or failure. `CompiledPlan` validates current queries once and retains their bounded normalized JSON; each execution receives fresh shared work/cell/byte allowances. Parser types are private.

The [runnable core API example](../crates/htmlcut-core/README.md) is the canonical
package example and is checked by Rust doctests.

`ExtractionResult::data()` borrows owned typed values; `into_data()` consumes them.
`candidate_count()`, `selected_count()` and `field_counts()` expose complete counts.
Empty Values and Records both encode as `[]`; the typed enum retains the distinction.
Fields are lexical, and rows/many-valued strings retain original order. Results own
no source, query, DOM, residual work or encoded payload. A successful core execution
proves declared query semantics completed, not that JSON encoding or delivery succeeded.

## Records and expectations

Root `Match` permits one/all/nth. `FieldMatch` also permits optional. `RecordField::css` constructs a required text field; fields are a map keyed by validated ASCII names of at most 64 bytes. There are 1–64 fields, no nested records or source-slicing fields. Required means exactly one node, optional means null on zero/string on one/failure on more, all means an array with default minimum one, and nth means one positive one-based position. A matched node lacking a requested attribute fails even under optional. Empty attributes/text are values. All/min=0 deliberately permits empty arrays.

A record forest contains its selected anchor and exactly `following_siblings` additional element-sibling subtrees (default 0, maximum 63). Text/comments do not count. Missing siblings fail; absorbing any root candidate, even an unselected nth candidate, fails. Nested candidates already within the anchor retain their supported behavior. `:scope` denotes the original anchor. Payload is restricted to the forest while selector predicates can inspect original context under the shared budget. Rows are never cloned/reparsed.

Expectations use document or selected scope, count bounds (default 1/1), and at most one whole-value `equals` or bounded Rust regex-search `pattern`. Inline flags/anchors control regex matching. Count-only expectations reject a supplied reading and do not project values; use CSS `[attr]` for presence. Predicate readings permit text/literal/attr and default to text. They inspect original content before exclusions. Document expectations run over explicit empty root selections; selected scope is vacuous over no roots. Every matched predicate node is checked after complete count validation. Any late failure rejects the whole operation.

## Readings

`Reading::Text` is static structural text. It collapses Unicode White_Space outside original HTML pre ancestry, retains non-ASCII and zero-width characters, and inserts boundaries at HTML blocks/breaks including details, summary and address. Parsed pre characters/edges/line endings are retained. HTML script/style/template and SVG script/style payloads at/below selection are omitted, including an inert selected root. A directly selected non-inert template descendant is readable. Hidden/CSS-hidden/noscript content remains; foreign names do not acquire HTML roles, while integration-point HTML does.

`Reading::Literal` concatenates parsed descendant text, including inert/hidden content, with no invented separators. `InnerHtml`/`OuterHtml` use filtered immutable DOM serialization. These representations can normalize source spelling and are not exact selected-byte replacements. Caller byte/string code owns byte-cutting jobs. Full accepted source remains available from the snapshot.

Attribute readings validate supported names and fail on absence. URL readings resolve supported attribute positions using explicit normalized absolute HTTP(S) base metadata without userinfo. Relative URLs require that metadata; HTML base elements and acquisition origin are not inferred. Raw/base inputs are capped at 8 KiB and URL processing at 32 KiB; value caps still apply. There are no transform arrays or arbitrary pipelines.

Markdown is a conventional CommonMark reading representation, not source/DOM/visual roundtrip or sanitization. It normalizes ASCII prose whitespace and preserves non-ASCII characters, protected pre characters, meaningful blocks, selected link/image metadata, source-derived ordinals and code. It does not guess hidden visibility or boilerplate. HTML script/style/template and supported SVG script/style payloads are excluded; foreign names do not invent HTML roles.

Headings use ATX syntax. Paragraphs, definitions, details/summary and address have distinct block boundaries. Lists use bullets; ordered labels are source-derived literal text (for example `- 8\. item`), preserving start/reversed/value resets and exclusion gaps. Renderers disregard later ordinary ordered markers, so they are not used to encode these ordinals.

All tables use nested lists: captions are preceding paragraphs, rows are outer items and cells are inner items, with header text bold. Empty/ragged/nested/block/code cells are retained once. No grid inference, span repetition, pipe-table cell dropping, or source span-layout roundtrip is promised. Use records/HTML/attributes for exact layout information. Selected fragments carry only required original list/pre/table context, not surrounding payload.

HTML emphasis/strong roles, including header cells, use fixed generated `<em>`/`<strong>` inline tags. Nested identical roles are idempotent; tags carry no source attributes. Consumers disabling inline HTML will not render emphasis. Non-pre code uses a backtick span longer than every payload run, with CommonMark-safe framing; parsed line endings become spaces and other characters remain literal. Selected fragments inherit original roles. Literal ampersands in prose, image alternatives and destinations are escaped to prevent a second character-reference decode.

Preformatted content uses a fence longer than conflicting literal backtick runs. An explicit `language-*` class on pre or its sole direct HTML code child supplies a language: 1–64 ASCII characters, starting alphanumeric, then alphanumeric or `_+.-`. Malformed recognized tokens or distinct conflicting labels fail with `invalid_representation`; duplicate identical labels are accepted. No language is inferred. Parsed text is emitted literally, including trailing LFs, followed by exactly one framing LF before the closing fence. CommonMark code events therefore contain payload plus that one structural LF. Link/image metadata inside code is annotated after the fence, keeping program text intact. Block-containing anchors keep block content and receive a following conventional destination link. Destinations containing LF/CR/NUL fail with `invalid_representation`; raw attributes/HTML retain their parsed values. Literal syntax is escaped where it could forge Markdown structure.

## Discovery and limits

`inspect(select, samples)` counts completely and returns 1–10 bounded samples containing exact id/classes, attribute names and the shared structural reading. Independent completeness labels identify identifier/name/text/sample omissions. Oversized identifiers are omitted rather than shortened into misleading selector tokens. Typed observations contain counts, samples and completeness flags, without source digests. The CLI applies a 16 KiB encoded delivery cap; encoding failure publishes no observation.

`survey(within, limit)` discovers groups of at least three direct HTML siblings
sharing a tag, optionally constrained by a bounded class signature, under a fresh 10 million-unit budget,
with 1,024 signatures per parent and at most sixteen returned groups. Non-HTML,
head, script, style, template, pre, code and noscript branches are excluded.
Optional scope must select exactly one node. Groups can overlap; the survey does
not infer semantic fields or columns. An empty complete survey means no eligible
groups, not that the page has no records. For one or two records, use
`inspect(select, samples)` (CLI `--select CSS --samples 2`) to count and sample,
then extract one representative row's outer HTML for field discovery. Every hint
uses bounded CSS escaping and must match exact original members in order; null
is a valid absence of proof. Table facts include complete structural-text headers
(reading `text`), uniqueness, direct cell ranges and spans, without column meaning
or grid reconstruction. Incomplete headers are not usable exact guards.

Preparation defaults are 50 MiB source, 250,000 elements, 1,000,000 nodes, depth 2,048 and parser work 10,000,000. Execution defaults are work 1,000,000, candidates 100,000, selected 10,000, cells 100,000, one value 8 MiB and total leaf bytes 64 MiB. Each field slot costs one cell, including null/empty arrays; each all-valued string costs another. Complete candidates are counted for all/nth. Explicit maxima remain assumptions and are never silently clamped to resource limits.

Queries admit at most 256 KiB including normalized defaults and escaping; patterns at
most 8 KiB, syntax depth 64, parsed selector matching depth 64, fields 64 and expectations/aggregate exclusions 32 each.
Regex programs and DFA allowances share 8 MiB. CLI compact JSON admits at most
64 MiB before LF; encoding failure publishes no data. Core results are typed and
have no encoded-byte cap. Inspection caps samples at 10, previews at 160 characters,
identifiers/classes/attributes separately, and surveys at 16 groups/two samples each.

| Limit | Charge/admission point | Refusal |
|---|---|---|
| source bytes | UTF-8 acceptance and preparation, before parsing | oversized source rejected |
| query bytes | before JSON parsing and normalized encoding | oversized plan rejected |
| selector matching depth | parsed right-to-left compound/branch paths, before a selector is retained | compilation ResourceLimit above 64 units |
| elements/nodes | each parser allocation, including template fragments and fixed bookkeeping | before governed allocation |
| depth | parent ancestry plus child-subtree height on attachment/reparenting | before attachment |
| parse work | sink allocation, mutation, warnings, ancestry and subtree-edge accounting plus retained parser stop hooks | sticky refusal discards partial DOM |
| execution work | owned traversal/context/value byte batches and selector Element navigation/predicate callbacks | whole query refused, never a successful non-match |
| candidates | each matching pass, complete enumeration | before appending candidate above bound |
| selected | selection cardinality | before projection |
| cells | scalar strings or field containers plus many-valued strings | before append |
| value/aggregate bytes | bounded projection writer and per-value admission | no truncated data |
| JSON bytes | CLI serializer writes | before any publication |

These are logical counters, not OS CPU/RSS isolation or exact instruction counts.
The parser stops through retained hooks at token reprocessing/adoption loops and mutation boundaries;
current-token callbacks refuse mutations and the partial DOM is discarded. Source
bytes bound unfinished text/comment/raw tokens, which can exceed a DOM node's payload.
Unmodified upstream current-token stopping, pointer ownership and selector depth/refusal
controls still fail. Five narrow carriers remain; independent product boundaries are
implemented and verified as described in [dependencies](dependencies.md).
Exact fork-internal work semantics are no longer a public compatibility promise.

Matching-depth admission is separate from syntactic bracket/parenthesis depth and
DOM depth. Each selector starts at one compound unit; combinator sequences add
units right-to-left. A nested logical, relational or selector-argument branch adds
its path to the depth of its containing compound. Alternatives and successive
branches use the maximum path; relative selectors include their synthetic anchor.
The iterative parsed-structure check refuses depth above 64 with compilation
`selector_matching_depth` / configured_bound=64 before matching. It applies to
roots, fields, guards, exclusions, inspection and generated discovery hints.
Neither a work budget nor a larger thread stack substitutes for this admission.
