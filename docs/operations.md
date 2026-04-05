# Operation Matrix

This document is the developer-facing matrix for HTMLCut's canonical operations.

The code-level source of truth lives in `htmlcut-core`:

- `OperationId`
- `OperationDescriptor`
- `OPERATION_CATALOG`

Those identifiers are valid because they refer only to real product operations that callers can invoke across the CLI and embeddable core. They are not decorative labels.

## Rules

- Operation IDs are stable domain identifiers, not implementation nicknames.
- Operation IDs exist only for canonical product operations.
- Flags, helper functions, internal builders, and request fields do not get operation IDs.
- Failure classes already have their own stable identifier system through diagnostic `code` values.
- The CLI must project the canonical operation IDs from `htmlcut-core`; it must not invent a second taxonomy.
- `htmlcut catalog` must stay derived from the same canonical operation IDs instead of inventing a separate capability list.
- `htmlcut schema` must stay aligned with the schema refs emitted by `htmlcut catalog`.

## Matrix

Use `htmlcut catalog` for the machine-readable operation matrix and `htmlcut schema` for the
validator-grade JSON contracts referenced by that matrix.

| Operation ID | CLI surface | Core surface | Request shape | Result shape | Notes |
| --- | --- | --- | --- | --- | --- |
| `document.parse` | none | `parse_document` | `SourceRequest + RuntimeOptions` | `ParseDocumentResult` | Core-only document loading and parsing for in-process callers. |
| `source.inspect` | `inspect source` | `inspect_source` | `SourceRequest + RuntimeOptions + InspectionOptions` | `SourceInspectionResult` | Source analysis and introspection. |
| `select.preview` | `inspect select` | `preview_extraction` with selector strategy | `ExtractionRequest + RuntimeOptions` | `ExtractionResult` | Structured selector preview before final extraction. Selection modes include exact-one `single`, `first`, `nth`, and `all`. |
| `slice.preview` | `inspect slice` | `preview_extraction` with slice strategy | `ExtractionRequest + RuntimeOptions` | `ExtractionResult` | Structured literal or regex slice preview before final extraction. Slice boundaries support precise `include_start` and `include_end` semantics. |
| `select.extract` | `select` | `extract` with selector strategy | `ExtractionRequest + RuntimeOptions` | `ExtractionResult` | Final selector extraction. Selection modes include exact-one `single`, `first`, `nth`, and `all`. |
| `slice.extract` | `slice` | `extract` with slice strategy | `ExtractionRequest + RuntimeOptions` | `ExtractionResult` | Final literal or regex slice extraction. Slice boundaries support precise `include_start` and `include_end` semantics. |

## Interop Boundary

The FFHN adapter lives in `htmlcut_core::interop::ffhn_v1`.

It is intentionally **not** an operation ID because it is a versioned library integration profile,
not a user-facing product operation exposed across the CLI and the generic core catalog.

## Change Contract

Any change to the operation surface must update all of the following together:

1. `htmlcut-core` operation catalog and any affected result contracts.
2. CLI report projection so the CLI keeps surfacing the same canonical IDs.
3. `htmlcut catalog` so the CLI's discovery surface stays aligned with the same IDs, summaries, and schema refs.
4. `htmlcut schema` so the exported JSON schema registry stays aligned with the same contracts.
5. CLI/core parity tests.
6. This file.
7. `changelog.md` under `Unreleased` if the external or maintainer-visible surface changed.

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
