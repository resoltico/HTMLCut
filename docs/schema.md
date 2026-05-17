---
afad: "4.0"
version: "10.1.0"
domain: SCHEMA
updated: "2026-05-13"
route:
  keywords: [schema registry, htmlcut.plan, htmlcut.result, htmlcut.error, htmlcut-json-schema-v1, HtmlInput, schema inventory]
  questions: ["what schemas does HTMLCut export?", "what are the htmlcut-v1 schema names?", "why is HtmlInput not in the schema registry?"]
---

# Schema Guide

HTMLCut exports a validator-grade JSON schema registry for its maintained public JSON contracts.

This is the authority for machine validation.

Do not scrape help text or infer JSON shapes from examples when a schema exists here.

## Registry Surfaces

CLI:

```bash
htmlcut schema --output json
htmlcut schema --name htmlcut.extraction_result --output json
htmlcut schema --name htmlcut.extraction_definition --output json
htmlcut schema --name htmlcut.result --schema-version 6 --output json
```

Rust:

```rust
use htmlcut_core::{
    CORE_RESULT_SCHEMA_NAME, CORE_RESULT_SCHEMA_VERSION, schema_catalog, schema_descriptor,
};

let registry = schema_catalog();
assert!(!registry.is_empty());

let extraction_result =
    schema_descriptor(CORE_RESULT_SCHEMA_NAME, CORE_RESULT_SCHEMA_VERSION).unwrap();
assert_eq!(extraction_result.owner, "core");

let schema_json = (extraction_result.json_schema)().unwrap();
assert!(schema_json.is_object());
```

The exported registry profile is:

- `htmlcut-json-schema-v1`

## Current Schema Inventory

Stable schema families are grouped by owner:

This inventory is completeness-linted against the live schema registry. If the registry gains or
loses a schema family, this guide is expected to change in the same diff.

Core request contracts:

- `htmlcut.source_request`
- `htmlcut.runtime_options`
- `htmlcut.inspection_options`
- `htmlcut.extraction_request`
- `htmlcut.extraction_definition`

Core result contracts:

- `htmlcut.extraction_result`
- `htmlcut.source_inspection_result`

CLI report contracts:

- `htmlcut.catalog_report`
- `htmlcut.schema_report`
- `htmlcut.extraction_report`
- `htmlcut.source_inspection_report`
- `htmlcut.error_report`

Interop v1 contracts:

- `htmlcut.plan`
- `htmlcut.result`
- `htmlcut.error`

Use `htmlcut schema --output json` when you need the current integer schema versions for those
families. The registry output is the authoritative version inventory.

The interop families are their own published language, not schema aliases for generic core
request/result types. Their selector text, delimiter boundaries, output contract, diagnostics, and
byte ranges are owned by `htmlcut_core::interop::v1`.

## Catalog Relationship

`htmlcut catalog` and `htmlcut schema` are separate on purpose.

- `catalog` answers: what operations exist and how are they invoked?
- `schema` answers: what are the exact JSON contracts behind those operations and reports?

Each catalog operation carries:

- request `rust_shape`
- request `schema_refs`
- result `rust_shape`
- result `schema_refs`
- unconditional defaults
- conditional default overrides
- command constraints

Use those refs with the schema registry instead of treating catalog prose as the validator surface.

For slice extraction, the request/result family is mode-correct:

- slice request documents carry a nested `pattern` object
- literal slice patterns carry `mode`, `from`, and `to` with no regex `flags`
- regex slice patterns carry `mode`, `from`, `to`, and `flags`
- request-side slice documents serialize boundary retention as one named
  `boundary_retention` enum, not as paired boolean mode flags

The request-side value enums serialize HTML fragment modes explicitly:

- selector extraction uses `inner-html` and `outer-html`
- slice extraction distinguishes `selected-html`, `inner-html`, and `outer-html`

The request-side schema family also covers:

- non-zero `max_bytes`, `fetch_timeout_ms`, and `fetch_connect_timeout_ms` values in
  `RuntimeOptions`
- `fetch_preflight` and `tls_trust` in `RuntimeOptions`
- replayable request URLs that must be absolute HTTP(S) without userinfo, query, or fragment
- public display URLs that may carry only the explicit `?[redacted]` query marker
- reusable serialized CLI/core requests through `ExtractionDefinition`

Those exported request/result schema roots are owned by the explicit
`htmlcut_core::wire::v1::*Document` DTO layer. The schema registry does not derive its top-level
wire contract directly from the in-process domain structs.

## Structured Match Metadata

Structured extraction emits a typed metadata union for every match.

In JSON Schema, `ExtractionMatchMetadata` is a `oneOf` over two variants:

- `kind = selector` with selector metadata such as `path`, `tag_name`, and rewritten attributes
- `kind = delimiter-pair` with byte ranges plus `include_start`, `include_end`, `matched_start`,
  and `matched_end`

That union is part of the maintained public contract, not an implementation detail. Downstream
validators should use the discriminator instead of treating `metadata` as loose JSON.

The structured `value` payload also carries collection context:

- `matchIndex`
- `matchCount`
- `candidateIndex`
- `candidateCount`

For slice structured payloads, the HTML fields are intentionally distinct:

- `selectedHtmlOutput` is the exact selected fragment
- `innerHtmlOutput` is the HTML between the two matched boundaries
- `outerHtmlOutput` includes both matched boundaries

That lets downstream callers reason about `--match all` result sets without reconstructing context
from outer report fields alone.

For URL-backed source metadata:

- `value` is a safe display form and never includes URL userinfo
- `effective_base_url` appears only after document parsing and base resolution succeed
- load or pre-parse failures can carry `input_base_url` without claiming an effective base

Successful source loads expose `SourceMetadata.load_steps`, a structured trace of the load actions
HTMLCut took. URL-backed reports use that to record whether `HEAD` preflight succeeded, was
skipped, fell back, or failed before the final `GET`.

`htmlcut.source_inspection_result` now includes `document.extraction_candidates` and
`document.reading_candidates`, first-class lists of suggested selectors with DOM paths and subtree
counts. That split is part of the public inspection contract, not formatter-only CLI sugar.

`htmlcut.error_report` reuses that same `SourceLoadStep` shape as `source_load_steps` when a
JSON-mode CLI failure already reached the traced source-loading path before aborting.

CLI error-report `code` fields are typed unions at the Rust/schema layer:

- core-side `DiagnosticCode` values when the CLI is projecting a core diagnostic directly
- CLI-owned `CliErrorCode` values for parse, request-file, output, and catalog/contract failures

## Top-Level Document Identity

Every maintained public JSON document exported by HTMLCut carries:

- `schema_name`
- `schema_version`

That applies to:

- `ExtractionResult`
- `SourceInspectionResult`
- CLI extraction reports
- CLI source inspection reports
- CLI catalog reports
- CLI schema reports
- CLI error reports
- interop v1 plan/result/error documents

## Interop Note

The interop v1 schemas are exported through the same registry, but `HtmlInput` is not.

That is intentional.

`HtmlInput` is a Rust-only in-process input type because downstream applications own fetch and
decoded HTML delivery in production flows.

## Versioning Rule

Generic HTMLCut schemas are versioned and may hard-break by version when architecture quality
requires it.

Interop v1 documents are versioned under one stable profile string:

- `htmlcut-v1`

When the interop plan/result/error contracts change, update their integer schema versions, tests,
fixtures, and maintained docs in the same change.
The maintained policy details live in [versioning-policy.md](versioning-policy.md).
