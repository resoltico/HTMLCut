---
afad: "3.5"
version: "4.3.0"
domain: ARCHITECTURE
updated: "2026-04-22"
route:
  keywords: [architecture, surfaces, htmlcut-cli, htmlcut-core, interop v1, ownership boundary, discovery model]
  questions: ["what are the maintained HTMLCut surfaces?", "when should I use htmlcut_core::interop::v1?", "what does HTMLCut own versus downstream consumers?"]
---

# Architecture Guide

HTMLCut has three maintained surfaces:

1. `htmlcut-cli`
2. `htmlcut-core`
3. `htmlcut_core::interop::v1`

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

Use `htmlcut_core::interop::v1` when you need the frozen `htmlcut-v1` downstream integration
contract.

It is the frozen versioned interop surface for downstream integrations, not a replacement for the
broader `htmlcut-core` API, and not a CLI command.

## Ownership Boundary

`htmlcut-core` owns:

- source loading for generic HTMLCut workflows
- HTML parsing
- selector extraction
- slice extraction
- inspection and preview
- diagnostics
- canonical operation IDs and operation catalog entries
- canonical CLI choice domains and spellings for match, value, output, pattern, whitespace, and
  fetch-preflight modes
- canonical CLI help metadata for the maintained command surfaces, including display summaries,
  discovery narratives, and operation-analysis guidance that the CLI renders without rewriting

`htmlcut-cli` owns:

- argument parsing
- clap tree assembly from the core-owned command/help contracts
- human vs JSON rendering
- bundles
- exit codes

`htmlcut_core::interop::v1` owns:

- downstream plan validation for `htmlcut-v1`
- plan-to-core-request compilation for `htmlcut-v1`
- typed interop result and error documents
- stable JSON and digest helpers for the frozen interop profile

Those owners are maintained as focused domain modules, not giant mixed-role files. In practice that
means HTMLCut keeps request contracts, source loading, document handling, extraction execution, and
frozen interop execution/stable-JSON logic in separate seams so the canonical owner for one concern
does not disappear into a monolith.

Downstream applications own fetch, retries, orchestration, comparison, and persistence. HTMLCut
does not fetch on a downstream application's behalf in production interop flows.

## Dependency Direction

The maintained dependency direction is:

1. `htmlcut-cli` -> `htmlcut-core`
2. downstream embedders -> `htmlcut-core`
3. downstream embedders that adopt frozen interop -> `htmlcut_core::interop::v1`

Forbidden shapes:

- downstreams shelling out to `htmlcut-cli` instead of using `htmlcut-core`
- downstream products relying on HTMLCut URL loading in production when they already own fetch
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

The catalog is owned by `htmlcut-core`. The CLI projects that same catalog and the same
core-owned CLI command-contract registry; it does not maintain a separate capability map or a
shadow command-contract builder.

That ownership line is enforced, not merely described. The maintainer gate parses the real clap
command tree and defaulted arguments and fails if they drift away from the core-owned CLI
contract registry. The CLI help surface is expected to render the same core-owned summaries,
analysis text, mode facts, default overrides, notes, and examples instead of hand-authoring a
second behavioral description. The CLI also parses the core-owned choice types directly instead of
defining its own parallel enums for those user-facing values.

For CLI-exposed operations, the catalog also carries a machine-readable command contract:

- invocation
- defaults
- modes
- request/result schema refs
- parameter inventory with requiredness and allowed values
- notes
- examples

That is the stable capability-discovery surface agents should prefer over parsing help text ad hoc.

The same gate also renders the real clap help text, catalog/schema text summaries, and
representative recovery errors and fails if those surfaces mention operation IDs or schema names
that are not registered in `htmlcut-core`.

For validator-grade contract discovery, use `htmlcut schema` or `schema_catalog()`. Do not treat
catalog prose as a schema substitute.

## Versioning And Breakage

HTMLCut does not preserve weak architecture for the sake of compatibility theater.

The rule is:

- generic CLI/core contracts may hard-break when architecture quality requires it
- product-specific downstream interop must be versioned explicitly

That is why downstream consumers integrate through `htmlcut-v1` instead of through ad hoc CLI
behavior or a mutable undocumented internal API.

## Doc Map

Use these docs together:

- [CLI Developer Guide](cli.md)
- [Core Developer Guide](core.md)
- [Schema Guide](schema.md)
- [Interop v1 Guide](interop-v1.md)
- [Operation Matrix](operations.md)
- [Platform Support](platform-support.md)
- [Quality Gates](quality-gates.md)
- [Release Protocol](release-protocol.md)
