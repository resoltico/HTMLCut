---
afad: "4.0"
version: "12.0.0"
domain: INTEROP
updated: "2026-07-16"
route:
  keywords: [interop, v1, htmlcut-v1, execute_plan, prepare_plan, execute_validated_plan, ValidatedPlan, HtmlInput, Plan, InteropResult, plain_text, extraction identity, HTMLCUT_EXTRACTION_SEMANTICS_VERSION, dom_canonicalization, comparison_text_output, interop profile]
  questions: ["how do I embed htmlcut extraction into a downstream project?", "what is the htmlcut interop v1 API?", "when should I use plain_text instead of rendered text?", "what schemas does htmlcut interop v1 export?", "how do I identify a deterministic htmlcut extraction?", "where is the candidate count on an htmlcut interop error?", "how does HTMLCut canonicalize a selected DOM subtree without changing raw evidence?"]
---

# HTMLCut Interop v1 Guide

**Purpose**: Embed HTMLCut extraction into a downstream Rust project using the maintained `htmlcut-v1` interop profile.
**Prerequisites**: Rust project with `htmlcut-core` as a Cargo dependency.

## Overview

The `htmlcut_core::interop::v1` module is the versioned downstream integration surface. It
exposes plan construction, plan preparation, and plan execution for downstream consumers. The
profile name is `htmlcut-v1`.

This is a library integration surface, not a CLI command and not an operation catalog entry.

The interop v1 JSON schemas are also exported through the general HTMLCut schema registry (`htmlcut schema`).

## Ownership Boundary

The downstream consumer owns:

- target definition
- fetch policy
- redirects, timeouts, headers, browser use, retries
- decoded HTML input
- comparison and persistence

HTMLCut owns:

- plan validation for `htmlcut-v1`
- the published selector, delimiter, output, diagnostic, and result vocabulary for `htmlcut-v1`
- extraction execution
- translation from the interop language into core extraction requests
- typed result and error documents
- stable JSON serialization
- deterministic digests
- extraction-semantics identity

## Public API

Use:

- `prepare_plan(&Plan) -> Result<ValidatedPlan, Box<InteropError>>`
- `execute_validated_plan(&HtmlInput, &ValidatedPlan) -> Result<InteropResult, Box<InteropError>>`
- `execute_plan(&HtmlInput, &Plan) -> Result<InteropResult, Box<InteropError>>`

Main types:

- `HtmlInput`
- `Plan`
- `ValidatedPlan`
- `InteropResult`
- `InteropError`
- `CssSelectorText`
- `DelimiterBoundaryText`
- `Output`
- `DomCanonicalization`
- `InteropDiagnostic`
- `ByteRange`

Validator discovery:

- `htmlcut schema --name htmlcut.plan --schema-version 8 --output json`
- `htmlcut schema --name htmlcut.result --schema-version 9 --output json`
- `htmlcut schema --name htmlcut.error --schema-version 3 --output json`

Rust callers can also use `htmlcut_core::schema_catalog()` and `schema_descriptor(...)`.

Deterministic JSON and digest helpers:

- `Plan::stable_json()` / `Plan::digest_sha256()`
- `HtmlInput::extraction_identity_sha256(&Plan)` for the complete-input extraction identity
- `HTMLCUT_EXTRACTION_SEMANTICS_VERSION` for the independently versioned extraction semantics
- `InteropResult::stable_json()` / `InteropResult::digest_sha256()` / `InteropResult::with_computed_digest()`
- `InteropError::stable_json()` / `InteropError::digest_sha256()` / `InteropError::with_computed_digest()`
- `stable_json_v1(...)` for the frozen canonical serializer itself

## Minimal Embedding Example

```rust
use htmlcut_core::interop::v1::{
    CssSelectorText, HtmlInput, HttpUrl, Output, Plan, PlanStrategy, Rendering, Selection,
    TextWhitespace, execute_validated_plan, prepare_plan,
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

let prepared = prepare_plan(&plan).unwrap();
let result = execute_validated_plan(&source, &prepared).unwrap();

assert_eq!(result.output.kind().as_str(), "plain_text");
assert_eq!(result.selected_matches[0].output_value, "Headline");
```

For one-shot callers, `execute_plan(...)` still performs validation internally. Use
`prepare_plan(...)` plus `execute_validated_plan(...)` when you want explicit preflight validation
and a reusable validated artifact instead of a one-shot call.

## Published Language Boundary

`htmlcut-v1` is not a JSON alias for `htmlcut-core` request/result types.

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

## Supported v1 Capability

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

Core execution errors preserve the upstream diagnostic without collapsing its measurement context.
`InteropError.details.core_diagnostic_code` is the exact uppercase HTMLCut diagnostic code, and
`InteropError.details.core_details.candidateCount` is always the number of candidates found before
selection. This includes `NO_MATCH`, for which the count is `0`; other core diagnostic details
remain alongside it in `core_details`.

Invalid CSS selectors use the exact human-readable message `CSS selector is invalid.`. HTMLCut
does not expose the selector text, upstream parser prose, debug formatting, or parser-internal
types in that message. Instead, both `diagnostics[].details.selector_parse` and
`details.core_details.selector_parse` carry the same closed object:

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
validation rejects a non-canonical invalid-selector message and each distinct selector-detail
failure: missing, malformed, non-object, zero-position, unknown class, or mismatched copy.
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
`interop_contract_rejection` detail preserves a closed rejection code plus the exact rejected
diagnostic count and diagnostic-code counts; it never copies unbounded or invalid diagnostic
payloads into the fallback.

## DOM Canonicalization

Construct a `DomCanonicalization` policy, then pass it to a CSS-selector plan with
`Plan::with_dom_canonicalization`:

```rust
use htmlcut_core::interop::v1::{AttributeName, DomCanonicalization};

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

- `stable_json_v1`
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

`crates/htmlcut-core/tests/fixtures/htmlcut-v1/`

The acceptance runner that freezes the profile and proves repeated executions are byte-identical
to the golden documents lives in:

`crates/htmlcut-core/src/tests/interop_v1/acceptance.rs`

## Important v1 Limits

These are intentionally not part of `htmlcut-v1`:

- XPath
- regex window extraction
- text-anchor extraction
- HTMLCut-owned fetch orchestration
- browser automation inside HTMLCut

`htmlcut.result` also intentionally excludes runtime timing fields so result JSON and result digests stay deterministic across runs.

`HtmlInput` is intentionally not part of the JSON schema registry because it is a Rust-only in-process source handoff type, not a persisted or exchanged JSON document.

## Versioning Rule

`htmlcut_core::interop::v1` is versioned through its exported schema families.

When `Plan`, `InteropResult`, or `InteropError` changes shape, update the corresponding integer
schema version, refresh the acceptance fixtures, and ship the docs change in the same release.
DOM canonicalization and CSS-only `plain_text` are represented by `htmlcut.plan@8`; rendered and
plain comparison output are represented by `htmlcut.result@9`. Bounded diagnostic messages and the
validated selector-parse error envelope are represented by `htmlcut.error@3` and
`htmlcut.result@9`.
The maintained policy details live in [versioning-policy.md](versioning-policy.md).
