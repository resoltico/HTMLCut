---
afad: "3.5"
version: "4.4.1"
domain: SCHEMA
updated: "2026-04-23"
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
htmlcut schema --name htmlcut.result --schema-version 1 --output json
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
assert_eq!(extraction_result.owner_surface, "htmlcut-core");
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

Frozen interop v1 contracts:

- `htmlcut.plan`
- `htmlcut.result`
- `htmlcut.error`

Use `htmlcut schema --output json` when you need the current integer schema versions for those
families. The registry output is the authoritative version inventory.

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

- literal slices carry `mode`, `from`, and `to` with no regex `flags`
- regex slices carry `mode`, `from`, `to`, and `flags`

The request/result value enums serialize the inner-fragment mode explicitly as
`inner-html`.

The request-side schema family also covers:

- `fetch_preflight` in `RuntimeOptions`
- reusable serialized CLI/core requests through `ExtractionDefinition`

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

That lets downstream callers reason about `--match all` result sets without reconstructing context
from outer report fields alone.

Successful source loads expose `SourceMetadata.load_steps`, a structured trace of the load actions
HTMLCut took. URL-backed reports use that to record whether `HEAD` preflight succeeded, was
skipped, fell back, or failed before the final `GET`.

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
- interop v1 plan/result/error documents

## Interop Note

The frozen interop v1 schemas are exported through the same registry, but `HtmlInput` is not.

That is intentional.

`HtmlInput` is a Rust-only in-process input type because downstream applications own fetch and
decoded HTML delivery in production flows.

## Versioning Rule

Generic HTMLCut schemas are versioned and may hard-break by version when architecture quality
requires it.

Interop v1 schemas are frozen by profile:

- `htmlcut-v1`

Do not mutate the v1 schemas in place. Add a new interop profile instead.
The maintained policy details live in [versioning-policy.md](versioning-policy.md).
