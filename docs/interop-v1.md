---
afad: "4.0"
version: "10.1.0"
domain: INTEROP
updated: "2026-05-05"
route:
  keywords: [interop, v1, htmlcut-v1, execute_plan, prepare_plan, execute_validated_plan, ValidatedPlan, HtmlInput, Plan, InteropResult, interop profile]
  questions: ["how do I embed htmlcut extraction into a downstream project?", "what is the htmlcut interop v1 API?", "what schemas does htmlcut interop v1 export?"]
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
- `InteropDiagnostic`
- `ByteRange`

Validator discovery:

- `htmlcut schema --name htmlcut.plan --schema-version 4 --output json`
- `htmlcut schema --name htmlcut.plan --schema-version 5 --output json`
- `htmlcut schema --name htmlcut.result --schema-version 6 --output json`
- `htmlcut schema --name htmlcut.error --schema-version 2 --output json`

Rust callers can also use `htmlcut_core::schema_catalog()` and `schema_descriptor(...)`.

Deterministic JSON and digest helpers:

- `Plan::stable_json()` / `Plan::digest_sha256()`
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
    Output::text(),
    Rendering::new(TextWhitespace::Normalize, false),
);

let prepared = prepare_plan(&plan).unwrap();
let result = execute_validated_plan(&source, &prepared).unwrap();

assert_eq!(result.output.kind().as_str(), "text");
assert_eq!(result.selected_matches[0].text_output, "# Headline");
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
- diagnostics: `InteropDiagnostic`
- byte ranges: `ByteRange`

That boundary lets `htmlcut-core` evolve its internal request/result vocabulary without forcing
downstream consumers to deserialize core-only types or internal structured payloads directly.

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

- CSS selector: `text`, `inner_html`, `outer_html`, `attribute`, `structured`
- delimiter pair: `text`, `inner_html`, `outer_html`, `selected_html`, `attribute`, `structured`

When `Output::attribute { name }` is selected and the chosen candidate does not expose that
attribute, the error contract uses `error_code = "missing_attribute"`.

Every successful `htmlcut.result` carries one top-level `output` object that records the requested
published output contract. Every `SelectedMatch` then carries:

- `output_value` for the exact requested output payload
- `text_output`, which uses the selected-match text projection rather than whole-document
  reader cleanup for the selected root itself
- `selected_html_output` when the strategy is `delimiter_pair`
- `inner_html_output`, which for `delimiter_pair` means the HTML between the two matched
  boundaries
- `outer_html_output`
- typed `metadata`

## Determinism Rules

The interop surface is versioned around:

- `stable_json_v1`
- SHA-256 digests over canonical JSON
- fixture-backed acceptance coverage

The acceptance corpus lives under:

`crates/htmlcut-core/tests/fixtures/htmlcut-v1/`

The acceptance runner that freezes the profile lives in:

`crates/htmlcut-core/tests/v1_acceptance.rs`

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
schema version, refresh the acceptance fixtures, and ship the docs change in the same release
slice.
The maintained policy details live in [versioning-policy.md](versioning-policy.md).
