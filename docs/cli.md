---
afad: "4.0"
version: "10.3.0"
domain: CLI
updated: "2026-07-15"
route:
  keywords: [cli, catalog, schema, inspect, select, slice, bundle workflow, output model]
  questions: ["what commands does htmlcut-cli expose?", "what does htmlcut schema include?", "how do select and slice outputs work?"]
---

# CLI Developer Guide

`htmlcut-cli` is the operator-facing adapter over `htmlcut-core`.

If you want install choices, a first runnable walkthrough, and request-file examples before you
read the full command model, start with [getting-started.md](getting-started.md).

The clap/help surface is rendered from `htmlcut_cli::contract`, with the root `htmlcut --help`
banner reusing the package version and description from Cargo metadata so the terminal identity
and published crate metadata stay in sync. The CLI does not maintain a second operation/help
taxonomy alongside `htmlcut-core` and `htmlcut_cli::contract`.

This guide owns the operator-facing command model.
The published `htmlcut_cli` library API for programmatic CLI execution, clap-tree inspection, exit
codes, and typed CLI report structs is documented separately in
[cli-library.md](cli-library.md).

It exposes five maintained operator commands:

- `catalog`
- `schema`
- `inspect`
- `select`
- `slice`

The maintained surface uses those canonical command names directly. Clap also exposes the built-in
`help` subcommand for root and nested command help, but there are no extra documented aliases
layered on top of the maintained command set.

## Source Inputs

Every extraction and inspection command accepts one input:

- a local file path
- an `http://` or `https://` URL
- `-` for stdin
- `--input-html <HTML>` when the source bytes are already in memory

`--base-url` sets the input base explicitly. For URL inputs, the request URL is the input base
automatically. If the document contains `<base href>`, HTMLCut resolves it against the input base to
produce the effective base URL used by `--rewrite-urls`. When HTML-valued extraction is later
rendered as plain text, HTMLCut resolves displayed link destinations against that effective base,
including bundled `selection.txt` output, even when `--rewrite-urls` is off. `--rewrite-urls`
controls the saved HTML fragment itself.

For URL inputs, HTMLCut uses HEAD-first fetch preflight by default:

- `head-first` treats successful HEAD responses as advisory preflight for status,
  `Content-Length`, and obvious non-HTML `Content-Type` values, and it automatically falls back to
  GET only when the HEAD response rejects the method or the HEAD exchange fails in a way that
  indicates HEAD intolerance
- `get-only` skips the HEAD probe for servers that still mishandle HEAD badly

The CLI exposes that policy through `--fetch-preflight head-first|get-only`.

Timeout controls are explicit:

- `--fetch-connect-timeout-ms` bounds the TCP connect phase for URL inputs
- `--fetch-timeout-ms` bounds the overall HTTP exchange for URL inputs

TLS trust policy is explicit too:

- `--tls-trust web-pki` uses the bundled Mozilla root set
- `--tls-trust platform` uses the host verifier and root store
- `--tls-trust custom-ca-bundle --tls-ca-bundle <PATH>` uses one explicit PEM CA bundle

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
- `htmlcut catalog --output-file <PATH>` when you want that text or JSON payload written to disk

If `--output-file` points at an existing path, pass `--overwrite` or choose a fresh file.

In text mode, every operation prints the mapped core surface plus the request/result contracts
plus the parameter inventory, typed default overrides, and command constraints.

That text surface is contract-linted against the same core registries that back `--output json`.

### `schema`

`schema` is the validator-grade contract discovery surface.

Use:

- `htmlcut schema --output json` for the full registry
- `htmlcut schema --name <schema_name>` for one schema family
- `htmlcut schema --name <schema_name> --schema-version <n>` for one exact schema version
- `htmlcut schema --output-file <PATH>` when you want that text or JSON payload written to disk

If `--output-file` points at an existing path, pass `--overwrite` or choose a fresh file.

The registry includes:

- `htmlcut-core` request/result schemas
- `htmlcut-cli` report schemas
- `htmlcut-cli` error-report schema
- interop v1 schemas

### `inspect`

`inspect` is the pre-extraction workflow:

- `inspect source` summarizes document structure, suggests extraction and reading selectors, and reports
  base-URL behavior
- `inspect select` previews selector matches with the same value modes as final selector extraction
- `inspect slice` previews slice matches with the same value modes as final slice extraction

`inspect` defaults to JSON. Text mode is for compact human review.

Suggested selectors are heuristics, not a promise that one selector is universally best for every
output goal. `inspect source` now separates narrower extraction selectors from broader reading
selectors so HTML saving and rendered-text review do not have to share one compromise list. Use
`inspect select` to compare the top candidates before you commit to `outer-html` or `inner-html`.

