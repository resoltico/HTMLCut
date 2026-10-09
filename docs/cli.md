# CLI

Use `extract`, `inspect` and optional `schema NAME`; help lists exact flags.
`schema --help` derives all supported names from the core schema API; retrieving
schemas remains optional for ordinary extraction.
`extract` requires exactly one source (`--file FILE` or `--stdin`) and query
(`--select CSS`, `--plan FILE`, or `--plan-json JSON`). A query file `-` uses stdin;
source and query cannot share stdin. Only regular files are accepted. Input must
be UTF-8; BOM, CRLF and NUL remain exact in the snapshot.

Inline one/all/nth selection and named required/optional/many fields compile through
the same closed contract as JSON. `--expect-text CSS TEXT` checks original document
context. Use JSON for count/regex bounds and nth fields. See [full semantics](core.md).
`inspect --file FILE` surveys eligible groups of at least three direct HTML
siblings; it excludes non-HTML and head/script/style/template/pre/code/noscript
branches. An empty survey does not establish that the page has no records. For smaller groups,
`--select CSS --samples 2` counts all matches and emits bounded samples; extract
one row's outer HTML to author fields. Inspection contains no source digest.

JSON is compact, deterministic, at most 64 MiB without its single LF. `--raw`
requires one scalar and emits its exact UTF-8 bytes without LF. Encoding completes
before stdout or single-file staging. `--output FILE` uses atomic create;
`--overwrite` permits atomic replacement. Inputs/output must be distinct and
symlink/nonregular destinations are refused. Write/flush errors remain failures;
a pipe can receive a prefix before failure. Atomic files do not promise crash durability.

Failures are source-free JSON on stderr: code, stage, field/path/count/resource facts
and useful recovery causes. Exit classes: 2 configuration, 3 semantic refusal,
4 resource exhaustion, 5 I/O, 6 internal invariant. No query failure publishes partial
data. Receipts, bundles, replay and their schemas are unsupported.

File-publication causes identify `operation: publication` and portable problems
such as `not_found`, `permission_denied`, `already_exists` or `unsupported_kind`
when known. They expose no supplied path or OS error text. Missing query version
or select members use `missing_required` at fixed `$.version`/`$.select` paths;
null or wrong types use `invalid_value`, while present unsupported integer
versions use `unsupported_version`. An omitted command is `missing_required`.
