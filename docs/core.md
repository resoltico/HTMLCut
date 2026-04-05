# Core Developer Guide

`htmlcut-core` is the only behavior engine in the workspace.

The CLI does not implement separate extraction logic. It builds core requests, executes them, and
renders the result.

## Public Entry Points

The maintained public surface is:

- `parse_document`
- `inspect_source`
- `preview_extraction`
- `extract`
- `operation_catalog`
- `operation_descriptor`
- `schema_catalog`
- `schema_descriptor`

## Core Request Model

The request surface is typed and intentionally rejects invalid states at construction time.

Key request types:

- `SourceRequest`
- `ExtractionRequest`
- `ExtractionSpec`
- `SelectionSpec`
- `ValueSpec`
- `SliceSpec`
- `RuntimeOptions`
- `OutputOptions`

Important invariants:

- selector exact-one behavior uses `SelectionSpec::single()`
- slice capture is modeled with `include_start` and `include_end`, not coarse inner/outer capture
- slice requests are mode-correct: literal slices do not carry regex flags, regex slices do
- structured extraction metadata is typed, not loose JSON

## Result Model

Core execution returns:

- `ParseDocumentResult`
- `SourceInspectionResult`
- `ExtractionResult`

Diagnostics are first-class and machine-readable through:

- `Diagnostic`
- `DiagnosticLevel`

Core result reports carry canonical `OperationId` values so every surface speaks the same operation
taxonomy.

The JSON-bearing core result documents also carry:

- `schema_name`
- `schema_version`

## Operation Catalog

`htmlcut-core` owns the canonical operation catalog.

Each operation descriptor includes:

- stable operation id
- CLI surface when one exists
- core surface
- request contract with `rust_shape` and `schema_refs`
- result contract with `rust_shape` and `schema_refs`
- summary

That catalog is the source of truth for `htmlcut catalog`.

Use `operation_catalog()` for discovery inside Rust callers instead of maintaining your own shadow
matrix of supported behaviors.

## Schema Registry

`htmlcut-core` also owns the core-side schema registry.

Use:

- `schema_catalog()` to enumerate exported schemas
- `schema_descriptor(name, version)` to resolve one exact schema

That registry covers:

- core request/result contracts
- frozen FFHN interop documents

It does not cover CLI-only report documents. Those are added by `htmlcut-cli` on the CLI side.

## Minimal Embedding Example

```rust
use htmlcut_core::{
    extract, operation_catalog, AttributeName, ExtractionRequest, ExtractionSpec,
    NormalizationOptions, SelectorQuery, SelectionSpec, SourceRequest, ValueSpec,
};
use url::Url;

let source = SourceRequest::memory(
    "inline",
    "<article><a href=\"../guide.html\">Guide</a></article>",
)
.with_base_url(Url::parse("https://example.com/docs/start.html").unwrap());

let request = ExtractionRequest {
    normalization: NormalizationOptions {
        rewrite_urls: true,
        ..Default::default()
    },
    ..ExtractionRequest::new(
        source,
        ExtractionSpec::selector(SelectorQuery::new("article a").unwrap())
            .with_selection(SelectionSpec::single())
            .with_value(ValueSpec::Attribute {
                name: AttributeName::new("href").unwrap(),
            }),
    )
};

let result = extract(&request, &Default::default());
assert!(result.ok);
assert_eq!(result.matches[0].value, "https://example.com/guide.html");
assert!(!operation_catalog().is_empty());
```

Use `SourceRequest::memory(...)` when HTML is already loaded by the embedding application. Reserve
`SourceRequest::url(...)` for generic HTMLCut-owned loading workflows.

## Design Boundary

`htmlcut-core` owns:

- source loading for generic CLI/core workflows
- document parsing
- inspection
- selector extraction
- slice extraction
- diagnostics
- operation catalog

It does not own product-specific orchestration for downstream systems. Those belong in versioned
interop layers or in downstream applications.
