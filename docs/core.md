---
afad: "3.5"
version: "4.1.0"
domain: CORE
updated: "2026-04-19"
route:
  keywords: [core, extract, inspect_source, preview_extraction, operation_catalog, schema_catalog, typed requests, diagnostics]
  questions: ["what is the maintained htmlcut-core surface?", "what does the core schema registry cover?", "how should a Rust caller embed htmlcut-core?"]
---

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
- `cli_operation_catalog`
- `cli_operation_contract`
- `cli_operation_display_command`
- `cli_operation_report_command`
- `find_cli_operation_by_command_path`
- `schema_catalog`
- `schema_descriptor`

The crate root intentionally keeps only the stable high-level API and top-level request/result
types.

Detailed contract types now live behind explicit namespaces:

- `htmlcut_core::request` for typed request-side contracts
- `htmlcut_core::result` for typed result-side contracts such as `ExtractionMatch`,
  `ExtractionStats`, `Range`, and structured metadata enums

## Core Request Model

The request surface is typed and intentionally rejects invalid states at construction time.

Key request types:

- `SourceRequest`
- `ExtractionRequest`
- `ExtractionDefinition`
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
- reusable request files serialize `ExtractionDefinition`, which owns the full `ExtractionRequest`
  plus `RuntimeOptions`
- URL loading defaults to `FetchPreflightMode::HeadFirst` with an explicit `GetOnly` escape hatch
- structured extraction metadata is typed, not loose JSON

## Result Model

Core execution returns:

- `ParseDocumentResult`
- `SourceInspectionResult`
- `ExtractionResult`

Diagnostics are first-class and machine-readable through:

- `Diagnostic`
- `DiagnosticCode`
- `DiagnosticLevel`

Core result reports carry canonical `OperationId` values so every surface speaks the same operation
taxonomy.

The JSON-bearing core result documents also carry:

- `schema_name`
- `schema_version`

When structured extraction returns collections, each structured value also carries:

- `matchIndex`
- `matchCount`
- `candidateIndex`
- `candidateCount`

## Operation Catalog

`htmlcut-core` owns the canonical operation catalog.

Each operation descriptor includes:

- stable operation id
- CLI surface when one exists
- core surface
- request contract with `rust_shape` and `schema_refs`
- result contract with `rust_shape` and `schema_refs`
- summary

That catalog is the source of truth for the operation matrix.

For CLI-exposed operations, `htmlcut-core` also owns a companion `OperationCliContract` registry.
That companion catalog carries:

- concrete command path tokens
- invocation synopsis
- typed match/value/output mode inventories
- typed default values and conditional default overrides
- parameter inventory with typed requiredness rules
- cross-parameter constraints
- catalog notes and example invocations

That CLI contract registry is the source of truth for `htmlcut catalog`, `command_name`
normalization in CLI reports, and contract-lint coverage over help/examples.
Use `operation_catalog()` and `cli_operation_catalog()` for discovery inside Rust callers instead
of maintaining your own shadow matrix of supported behaviors.

## Schema Registry

`htmlcut-core` also owns the core-side schema registry.

Use:

- `schema_catalog()` to enumerate exported schemas
- `schema_descriptor(name, version)` to resolve one exact schema

That registry covers:

- core request/result contracts
- reusable extraction-definition documents
- frozen interop v1 documents

It does not cover CLI-only report documents. Those are added by `htmlcut-cli` on the CLI side.

## Minimal Embedding Example

```rust
use htmlcut_core::{
    extract, operation_catalog,
    request::{
        AttributeName, ExtractionRequest, ExtractionSpec, NormalizationOptions, SelectorQuery,
        SelectionSpec, SourceRequest, ValueSpec,
    },
    result::ExtractionMatchMetadata,
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
assert_eq!(
    result.matches[0].value.as_str(),
    Some("https://example.com/guide.html")
);
match &result.matches[0].metadata {
    ExtractionMatchMetadata::Selector(metadata) => {
        assert_eq!(metadata.tag_name, "a");
    }
    ExtractionMatchMetadata::DelimiterPair(_) => unreachable!("selector extraction"),
}
assert!(!operation_catalog().is_empty());
```

Use `SourceRequest::memory(...)` when HTML is already loaded by the embedding application. Reserve
`SourceRequest::url(...)` for generic HTMLCut-owned loading workflows.

For a complete reusable-definition round trip, see
`crates/htmlcut-core/examples/reusable_extraction_definition.rs`.

## Design Boundary

`htmlcut-core` owns:

- source loading for generic CLI/core workflows
- HEAD-first URL preflight policy and content-type/size rejection
- document parsing
- inspection
- selector extraction
- slice extraction
- relative-URL rewriting for standard URL-bearing HTML attributes such as `srcset`, `poster`,
  `action`, `ping`, and `meta refresh`
- plain-text rendering for document-shaped HTML including `pre`, inline `code`, blockquotes, and
  definition lists
- diagnostics
- operation catalog

It does not own product-specific orchestration for downstream systems. Those belong in versioned
interop layers or in downstream applications.
