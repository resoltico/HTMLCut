# CLI

Use `extract`, `inspect` and optional `schema NAME`. Schema help lists supported
canonical names from the core API; ordinary extraction needs no schema retrieval.

`extract` requires exactly one source (`--file FILE` or `--stdin`) and query
(`--select CSS`, `--plan FILE` or `--plan-json JSON`). Query file `-` consumes stdin;
source and query cannot share stdin. Named inputs must resolve to regular files;
they are opened and read once with bounded admission. UTF-8 is required; snapshots
preserve accepted BOM/CRLF/NUL. Acquisition/rendering happens outside HTMLCut.

Inline one/all/nth and required/optional/many field flags compile through the same
version-8 contract as JSON. `--expect-text CSS TEXT` checks document context;
use JSON for count/regex guards, field nth and plural bounds.
Readings are text/literal/outer-html/attr:NAME/url:NAME.
`inspect --file FILE --select CSS --samples 2` counts completely and returns bounded
samples. Extract one row's outer HTML to inspect its structure. Unscoped survey,
sibling forests and document conversion readings are removed without aliases.

The private worker applies native resource policy before parsing, and the parent
buffers bounded channels under a ten-second active-worker deadline. Waiting for caller-owned source input
is excluded; no source bytes are parsed during that wait. Only a verified complete response
and successful child termination permit publication. No child panic or raw child
diagnostic is forwarded. See [architecture](architecture.md) for guarantees.

Successful JSON is a deterministic compact bare array, at most 64 MiB before one LF.
`--raw` requires exactly one scalar and writes UTF-8 without LF. `--output FILE`
atomically creates a file; `--overwrite` permits replacement. Query, worker,
encoding and staging failures leave an existing destination intact. Input/output
aliases and symlink/nonregular destinations are refused. Stdout write/flush errors
remain failures; a downstream pipe can receive a prefix after publication begins.
Atomic files do not promise crash durability.

Failures are bounded source-free JSON on stderr: code, stage and safe field/path/
count/resource facts. Exit classes are 2 configuration, 3 semantic refusal,
4 resource exhaustion, 5 I/O and 6 internal invariant. Supplied paths, source values
and OS error strings are excluded. File errors retain portable operation/problem
facts such as not_found, permission_denied, already_exists and unsupported_kind.
Missing plan version/select uses missing_required; null/wrong types uses invalid_value;
unsupported integer versions use unsupported_version. Old plans are rejected.

Known regular-file stdin identities participate in source/plan output-alias refusal.
The CLI uses the already duplicated reader handle, including `--plan -`; it does not
infer origins for anonymous pipes. Callers keep destinations distinct from upstream
files when piping bytes from another process.