`inspect source` carries source-analysis-specific controls:

- `--sample-limit` bounds the sampled extraction candidates, reading candidates, headings, links,
  tags, and classes
- `--include-source-text` includes the full source in JSON output and enables a bounded source
  preview in text output
- `--preview-chars` bounds that source preview in text mode

`inspect select` and `inspect slice` can load a reusable extraction-definition file through
`--request-file <PATH>` instead of spelling the source and strategy inline.

Those same four command surfaces accept `--emit-request-file <PATH>`, which writes the normalized
extraction-definition JSON used for that run. That makes it practical to prototype inline first,
then promote the exact normalized request into a reusable JSON file without manually rewriting the
contract. Emitted request files always carry explicit `schema_name`, `schema_version`, and
`request.spec_version` identity fields while omitting unchanged defaults. Replayable request files
accept only absolute HTTP(S) URLs without userinfo, query strings, or fragments.

When an extraction-definition file is missing, malformed, on an unsupported schema revision, or
uses the wrong extraction strategy for the command, the CLI points back to the maintained
`htmlcut schema --name htmlcut.extraction_definition --output json` contract and the matching
`htmlcut catalog --operation <id> --output json` entry instead of failing with pathless or
contract-free guidance.

Successful URL-backed inspection and extraction reports carry a structured load trace in the report
metadata. `inspect source --output text` prints that trace directly, and verbose stderr output for
inspection and extraction commands replays the same successful load steps for operators.

When the page is noisy, `inspect source` samples headings and link previews across the ranked
reading candidates instead of blindly sampling the first headings and anchors in DOM order or
depending on one candidate rank to be perfect. Extraction candidates bias toward cleaner saved HTML
roots, while reading candidates bias toward broader title-preserving wrappers.

CLI JSON reports carry a normalized `command` label. The maintained labels are:

- `catalog`
- `schema`
- `select`
- `slice`
- `inspect-source`
- `inspect-select`
- `inspect-slice`

Top-level JSON reports carry their own schema identity:

- `schema_name`
- `schema_version`

When JSON rendering is active and the CLI fails before it can emit a command-specific success or
failure report, it emits `htmlcut.error_report` instead. That error report carries:

- `exit_code`
- primary `error` category/code/message
- structured `diagnostics`
- `source_load_steps` when source loading reached a traced network stage before failing
- `schema_name`
- `schema_version`

The `code` values in that error report are stable strings from one of two maintained inventories:

- core `DiagnosticCode` values projected through the CLI
- CLI-specific `CliErrorCode` values such as parse, request-file, and bundle-write failures

### `select`

`select` extracts from CSS selector matches.

It can be driven two ways:

- inline flags such as `<INPUT>` or `--input-html <HTML>` plus `--css`
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

`--value text` uses HTML-aware text rendering rather than raw descendant concatenation. That means
heading boundaries, ordered-list numbering, nested-list indentation, image `alt` text, link
targets, table captions and rows, compact label-value rows, and preformatted whitespace on the
selected node are preserved in the extracted value. HTMLCut treats explicit selection as a strong
signal: the selected root itself is rendered unless it is actually hidden or empty, while reader-
cleanup heuristics remain available for descendants and whole-document review.
The default `--whitespace rendered` preserves that rendered layout after HTML-aware rendering; it
does not promise byte-for-byte source whitespace preservation.

`--value inner-html` and `--value outer-html` keep the selected fragment as HTML. Apart from
optional `--rewrite-urls`, HTMLCut does not sanitize or editorialize that fragment. URL rewriting
covers supported HTML URL-bearing attributes plus CSS `url(...)` and quoted `@import` references.
If the chosen selector includes widgets, scripts, related-topic blocks, or footer chrome, those
remain part of the saved HTML output by design.

When you pair either HTML value mode with `--output text`, HTMLCut renders that HTML fragment into
readable plain text instead of echoing raw markup. The same rule applies to bundled `selection.txt`
artifacts. When an effective base URL is known, rendered link destinations are resolved to
standalone absolute URLs so the text artifact remains portable outside the original document.

### `slice`

`slice` extracts between raw source boundaries.

Like `select`, it supports either inline source/boundary flags or `--request-file <PATH>`.

Boundary semantics are exact:

- literal matching is raw substring matching, not tag-aware
- literal slice requests do not expose regex flags in JSON contracts
- regex boundaries are consumed exactly as matched
- regex flags accept `i`, `m`, `s`, `U`, and `x`
- default selection excludes both matched boundaries
- `--boundary-retention` chooses one named boundary-retention mode
- `--value selected-html` returns the exact selected fragment after applying boundary retention
- `--value inner-html` returns the HTML between the two matched boundaries
- `--value outer-html` returns the full outer matched range including both boundaries

