---
afad: "4.0"
version: "14.0.0"
domain: INTEROP
updated: "2026-09-24"
route:
  keywords: [interop, v2, htmlcut-v2, compile_plan, prepare_document, execute, explore, resolve_target_and_propose, CompiledPlan, PreparedDocument, HtmlInput, Plan, InteropResult, ExplorationResult, plain_text, extraction identity, discovery identity, HTMLCUT_EXTRACTION_SEMANTICS_VERSION, HTMLCUT_DISCOVERY_SEMANTICS_VERSION, dom_canonicalization]
  questions: ["how do I embed htmlcut extraction into a downstream project?", "what is the htmlcut interop v2 API?", "how do I prepare a document once and execute many plans?", "how do I explore elements and obtain selector proposals?", "how do I resolve a browser target without guessing?", "when should I use plain_text instead of rendered text?", "how do I identify a deterministic htmlcut extraction?"]
---

# HTMLCut Interop v2 Guide

**Purpose**: Embed HTMLCut extraction into a downstream Rust project using the maintained `htmlcut-v2` interop profile.
**Prerequisites**: Rust project with `htmlcut-core` as a Cargo dependency.

## Overview

The `htmlcut_core::interop::v2` module is the versioned downstream integration surface. It
exposes plan construction, plan preparation, and plan execution for downstream consumers. The
profile name is `htmlcut-v2`.

This is a library integration surface, not a CLI command and not an operation catalog entry.

The interop v2 JSON schemas are also exported through the general HTMLCut schema registry (`htmlcut schema`).

## Ownership Boundary

The downstream consumer owns:

- target definition
- fetch policy
- redirects, timeouts, headers, browser use, retries
- decoded HTML input
- comparison and persistence

HTMLCut owns:

- plan validation for `htmlcut-v2`
- the published selector, delimiter, output, diagnostic, and result vocabulary for `htmlcut-v2`
- extraction execution
- translation from the interop language into core extraction requests
- typed result and error documents
- stable JSON serialization
- deterministic digests
- extraction-semantics identity

## Public API

Use:

- `compile_plan(&Plan) -> Result<CompiledPlan, Box<InteropError>>`
- `prepare_document(HtmlInput, PreparationLimits) -> Result<PreparedDocument, Box<PreparationError>>`
- `execute(&PreparedDocument, &CompiledPlan) -> Result<InteropResult, Box<InteropError>>`
- `explore(&PreparedDocument, &ExplorationOptions) -> Result<ExplorationResult, Box<ExplorationError>>`
- `resolve_target_and_propose(&PreparedDocument, &ElementTargetHint, &TargetResolutionOptions) -> Result<TargetResolutionResult, Box<ExplorationError>>`

Main types:

- `HtmlInput`
- `Plan`
- `CompiledPlan`
- `PreparedDocument`
- `InteropResult`
- `InteropError`
- `CssSelectorText`
- `DelimiterBoundaryText`
- `Output`
- `DomCanonicalization`
- `InteropDiagnostic`
- `ByteRange`
- `ExplorationOptions`
- `ElementTargetHint`

Validator discovery:

- `htmlcut schema --name htmlcut.plan --schema-version 9 --output json`
- `htmlcut schema --name htmlcut.result --schema-version 10 --output json`
- `htmlcut schema --name htmlcut.error --schema-version 4 --output json`
- `htmlcut schema --name htmlcut.exploration_result --schema-version 1 --output json`
- `htmlcut schema --name htmlcut.target_resolution_result --schema-version 1 --output json`
- `htmlcut schema --name htmlcut.preparation_error --schema-version 1 --output json`

Rust callers can also use `htmlcut_core::schema_catalog()` and `schema_descriptor(...)`.

Deterministic JSON and digest helpers:

- `Plan::stable_json()` / `Plan::digest_sha256()`
- `HtmlInput::extraction_identity_sha256(&Plan)` for the complete-input extraction identity
- `HTMLCUT_EXTRACTION_SEMANTICS_VERSION` for the independently versioned extraction semantics
- `InteropResult::stable_json()` / `InteropResult::digest_sha256()` / `InteropResult::with_computed_digest()`
- `InteropError::stable_json()` / `InteropError::digest_sha256()` / `InteropError::with_computed_digest()`
- `stable_json_v2(...)` for the frozen canonical serializer itself

