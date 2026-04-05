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
- frozen FFHN interop schemas

### `inspect`

`inspect` is the pre-extraction workflow:

- `inspect source` summarizes document structure and base-URL behavior
- `inspect select` previews selector matches in structured form
- `inspect slice` previews slice matches in structured form

`inspect` defaults to JSON. Text mode is for compact human review.

Top-level JSON reports now carry their own schema identity:

- `schema_name`
- `schema_version`

### `select`

`select` extracts from CSS selector matches.

Selection modes:

- `single`
- `first`
- `nth`
- `all`

Value modes:

- `text`
- `html`
- `outer-html`
- `attribute`
- `structured`

### `slice`

`slice` extracts between raw source boundaries.

Boundary semantics are exact:

- literal matching is raw substring matching, not tag-aware
- literal slice requests do not expose regex flags in JSON contracts
- regex boundaries are consumed exactly as matched
- default selection excludes both matched boundaries
- `--include-start` and `--include-end` control boundary inclusion independently
- `--value html` returns the selected fragment as HTML
- `--value outer-html` returns the full outer matched range including both boundaries

Use `inspect slice` before committing to extraction whenever the boundary pattern may consume more
than you intended.

## Output Model

`select` and `slice` separate:

- extraction value: `--value`
- stdout rendering: `--output`

Stdout modes:

- `text`
- `html`
- `json`
- `none`

`--output none` is valid only with `--bundle`.

## Bundle Workflow

`--bundle <dir>` writes:

- `selection.html`
- `selection.txt`
- `report.json`

`selection.html` is a wrapped review artifact, not a byte-for-byte replay of the source document.
`report.json` is the structured execution report.

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

## Slice Preview Rule

`inspect slice --output text` shows:

- selected range data
- selected text
- fragment context when it adds signal

That fragment line is the main debugging aid when a boundary pattern consumes the payload and leaves
the selected text empty.
