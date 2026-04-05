# Architecture Guide

HTMLCut has three maintained surfaces:

1. `htmlcut-cli`
2. `htmlcut-core`
3. `htmlcut_core::interop::ffhn_v1`

They are related, but they are not interchangeable.

## Use The Right Surface

Use `htmlcut-cli` when you need:

- command-line operation
- file, URL, or stdin workflows
- schema export for CLI/agent validation
- stdout rendering
- bundle artifacts
- exit-code semantics

Use `htmlcut-core` when you need:

- in-process extraction or inspection
- typed request and result contracts
- canonical diagnostics
- operation discovery through `operation_catalog()`
- schema discovery through `schema_catalog()`

Use `htmlcut_core::interop::ffhn_v1` only when you are implementing FFHN against the frozen
`ffhn-htmlcut-v1` contract.

It is not a generic replacement for `htmlcut-core`, and it is intentionally not exposed as a CLI
command.

## Ownership Boundary

`htmlcut-core` owns:

- source loading for generic HTMLCut workflows
- HTML parsing
- selector extraction
- slice extraction
- inspection and preview
- diagnostics
- canonical operation IDs and operation catalog entries

`htmlcut-cli` owns:

- argument parsing
- help text
- human vs JSON rendering
- bundles
- exit codes

`htmlcut_core::interop::ffhn_v1` owns:

- FFHN plan validation
- FFHN plan to core-request compilation
- typed FFHN result and error documents
- stable JSON and digest helpers for the frozen interop profile

FFHN owns fetch and compare. HTMLCut does not fetch on FFHN's behalf in production interop flows.

## Dependency Direction

The maintained dependency direction is:

1. `htmlcut-cli` -> `htmlcut-core`
2. `ffhn-core` -> `htmlcut-core`
3. `ffhn-core` -> `htmlcut_core::interop::ffhn_v1`

Forbidden shapes:

- downstreams shelling out to `htmlcut-cli` instead of using `htmlcut-core`
- FFHN relying on HTMLCut URL loading in production
- the CLI inventing behavior that `htmlcut-core` does not own

## Discovery Model

For CLI and agent discovery, use:

```bash
htmlcut catalog --output json
htmlcut catalog --operation select.extract --output text
htmlcut schema --name htmlcut.extraction_report --output json
```

For Rust-side discovery, use:

```rust
use htmlcut_core::{operation_catalog, schema_catalog};

let operations = operation_catalog();
assert!(!operations.is_empty());
let schemas = schema_catalog();
assert!(!schemas.is_empty());
```

The catalog is owned by `htmlcut-core`. The CLI projects that same catalog; it does not maintain a
separate capability map.

For CLI-exposed operations, the catalog also carries a machine-readable command contract:

- invocation
- defaults
- modes
- request/result schema refs
- parameter inventory with requiredness and allowed values
- notes
- examples

That is the stable capability-discovery surface agents should prefer over parsing help text ad hoc.

For validator-grade contract discovery, use `htmlcut schema` or `schema_catalog()`. Do not treat
catalog prose as a schema substitute.

## Versioning And Breakage

HTMLCut does not preserve weak architecture for the sake of compatibility theater.

The rule is:

- generic CLI/core contracts may hard-break when architecture quality requires it
- product-specific downstream interop must be versioned explicitly

That is why FFHN integrates through `ffhn-htmlcut-v1` instead of through ad hoc CLI behavior or a
mutable undocumented internal API.

## Doc Map

Use these docs together:

- [CLI Developer Guide](cli.md)
- [Core Developer Guide](core.md)
- [Schema Guide](schema.md)
- [FFHN Interop Guide](ffhn-interop.md)
- [Operation Matrix](operations.md)
- [Platform Support](platform-support.md)
- [Quality Gates](quality-gates.md)
- [Release Protocol](release-protocol.md)
