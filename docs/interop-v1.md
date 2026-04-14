---
afad: "3.5"
version: "4.0.1"
domain: INTEROP
updated: "2026-04-14"
route:
  keywords: [interop, v1, htmlcut-v1, execute_plan, validate_plan, HtmlInput, Plan, InteropResult, interop profile]
  questions: ["how do I embed htmlcut extraction into a downstream project?", "what is the htmlcut interop v1 API?", "what schemas does htmlcut interop v1 export?"]
---

# HTMLCut Interop v1 Guide

**Purpose**: Embed HTMLCut extraction into a downstream Rust project using the frozen `htmlcut-v1` interop profile.
**Prerequisites**: Rust project with `htmlcut-core` as a Cargo dependency.

## Overview

The `htmlcut_core::interop::v1` module is a frozen, versioned integration surface. It exposes plan construction, plan validation, and plan execution for downstream consumers. The profile name is `htmlcut-v1`.

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
- extraction execution
- typed result and error documents
- stable JSON serialization
- deterministic digests

## Public API

Use:

- `validate_plan(&Plan) -> Result<(), Box<InteropError>>`
- `execute_plan(&HtmlInput, &Plan) -> Result<InteropResult, Box<InteropError>>`

Main types:

- `HtmlInput`
- `Plan`
- `InteropResult`
- `InteropError`

Validator discovery:

- `htmlcut schema --name htmlcut.plan --schema-version 1 --output json`
- `htmlcut schema --name htmlcut.result --schema-version 1 --output json`
- `htmlcut schema --name htmlcut.error --schema-version 1 --output json`

Rust callers can also use `htmlcut_core::schema_catalog()` and `schema_descriptor(...)`.

## Minimal Embedding Example

```rust
use htmlcut_core::SelectorQuery;
use htmlcut_core::interop::v1::{
    HtmlInput, Normalization, Output, OutputKind, Plan, PlanStrategy,
    Selection, TextWhitespace, execute_plan, validate_plan,
};
use url::Url;

let source = HtmlInput::new(
    "example_news",
    "<article><h1>Headline</h1></article>",
)
.unwrap()
.with_input_base_url(Url::parse("https://example.com/news/").unwrap());

let plan = Plan::new(
    PlanStrategy::css_selector(SelectorQuery::new("article h1").unwrap()),
    Selection::single(),
    Output::new(OutputKind::Text),
    Normalization::new(TextWhitespace::Normalize, false),
);

validate_plan(&plan).unwrap();
let result = execute_plan(&source, &plan).unwrap();

assert_eq!(result.selected_match.comparison_input_text, "Headline");
```

## Supported v1 Capability

Strategy kinds:

- `css_selector`
- `delimiter_pair`

Selection modes:

- `single`
- `first`
- `nth`

Output kinds:

- `text`
- `inner_html`
- `outer_html`

## Determinism Rules

The interop surface is frozen around:

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
- all-match compare aggregation
- HTMLCut-owned fetch orchestration
- browser automation inside HTMLCut
- attribute extraction

Because attribute extraction is out of scope, `missing_attribute` is not part of the v1 error vocabulary.

`htmlcut.result` also intentionally excludes runtime timing fields so result JSON and result digests stay deterministic across runs.

`HtmlInput` is intentionally not part of the JSON schema registry because it is a Rust-only in-process source handoff type, not a persisted or exchanged JSON document.

## Versioning Rule

`htmlcut-v1` is frozen.

If a downstream consumer later needs capability that v1 does not expose, add a new versioned interop profile and a new adapter surface. Do not mutate `htmlcut-v1` in place.
The maintained policy details live in [versioning-policy.md](versioning-policy.md).