`selected-html` is the CLI spelling for the core-side `SelectedHtml` value kind.
Reusable extraction-definition JSON serializes slice boundaries under
`request.extraction.pattern` with `mode`, `from`, and `to`, and serializes the boundary policy
under `boundary_retention` with one of `exclude-both`, `include-start`, `include-end`, or
`include-both`.

Use `inspect slice` before committing to extraction whenever the boundary pattern may consume more
than you intended.

## Output Model

`select` and `slice` separate:

- extraction value: `--value`
- stdout rendering: `--output`
- reusable extraction-definition file: `--request-file`

Stdout modes:

- `text`
- `html`
- `json`
- `none`

Default stdout behavior:

- `--value text` and `--value attribute` default to `text`
- `select --value inner-html` and `select --value outer-html` default to `html`
- `slice --value selected-html`, `slice --value inner-html`, and `slice --value outer-html`
  default to `html`
- `--value structured` defaults to `json`
- `inspect` defaults to `json`
- `--max-bytes` accepts raw bytes or KiB/MiB/GiB values only when they resolve to a whole positive byte count after unit scaling

`--output none` is valid only with `--bundle`.

`--output-file <PATH>` writes exactly the stdout payload to one file without creating a bundle.
That works for text, HTML, JSON, and inspection text/JSON outputs. It is intentionally invalid with
`--output none` because there is no stdout payload to write.

`select --output html` is only valid with `--value inner-html` or `--value outer-html`.
`slice --output html` is valid with `--value selected-html`, `--value inner-html`, or
`--value outer-html`.
When an HTML value mode is paired with `--output text`, the CLI renders the extracted fragment
through HTMLCut's plain-text renderer instead of emitting raw markup.

When `--bundle`, `--output-file`, or `--emit-request-file` points into a directory tree that does
not exist yet, the CLI creates the parent directories automatically.

When any of those targets already exists, HTMLCut refuses to replace it until you pass
`--overwrite`.

With `--verbose`, successful `catalog`, `schema`, extraction, and inspection runs confirm
`--output-file` writes on stderr. Extraction and preview commands also confirm successful
`--emit-request-file` writes there.

## Bundle Workflow

`--bundle <dir>` writes:

- `selection.json`
- `selection.html`
- `selection.txt`
- `report.json`

`selection.json` is the canonical machine-readable payload for the selected matches. It keeps the
extracted value plus match metadata in one stable JSON artifact. `selection.html` is a wrapped
review artifact, not a byte-for-byte replay of the source document. `selection.txt` is the
rendered plain-text view of the same extracted matches, including when the underlying extracted
value is HTML. `report.json` is the lightweight execution summary for the bundle run; it points at
the same extraction outcome without duplicating the canonical value bodies that already live in the
selection artifacts.

If `<dir>` already exists, rerun with `--overwrite` only when you intend to replace the managed
bundle artifacts in place.

## Reusable Extraction-Definition Files

The CLI accepts first-class extraction-definition JSON files for:

- `select`
- `slice`
- `inspect select`
- `inspect slice`

Those files serialize the exact `ExtractionRequest` plus `RuntimeOptions` that the CLI would
otherwise build inline. Once `--request-file` is present, inline source and strategy flags are
rejected instead of being merged.

When an extraction-definition file is invalid, the CLI points recovery back to:

- `htmlcut schema --name htmlcut.extraction_definition --output json`
- `htmlcut catalog --operation <id> --output json`

Shape-mismatch failures call out the common footgun directly: selector and slice boundary fields
are serialized as plain JSON strings, not nested objects. The failure report includes the exact
JSON path that failed to deserialize.

For embeddable Rust callers, the matching core type is `htmlcut_core::ExtractionDefinition`.

## Failure Model

Human output modes print the primary failure to stderr.

JSON modes still exit non-zero on failure, but they emit structured JSON to stdout.

Unknown operation IDs and schema lookups suggest the nearest registered names or available
schema versions instead of failing with an unqualified miss.

Failed URL-backed operations preserve the same structured source-load trace that successful
runs expose. Human stderr output replays that trace, and JSON reports keep it under
`source.load_steps`.

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
- top-level `--version` prints the tool version plus engine identity, schema profile, and
  repository metadata

## Slice Preview Rule

`inspect slice --output text` shows:

- selected range data
- exact matched start and end boundary text
- selected text
- fragment context when it adds signal

That fragment line is the main debugging aid when a boundary pattern consumes the payload and leaves
the selected text empty, while the matched-boundary lines make the literal `<a` versus `<article>`
footgun obvious.