## Minimal Embedding Example

```rust
use htmlcut_core::interop::v2::{
    CssSelectorText, HtmlInput, HttpUrl, Output, Plan, PlanStrategy, Rendering, Selection,
    TextWhitespace, compile_plan, execute, prepare_document, PreparationLimits,
};

let source = HtmlInput::new(
    "example_news",
    "<article><h1>Headline</h1></article>",
)
.unwrap()
.with_input_base_url(HttpUrl::parse("https://example.com/news/").unwrap());

let plan = Plan::new(
    PlanStrategy::css_selector(CssSelectorText::new("article h1").unwrap()),
    Selection::single(),
    Output::plain_text(),
    Rendering::new(TextWhitespace::Normalize, false),
);

let compiled = compile_plan(&plan).unwrap();
let document = prepare_document(source, PreparationLimits::default()).unwrap();
let result = execute(&document, &compiled).unwrap();

assert_eq!(result.output.kind().as_str(), "plain_text");
assert_eq!(result.selected_matches[0].output_value, "Headline");
```

`CompiledPlan` and `PreparedDocument` are opaque and non-serializable. Compile a source-independent plan once, prepare each exact HTML snapshot once, then execute the compiled plan against any prepared document. Every successful result carries `source.prepared_source_digest_sha256`, binding its evidence to the exact prepared snapshot and preparation limits. HTMLCut deliberately provides no one-shot compatibility wrapper that hides reparse or recompilation work.

## Exploration and target identity

`explore` returns one bounded document-order page. Pass its `next_cursor` back in `ExplorationOptions.cursor` with the same prepared document and page options to continue. The cursor is a deterministic continuation hint, not an authorization token. A changed prepared source or option set is rejected; a terminal page has no `next_cursor`. Work exhaustion may still be reported on the terminal page when a descriptor or proposal could not be completed within the budget. The CLI accepts the serialized object through `inspect elements --cursor <JSON>`; save live HTML to a static file before paginating because a URL is fetched anew on each CLI invocation.

For an `ElementTargetHint`, concatenate descendant DOM text nodes in document order, normalize CRLF and lone CR to LF across text-node boundaries, and leave all other whitespace unchanged. Compute lowercase hex SHA-256 over the UTF-8 bytes of `"htmlcut.dom_text.v1\0"` (the `\0` is one NUL byte) followed by the normalized text's UTF-8 bytes. `normalized_dom_text_digest` is the Rust helper for this published algorithm. The digest of `One` is `e163d4661874af8285350875496da6d42afb63bbe2c9e6f7d390c6c632aadfc6`. Fingerprinted descendant text is limited to 1 MiB. Browser-side evidence must represent the same static HTML snapshot; a DOM changed by JavaScript may not resolve, by design.

## Published Language Boundary

`htmlcut-v2` is not a JSON alias for `htmlcut-core` request/result types.

It owns its own published language:

- selector text: `CssSelectorText`
- delimiter boundary text: `DelimiterBoundaryText`
- output contract: `Output`
- detached-clone canonicalization: `DomCanonicalization`
- diagnostics: `InteropDiagnostic`
- byte ranges: `ByteRange`

That boundary lets `htmlcut-core` evolve its internal request/result vocabulary without forcing
downstream consumers to deserialize core-only types or internal structured payloads directly.

Choose `Output::plain_text()` when the selected DOM element's literal descendant text is the value
being measured. Choose `Output::text()` when HTML-aware document structure is part of the desired
output, such as a Markdown-like heading marker, list item, or resolved link destination.

## Supported v2 Capability

Strategy kinds:

- `css_selector`
- `delimiter_pair`

Selection modes:

- `single`
- `first`
- `nth`
- `all`

Output kinds:

- CSS selector: `text`, `plain_text`, `inner_html`, `outer_html`, `attribute`, `structured`
- delimiter pair: `text`, `inner_html`, `outer_html`, `selected_html`, `attribute`, `structured`

When `Output::attribute { name }` is selected and the chosen candidate does not expose that
attribute, the error contract uses `error_code = "missing_attribute"`.

Every successful `htmlcut.result` carries one top-level `output` object that records the requested
published output contract. Every `SelectedMatch` then carries:

- `output_value` for the exact requested output payload
- `text_output`, HTML-aware semantic rendered text rather than whole-document reader cleanup
- `plain_text_output` for CSS selections: direct DOM descendant text without heading, list, link,
  or other structural decoration
