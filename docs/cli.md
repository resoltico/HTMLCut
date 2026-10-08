# CLI

Use `extract`, `inspect` and optional `schema NAME`; help lists exact flags.
`extract` requires exactly one source (`--file FILE` or `--stdin`) and query
(`--select CSS`, `--plan FILE`, or `--plan-json JSON`). A query file `-` uses stdin;
source and query cannot share stdin. Only regular files are accepted. Input must
be UTF-8; BOM, CRLF and NUL remain exact in the snapshot.

Inline one/all/nth selection and named required/optional/many fields compile through
the same closed contract as JSON. `--expect-text CSS TEXT` checks original document
context. Use JSON for count/regex bounds and nth fields. See [full semantics](core.md).
`inspect --file FILE` surveys repeated groups; `--select CSS --samples 3` counts all
matches and emits bounded samples. Inspection contains no source digest.

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
