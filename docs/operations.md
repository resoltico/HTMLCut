---
afad: "4.0"
version: "14.0.0"
domain: OPERATIONS
updated: "2026-09-24"
route:
  keywords: [operation matrix, operation catalog, elements.explore, target.propose, select.extract, slice.extract, source.inspect, interop boundary, change contract]
  questions: ["what are HTMLCut's canonical operations?", "which surfaces must stay aligned when an operation changes?", "why is interop v2 not an operation id?"]
---

# Operation Matrix

This document is the developer-facing matrix for HTMLCut's canonical operations.

The code-level source of truth lives in two maintained registries:

- `htmlcut-core` owns `OperationId`, `OperationDescriptor`, and `OPERATION_CATALOG`
- `htmlcut-cli` owns `htmlcut_cli::contract::OperationCliContract`
- `htmlcut-cli` owns `htmlcut_cli::contract::cli_operation_catalog`
- `htmlcut-cli` owns `htmlcut_cli::contract::cli_operation_contract`

Those identifiers are valid because they refer only to real product operations that callers can invoke across the CLI and embeddable core. They are not decorative labels.

## Rules

- Operation IDs are stable domain identifiers, not implementation nicknames.
- Operation IDs exist only for canonical product operations.
- Flags, helper functions, internal builders, and request fields do not get operation IDs.
- Failure classes already have their own stable identifier system through diagnostic `code` values.
- CLI-facing command paths, defaults, mode inventories, parameter rules, and examples are owned by the `htmlcut_cli::contract` registry, not rebuilt ad hoc elsewhere.
- The CLI must project the canonical operation IDs from `htmlcut-core`; it must not invent a second taxonomy.
- CLI-visible operations map to one canonical command path each. Hidden aliases do not get their own parallel contract surface.
- `htmlcut catalog` must stay derived from the same canonical operation IDs instead of inventing a separate capability list.
- `htmlcut schema` must stay aligned with the schema refs emitted by `htmlcut catalog`.

## Matrix

Use `htmlcut catalog` for the machine-readable operation matrix and `htmlcut schema` for the
validator-grade JSON contracts referenced by that matrix.

This matrix is completeness-linted against `htmlcut-core`'s operation catalog. The table is a
maintained human guide, but it is not allowed to silently drift away from the canonical registry.

| Operation ID | CLI surface | Core surface | Request shape | Result shape | Notes |
| --- | --- | --- | --- | --- | --- |
| `source.inspect` | `inspect source` | `inspect source` | `source request + runtime options + inspection options` | `source inspection result` | Source analysis and introspection. |
| `elements.explore` | `inspect elements` | `explore prepared elements` | `prepared document + exploration options` | `htmlcut.exploration_result@1` | Bounded static element discovery with same-snapshot selector proposals and cursor evidence. |
| `target.propose` | `inspect propose` | `resolve target and propose` | `prepared document + complete target hint + target-resolution options` | `htmlcut.target_resolution_result@1` | Fail-closed browser-target resolution and same-snapshot selector proposals; path alone is never sufficient. |
| `select.preview` | `inspect select` | `preview selector extraction` | `extraction request + runtime options` | `extraction result` | Selector preview before final extraction. Preview accepts the same value modes as final selector extraction while surfacing match metadata. |
| `slice.preview` | `inspect slice` | `preview slice extraction` | `extraction request + runtime options` | `extraction result` | Literal or regex slice preview before final extraction. Slice requests model named boundary-retention modes, while structured match metadata reports the realized `include_start` / `include_end` facts. |
| `select.extract` | `select` | `extract selector values` | `extraction request + runtime options` | `extraction result` | Final selector extraction. Selection modes include exact-one `single`, `first`, `nth`, and `all`. |
| `slice.extract` | `slice` | `extract slice values` | `extraction request + runtime options` | `extraction result` | Final literal or regex slice extraction. Slice requests model named boundary-retention modes, while structured match metadata reports the realized `include_start` / `include_end` facts. |

## Interop Boundary

The downstream interop adapter lives in `htmlcut_core::interop::v2`. Its prepared-document APIs own
the core surfaces behind `elements.explore` and `target.propose`.

It is intentionally **not** an operation ID because it is a versioned library integration profile,
not a user-facing product operation exposed across the CLI and the generic core catalog.

## Change Contract

Any change to the operation surface must update all of the following together:

1. `htmlcut-core` operation catalog and any affected result contracts.
2. `htmlcut_cli::contract` metadata so invocation strings, mode inventories, parameter rules, examples, and normalized command labels stay canonical.
3. CLI report projection so the CLI keeps surfacing the same canonical IDs and command contracts.
4. `htmlcut catalog` so the CLI's discovery surface stays aligned with the same IDs, summaries, and schema refs.
5. `htmlcut schema` so the exported JSON schema registry stays aligned with the same contracts.
6. Contract-lint coverage and CLI/core parity tests.
7. This file.
8. `changelog.md` under `Unreleased` if the external or maintainer-visible surface changed.

## Design Boundary

Do not add operation IDs for:

- individual flags
- request fields
- helper functions
- versioned interop adapter functions
- private builders
- rendering helpers
- diagnostics

That would turn the system into taxonomy noise. Keep the operation catalog small, stable, and meaningful.