- `comparison_text_output` when a text-semantic CSS plan canonicalizes a detached selected clone
- `comparison_plain_text_output` when a plain-text CSS plan canonicalizes a detached selected clone
- `selected_html_output` when the strategy is `delimiter_pair`
- `inner_html_output`, which for `delimiter_pair` means the HTML between the two matched
  boundaries
- `outer_html_output`
- typed `metadata`

Core execution errors preserve their primary HTMLCut diagnostic without exposing an open-ended
error map. `InteropError.detail` is a required tagged union. Its `core_execution` variant carries
the exact uppercase `core_diagnostic_code` and `candidate_count`, including `NO_MATCH` with count
`0`. Plan validation uses `contract_violation`; projection failures use a closed
`adapter_invariant` failure code; an absent requested attribute uses `missing_requested_attribute`.
Consumers must match the detail kind rather than inspect arbitrary JSON keys.

Invalid CSS selectors use the exact human-readable message `CSS selector is invalid.`. HTMLCut
does not expose the selector text, upstream parser prose, debug formatting, or parser-internal
types in that message. `InteropError.detail` is `invalid_selector` and carries the same closed
selector-parse object as `diagnostics[].details.selector_parse`:

```json
{
  "line": 1,
  "column_utf16": 1,
  "parse_error_class": "invalid_attribute_selector"
}
```

`line` is one-based. `column_utf16` is one-based and counts UTF-16 code units. The
`parse_error_class` vocabulary is owned and exhaustively mapped by HTMLCut; consumers must treat
unknown values as invalid rather than attempting to interpret upstream parser output. Runtime
validation rejects a non-canonical invalid-selector message, missing or malformed diagnostic
evidence, duplicate diagnostics, and any disagreement between the diagnostic and tagged error
detail.
The current closed vocabulary is:

- `unexpected_token`, `end_of_input`, `invalid_at_rule`, `invalid_at_rule_body`, and `invalid_qualified_rule`
- `pseudo_element_expected_colon`, `pseudo_element_expected_ident`, and `no_ident_for_pseudo`
- `invalid_attribute_selector`, `unexpected_token_in_attribute_selector`, `expected_bar_in_attribute_selector`, `invalid_attribute_value`, and `invalid_qualified_name_in_attribute_selector`
- `empty_selector`, `dangling_combinator`, `non_compound_selector`, and `invalid_state`
- `non_pseudo_element_after_slotted`, `invalid_pseudo_element_after_slotted`, and `invalid_pseudo_element_inside_where`
- `unsupported_pseudo_class_or_element`, `unexpected_ident`, `expected_namespace`, `explicit_namespace_unexpected_token`, and `class_needs_ident`

Every `InteropError.message` and `InteropDiagnostic.message` is limited to 1024 UTF-8 bytes.
The JSON Schema advertises a 1024-character maximum where standard JSON Schema can express it;
runtime validation is authoritative for the stricter byte limit and for the cross-carrier selector
parse invariant. `with_computed_digest`, `digest_sha256`, and `stable_json` enforce those rules
before returning a public document.

If construction rejects an interop error, HTMLCut returns a valid internal-error fallback. Its
`detail.kind` is `finalization_rejected` with a closed rejection code; it never copies invalid
diagnostic payloads, debug formatting, or arbitrary structured data into the fallback.

## DOM Canonicalization

Construct a `DomCanonicalization` policy, then pass it to a CSS-selector plan with
`Plan::with_dom_canonicalization`:

```rust
use htmlcut_core::interop::v2::{AttributeName, DomCanonicalization};

let canonicalization = DomCanonicalization::new(
    [AttributeName::new("data-nonce").unwrap()], true,
);
```

The policy has two effective fields:

- `ignore_attributes`: no attributes are ignored unless the plan names them
- `strip_whitespace_nodes`

DOM canonicalization is valid only for CSS `Output::text`, `Output::plain_text`, and
`Output::structured` plans.
Execution is deliberately ordered: HTMLCut selects candidates on the original parsed DOM, retains
the original `text_output`, HTML fields, candidate count, path, diagnostics, and metadata, then
clones the selected subtree. Only that detached clone is canonicalized and rendered into
`SelectedMatch.comparison_text_output`.

