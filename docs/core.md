# Core contract

`SourceSnapshot` preserves caller-owned UTF-8 source, including BOM/CRLF/NUL,
and validated explicit base metadata. `PreparedDocument` lazily parses it once
with the maintained upstream `scraper` HTML5 DOM and caches preparation outcomes.
`CompiledPlan` validates and normalizes the closed version-8 plan once. Executions
reuse the immutable original DOM and receive fresh logical limits.
The [core package example](../crates/htmlcut-core/README.md) is checked as a doctest.

The core is an in-process API. Source admission precedes parsing, but DOM node and
depth limits are checked after upstream parsing. Logical traversal and result
limits cannot interrupt every upstream parser or selector operation. Callers
processing adversarial input must supply process isolation; only the CLI supplies
the supervised worker boundary. Successful core execution establishes declared
DOM predicates and complete typed results, separately from encoding or delivery.

## Selection, records and guards

Root `match` is `one` (exactly one), `all` (default min=1, optional max), or `nth`
(positive one-based index, complete candidate count). Fields also support
`optional`: zero returns null, one returns a string, more refuses. `all` fields
return arrays; explicit min=0 permits empty arrays. Required attributes missing on
matched elements always refuse, including optional fields. Empty present values
remain values. Records have 1–64 ASCII identifier keys of at most 64 bytes,
processed lexically; rows and many-valued strings remain in document order.

Each row is exactly one selected anchor subtree. Root and field matching include
their scope anchor. `:scope` refers to that original anchor; selector predicates
can inspect original surrounding context, while returned field nodes remain
inside the anchor. Rows are never cloned or reparsed. There are no sibling forests,
fallback selectors, nested records or transformations.

`expect` is conjunctive. Count bounds default to 1/1. Document scope is the default;
selected scope checks each selected original subtree. Whole-reading `equals` or
Rust regex-search `pattern` checks every matched node after complete counting.
Predicate readings are text/literal/attr (default text); count-only guards reject
a reading. Guards run before exclusions. Document guards run even when min=0
permits no roots; selected guards are vacuous over no roots. Any late field/guard
failure discards the entire result. These checks do not establish business truth.

## Readings

`text` is deterministic structural text, not browser visibility. It collapses
Unicode whitespace outside original HTML pre ancestry, retains parsed pre text,
adds block/break boundaries and omits HTML script/style/template and SVG
script/style payloads. Hidden and noscript content remains. Foreign elements do
not acquire HTML block roles. A directly selected non-inert template descendant
is readable.

`literal` concatenates parsed descendant text including inert/hidden content,
without adding separators. `outer-html` serializes the selected root with immutable
subtree exclusions; parser normalization means it is not source-byte cutting.
Attributes validate names and fail on absence. `url:NAME` supports href/src/action/
poster/cite/formaction/data in supported element positions. Relative URLs require
explicit absolute HTTP(S) base metadata without userinfo. HTML base elements and
acquisition origin are never inferred. URL inputs/base are capped at 8 KiB and
processing at 32 KiB, subject to value limits.

Markdown, resolved Markdown and inner HTML are removed. Use a maintained document
converter or reader for those jobs. Strings are lexical values; caller code owns
currency/date parsing and domain validation.

## Results, inspection and limits

`ExtractionResult::data()` borrows owned `Values` or `Records`; `into_data()`
consumes them. Candidate, selected and per-field counts are complete.
Both empty variants encode as `[]` while their typed distinction remains.
Results retain no source, query, DOM or encoded payload.

`inspect(select, samples)` requires an explicit selector and returns a complete
count and 1–10 bounded identifier/attribute-name/structural-text samples.
Completeness labels distinguish omissions; oversized identifiers are omitted
rather than shortened into misleading selector tokens. For diagnosis, extract
one representative row as outer HTML. No survey or inferred fields are supplied.

Source bytes, plan bytes, selector syntax/matching depth (64 active path units), prepared DOM size,
candidates, selected values, cells, value bytes and aggregate bytes have finite
limits defined authoritatively in `limits.rs`. DOM limits admit the parsed result,
not each upstream allocation. Work units cover owned traversal and value work,
not all upstream matcher callbacks or CPU instructions. Exhaustion is a refusal,
never a successful false non-match. Explicit count maxima are assumptions and
are never silently clamped. Each field slot costs a cell, including null/empty
arrays; each all-valued string costs another.

The CLI independently bounds serialized data to 64 MiB before LF and inspection
to 16 KiB, then publishes only a complete successful worker response. Core typed
results do not promise encoded size or OS resource containment. See
[architecture](architecture.md) for the worker policy and its precise guarantee.

Known regular-file stdin identities participate in source/plan output-alias refusal.
The CLI uses the already duplicated reader handle, including `--plan -`; it does not
infer origins for anonymous pipes. Callers keep destinations distinct from upstream
files when piping bytes from another process.
