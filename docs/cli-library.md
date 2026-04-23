---
afad: "3.5"
version: "4.4.1"
domain: CLI_LIBRARY
updated: "2026-04-23"
route:
  keywords: [cli library, htmlcut_cli, run, command, exit codes, report schemas, typed reports, clap command]
  questions: ["what does the public htmlcut-cli crate export?", "when should I use htmlcut_cli::run instead of htmlcut_core?", "how do I consume typed catalog or schema reports from htmlcut-cli?"]
---

# CLI Library Guide

`htmlcut-cli` is published as both:

- the `htmlcut` binary
- the `htmlcut_cli` Rust crate

This guide covers the Rust crate surface.
[cli.md](cli.md) covers the operator-facing command behavior.

## Use The Right Surface

Prefer `htmlcut_core` when you already want typed in-process extraction or inspection.

Use `htmlcut_cli` when you need one of these exact adapter-level behaviors inside Rust:

- execute the real CLI parser and rendering pipeline
- inspect the canonical clap command tree
- consume the typed CLI report documents that correspond to `catalog`, `schema`, extraction, and
  inspection commands
- reason about CLI exit-code categories directly

## Primary Entry Points

The main root-level helpers are:

- `htmlcut_cli::run(args, stdout, stderr) -> i32`
- `htmlcut_cli::command() -> clap::Command`

`run(...)` executes the full CLI adapter against one argv stream, writes the rendered output, and
returns the canonical exit code. Use it when tests or tools need the actual CLI semantics instead
of a reimplemented wrapper.

`command()` returns the canonical clap tree used by the binary itself. HTMLCut's docs-contract and
CLI contract-lint surfaces use this to verify that help text and parsing behavior still match the
core-owned command contracts.

## Exit Codes

The crate exports stable exit-code constants:

- `EXIT_CODE_INTERNAL`
- `EXIT_CODE_USAGE`
- `EXIT_CODE_SOURCE`
- `EXIT_CODE_EXTRACTION`
- `EXIT_CODE_OUTPUT`

Those constants correspond to the documented CLI failure categories in [cli.md](cli.md).

## Typed Report Surface

`htmlcut_cli` also exports the typed Rust structs and schema constants behind its maintained JSON
reports.

Catalog:

- `CATALOG_REPORT_SCHEMA_NAME`
- `CATALOG_SCHEMA_VERSION`
- `CatalogCommandReport`
- `CatalogOperationReport`
- `CatalogCommandContract`

Schema:

- `SCHEMA_COMMAND_REPORT_SCHEMA_NAME`
- `SCHEMA_COMMAND_REPORT_SCHEMA_VERSION`
- `SchemaCommandReport`
- `SchemaDocumentReport`
- `SchemaRefReport`

Extraction and preview:

- `EXTRACTION_COMMAND_REPORT_SCHEMA_NAME`
- `EXTRACTION_COMMAND_REPORT_SCHEMA_VERSION`
- `ExtractionCommandReport`
- `BundlePaths`

Source inspection:

- `SOURCE_INSPECTION_COMMAND_REPORT_SCHEMA_NAME`
- `SOURCE_INSPECTION_COMMAND_REPORT_SCHEMA_VERSION`
- `SourceInspectionCommandReport`

These types are useful when a Rust caller wants to deserialize or inspect the exact CLI-facing
documents rather than the lower-level `htmlcut_core` result types.

## Boundary

`htmlcut_cli` is still an adapter crate.

It owns:

- argv parsing
- stdout and stderr rendering
- output-file and bundle projection
- exit-code semantics
- CLI report projection

It does not own separate extraction behavior. The actual engine, schema registry, operation
catalog, diagnostics, and interop surface still live in `htmlcut_core`.

## Naming Rule

Use the hyphenated name `htmlcut-cli` in Cargo manifests and install flows.
Use the underscored path `htmlcut_cli` in Rust code.

The full workspace package map lives in [workspace-layout.md](workspace-layout.md).
