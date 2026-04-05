# Schema Guide

HTMLCut exports a validator-grade JSON schema registry for its maintained public JSON contracts.

This is the authority for machine validation.

Do not scrape help text or infer JSON shapes from examples when a schema exists here.

## Registry Surfaces

CLI:

```bash
htmlcut schema --output json
htmlcut schema --name htmlcut.extraction_result --output json
htmlcut schema --name htmlcut.ffhn_result --schema-version 1 --output json
```

Rust:

```rust
use htmlcut_core::{schema_catalog, schema_descriptor};

let registry = schema_catalog();
assert!(!registry.is_empty());

let extraction_result = schema_descriptor("htmlcut.extraction_result", 3).unwrap();
assert_eq!(extraction_result.owner_surface, "htmlcut-core");
```

The exported registry profile is:

- `htmlcut-json-schema-v1`

## Current Schema Inventory

Core contracts:

- `htmlcut.source_request@2`
- `htmlcut.runtime_options@2`
- `htmlcut.inspection_options@2`
- `htmlcut.extraction_request@2`
- `htmlcut.extraction_result@3`
- `htmlcut.source_inspection_result@2`

CLI report contracts:

- `htmlcut.catalog_report@3`
- `htmlcut.schema_report@1`
- `htmlcut.extraction_report@3`
- `htmlcut.source_inspection_report@2`

Frozen FFHN interop contracts:

- `htmlcut.ffhn_plan@1`
- `htmlcut.ffhn_result@1`
- `htmlcut.ffhn_error@1`

## Catalog Relationship

`htmlcut catalog` and `htmlcut schema` are separate on purpose.

- `catalog` answers: what operations exist and how are they invoked?
- `schema` answers: what are the exact JSON contracts behind those operations and reports?

Each catalog operation now carries:

- request `rust_shape`
- request `schema_refs`
- result `rust_shape`
- result `schema_refs`
- unconditional defaults
- conditional default overrides
- command constraints

Use those refs with the schema registry instead of treating catalog prose as the validator surface.

For slice extraction, the request/result family is now mode-correct:

- literal slices carry `mode`, `from`, and `to` with no regex `flags`
- regex slices carry `mode`, `from`, `to`, and `flags`

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
- FFHN interop plan/result/error documents

## FFHN Note

The frozen FFHN interop schemas are exported through the same registry, but `FfhnSourceInput` is
not.

That is intentional.

`FfhnSourceInput` is a Rust-only in-process input type because FFHN owns fetch and decoded HTML
delivery in production flows.

## Versioning Rule

Generic HTMLCut schemas are versioned and may hard-break by version when architecture quality
requires it.

FFHN interop schemas are frozen by profile:

- `ffhn-htmlcut-v1`

Do not mutate the FFHN v1 schemas in place. Add a new interop profile instead.
