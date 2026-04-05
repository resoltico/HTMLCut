# FFHN Interop Guide

The [`FFHN`](https://github.com/resoltico/ffhn) integration surface is:

- module: `htmlcut_core::interop::ffhn_v1`
- profile: `ffhn-htmlcut-v1`

This is a library integration surface, not a CLI command and not an operation catalog entry.

The FFHN interop JSON contracts are also exported through the general HTMLCut schema registry.

## Ownership Boundary

FFHN owns:

- target definition
- fetch policy
- redirects, timeouts, headers, browser use, retries
- decoded HTML input
- comparison and persistence

HTMLCut owns:

- plan validation for `ffhn-htmlcut-v1`
- extraction execution
- typed result and error documents
- stable JSON serialization
- deterministic digests

## Public API

Use:

- `validate_ffhn_plan(&FfhnPlan) -> Result<(), Box<FfhnError>>`
- `execute_ffhn_plan(&FfhnSourceInput, &FfhnPlan) -> Result<FfhnResult, Box<FfhnError>>`

Main types:

- `FfhnSourceInput`
- `FfhnPlan`
- `FfhnResult`
- `FfhnError`

Validator discovery:

- `htmlcut schema --name htmlcut.ffhn_plan --schema-version 1 --output json`
- `htmlcut schema --name htmlcut.ffhn_result --schema-version 1 --output json`
- `htmlcut schema --name htmlcut.ffhn_error --schema-version 1 --output json`

Rust callers can also use `htmlcut_core::schema_catalog()` and `schema_descriptor(...)`.

## Minimal Embedding Example

```rust
use htmlcut_core::SelectorQuery;
use htmlcut_core::interop::ffhn_v1::{
    FfhnNormalization, FfhnOutput, FfhnOutputKind, FfhnPlan, FfhnPlanStrategy, FfhnSelection,
    FfhnSourceInput, FfhnWhitespaceMode, execute_ffhn_plan, validate_ffhn_plan,
};
use url::Url;

let source = FfhnSourceInput::new(
    "example_news",
    "<article><h1>Headline</h1></article>",
)
.unwrap()
.with_input_base_url(Url::parse("https://example.com/news/").unwrap());

let plan = FfhnPlan::new(
    FfhnPlanStrategy::css_selector(SelectorQuery::new("article h1").unwrap()),
    FfhnSelection::single(),
    FfhnOutput::new(FfhnOutputKind::Text),
    FfhnNormalization::new(FfhnWhitespaceMode::Normalize, false),
);

validate_ffhn_plan(&plan).unwrap();
let result = execute_ffhn_plan(&source, &plan).unwrap();

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

`crates/htmlcut-core/tests/fixtures/ffhn-htmlcut-v1/`

The acceptance runner that freezes the profile lives in:

`crates/htmlcut-core/tests/ffhn_v1_acceptance.rs`

## Important v1 Limits

These are intentionally not part of `ffhn-htmlcut-v1`:

- XPath
- regex window extraction
- text-anchor extraction
- all-match compare aggregation
- HTMLCut-owned fetch orchestration
- browser automation inside HTMLCut
- attribute extraction

Because attribute extraction is out of scope, `missing_attribute` is not part of the FFHN v1 error
vocabulary.

`htmlcut.ffhn_result` also intentionally excludes runtime timing fields so result JSON and result
digests stay deterministic across runs.

`FfhnSourceInput` is intentionally not part of the JSON schema registry because it is a Rust-only
in-process source handoff type, not a persisted or exchanged JSON document.

## Versioning Rule

`ffhn-htmlcut-v1` is frozen.

If FFHN later needs capability that v1 does not expose, add a new versioned interop profile and a
new adapter surface. Do not mutate `ffhn-htmlcut-v1` in place.
