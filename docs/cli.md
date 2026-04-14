---
afad: "3.5"
version: "4.0.1"
domain: CLI
updated: "2026-04-14"
route:
  keywords: [cli, catalog, schema, inspect, select, slice, bundle workflow, output model]
  questions: ["what commands does htmlcut-cli expose?", "what does htmlcut schema include?", "how do select and slice outputs work?"]
---

# CLI Developer Guide

`htmlcut-cli` is the operator-facing adapter over `htmlcut-core`.

It exposes five commands:

- `catalog`
- `schema`
- `inspect`
- `select`
- `slice`

## Source Inputs

Every extraction and inspection command accepts one input:

- a local file path
- an `http://` or `https://` URL
- `-` for stdin

`--base-url` sets the input base explicitly. For URL inputs, the request URL is the input base
automatically. If the document contains `<base href>`, HTMLCut resolves it against the input base to
produce the effective base URL used by `--rewrite-urls`.

For URL inputs, HTMLCut now uses HEAD-first fetch preflight by default:

- `head-first` probes status, `Content-Length`, and obvious non-HTML `Content-Type` values before
  issuing the full GET, and it automatically falls back to GET when a server rejects HEAD or
  breaks the HEAD exchange
- `get-only` skips the HEAD probe for servers that still mishandle HEAD badly

The CLI exposes that policy through `--fetch-preflight head-first|get-only`.

## Command Model

### `catalog`

`catalog` is the discovery surface for agents and developers.

Each operation entry carries:

- catalog schema version at the top level
- `operation_id`
- CLI command surface when one exists
- CLI/core availability
- summary
- core surface
- request contract with `rust_shape` and `schema_refs`
- result contract with `rust_shape` and `schema_refs`
- command invocation
- unconditional defaults
- conditional default overrides
- command constraints
- supported modes
- parameter inventory with requiredness, defaults, and allowed values
- stable notes
- examples

Use:

- `htmlcut catalog --output json` for machine-readable discovery
- `htmlcut catalog --operation <id>` when you want one operation in detail

In text mode, the filtered single-operation view also prints the mapped core surface plus the
request/result contracts plus the parameter inventory, typed default overrides, and command
constraints.

### `schema`

`schema` is the validator-grade contract discovery surface.

Use:

- `htmlcut schema --output json` for the full registry
- `htmlcut schema --name <schema_name>` for one schema family
- `htmlcut schema --name <schema_name> --schema-version <n>` for one exact schema version

The registry includes:

- `htmlcut-core` request/result schemas
- `htmlcut-cli` report schemas
- frozen interop v1 schemas

### `inspect`

`inspect` is the pre-extraction workflow:

- `inspect source` summarizes document structure and base-URL behavior
- `inspect select` previews selector matches in structured form
- `inspect slice` previews slice matches in structured form

`inspect` defaults to JSON. Text mode is for compact human review.

`inspect select` and `inspect slice` can also load a reusable extraction-definition file through
`--request-file <PATH>` instead of spelling the source and strategy inline.

Top-level JSON reports now carry their own schema identity:

- `schema_name`
- `schema_version`

### `select`

`select` extracts from CSS selector matches.

It can be driven two ways:

- inline flags such as `<INPUT>` plus `--css`
- `--request-file <PATH>` pointing at a serialized `ExtractionDefinition`

Selection modes:

- `single`
- `first`
- `nth`
- `all`

Value modes:

- `text`
- `inner-html`
- `outer-html`
- `attribute`
- `structured`

### `slice`

`slice` extracts between raw source boundaries.

Like `select`, it supports either inline source/boundary flags or `--request-file <PATH>`.

Boundary semantics are exact:

- literal matching is raw substring matching, not tag-aware
- literal slice requests do not expose regex flags in JSON contracts
- regex boundaries are consumed exactly as matched
- default selection excludes both matched boundaries
- `--include-start` and `--include-end` control boundary inclusion independently
- `--value inner-html` returns the selected fragment as HTML
- `--value outer-html` returns the full outer matched range including both boundaries

`inner-html` is the CLI spelling for the core-side `InnerHtml` value kind.

Use `inspect slice` before committing to extraction whenever the boundary pattern may consume more
than you intended.

## Output Model

`select` and `slice` separate:

- extraction value: `--value`
- stdout rendering: `--output`
- reusable request definition: `--request-file`

Stdout modes:

- `text`
- `html`
- `json`
- `none`

`--output none` is valid only with `--bundle`.

`--output-file <PATH>` writes exactly the stdout payload to one file without creating a bundle.
That works for text, HTML, JSON, and inspection text/JSON outputs. It is intentionally invalid with
`--output none` because there is no stdout payload to write.

## Bundle Workflow

`--bundle <dir>` writes:

- `selection.html`
- `selection.txt`
- `report.json`

`selection.html` is a wrapped review artifact, not a byte-for-byte replay of the source document.
`report.json` is the structured execution report.

## Reusable Definition Files

The CLI now accepts first-class extraction-definition JSON files for:

- `select`
- `slice`
- `inspect select`
- `inspect slice`

Those files serialize the exact `ExtractionRequest` plus `RuntimeOptions` that the CLI would
otherwise build inline. Once `--request-file` is present, inline source and strategy flags are
rejected instead of being merged.

For embeddable Rust callers, the matching core type is `htmlcut_core::ExtractionDefinition`.

## Failure Model

Human output modes print the primary failure to stderr.

JSON modes still exit non-zero on failure, but they emit structured JSON to stdout.

Exit code categories:

- `1` internal
- `2` usage
- `3` source
- `4` extraction
- `5` output

Typical source failures are explicit:

- directory paths are rejected as directory inputs, not as generic read failures
- non-UTF-8 files are rejected as UTF-8 violations, not as opaque host-library errors

For successful runs:

- `--quiet` suppresses non-fatal warnings and progress lines on stderr
- `--verbose` still increases stderr detail, but it intentionally conflicts with `--quiet`
- `--version` prints the tool version plus engine identity, schema profile, and repository metadata

## Slice Preview Rule

`inspect slice --output text` shows:

- selected range data
- selected text
- fragment context when it adds signal

That fragment line is the main debugging aid when a boundary pattern consumes the payload and leaves
the selected text empty.