For `Output::text`, `output_value` is the rendered comparison text when canonicalization is
configured; the original rendered text remains in `text_output` as evidence. For
`Output::plain_text`, `output_value` is the plain comparison text and the original remains in
`plain_text_output`. `Output::structured` preserves the raw structured selected payload and
exposes either clone rendering through the selected match's `comparison_text_output` and
`comparison_plain_text_output` fields. `Output::inner_html`, `Output::outer_html`, and direct
attributes retain original evidence and reject `dom_canonicalization`, rather than accepting an
inert policy. `Output::selected_html` belongs to `delimiter_pair`, for which canonicalization is
also rejected.

`Output::attribute { name }` reads the original CSS match-metadata attribute map. A plan that both
ignores and measures the same attribute, including ASCII case variants such as `href` and `HREF`,
retains its specific `plan_invalid` error; any other direct-attribute canonicalization is likewise
rejected as inert. The result validator rejects `comparison_text_output` for direct-attribute and
raw-HTML output kinds; requires text `output_value` to equal `comparison_text_output` when present
(otherwise `text_output`); and rejects `comparisonTextOutput` inside structured raw evidence.

## Determinism Rules

The interop surface is versioned around:

- `stable_json_v2`
- SHA-256 digests over canonical JSON
- fixture-backed acceptance coverage
- `HTMLCUT_EXTRACTION_SEMANTICS_VERSION`, currently `4`

`HtmlInput::extraction_identity_sha256(&Plan)` is the canonical identity for one extraction. It
binds every `HtmlInput` field (including the decoded HTML bytes, logical label, and optional input
base URL), the complete `Plan` including a plan that yields a diagnostic, and
`HTMLCUT_EXTRACTION_SEMANTICS_VERSION`. HTMLCut owns the identity algorithm so downstream
consumers do not reimplement or omit part of its input.

`dom_canonicalization` is part of the serialized `Plan`, so it participates directly in
`plan_digest_sha256` and extraction identity. The counter is `4`: a fixed CSS plan can now choose
a plain DOM-descendant-text projection distinct from HTML-aware rendered text. A fixed plan remains
deterministic within its declared output kind.

Increment `HTMLCUT_EXTRACTION_SEMANTICS_VERSION` only when fixed complete input and a plan that
passes preflight could produce a different extraction projection. Invalid-plan diagnostic-envelope
changes are instead versioned by `htmlcut.error` and `htmlcut.result`; they do not change the
typed measurement semantics counter. Do not derive the counter from the HTMLCut crate version,
the core specification version, or dependency versions. The counter is intentionally an identity
input, not a field that every `InteropResult` or `InteropError` must carry.

The acceptance corpus lives under:

`crates/htmlcut-core/tests/fixtures/htmlcut-v2/`

The acceptance runner that freezes the profile and proves repeated executions are byte-identical
to the golden documents lives in:

`crates/htmlcut-core/src/tests/interop_v2/acceptance.rs`

## Important v2 Limits

These are intentionally not part of `htmlcut-v2`:

- XPath
- regex window extraction
- text-anchor extraction
- HTMLCut-owned fetch orchestration
- browser automation inside HTMLCut

`htmlcut.result` also intentionally excludes runtime timing fields so result JSON and result digests stay deterministic across runs.

`HtmlInput` is intentionally not part of the JSON schema registry because it is a Rust-only in-process source handoff type, not a persisted or exchanged JSON document.

`PreparationError` is a separate public document because document preparation has no plan and therefore cannot truthfully carry a plan digest. Exploration and target-resolution failures likewise use their own closed error document rather than inventing extraction error identity.

## Versioning Rule

`htmlcut_core::interop::v2` is versioned through its exported schema families.

When `Plan`, `InteropResult`, or `InteropError` changes shape, update the corresponding integer
schema version, refresh the acceptance fixtures, and ship the docs change in the same release.
DOM canonicalization, explicit execution budgets, and CSS-only `plain_text` are represented by `htmlcut.plan@9`; rendered and plain comparison output are represented by `htmlcut.result@10`. Bounded diagnostic messages and the validated selector-parse error envelope are represented by `htmlcut.error@4`. Prepared-document, exploration, target-resolution, and proposal contracts are separate v2 schema families.
The maintained policy details live in [versioning-policy.md](versioning-policy.md).
