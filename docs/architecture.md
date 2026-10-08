# Architecture

HTMLCut is a strict saved/rendered UTF-8 HTML extractor: an immutable prepared DOM,
a reusable compiled query, owned typed results, and a native CLI that bounds JSON
encoding before publishing any bytes. Acquisition and rendering belong to callers.

## Design

Retain CSS selection, complete counts, one/all/nth, required/optional/many fields,
original context and sibling scope, exclusions, count/equality/regex guards, text,
literal, Markdown, HTML, attributes and explicit-base URL readings. Retire receipts,
identities, bundles and replay. Results retain only values and useful counts, allowing
the source and prepared DOM to be dropped independently. Typed core success proves
query completion; it does not prove JSON encoding or delivery.

Use aligned upstream parser/selector public boundaries if independently proven safe.
A bounded TreeSink admits allocations and attachments before mutation. A public
TokenSink guard refuses forwarding after sticky failure, including end/foreign
callbacks; UTF-8 chunks bound feed granularity. Unfinished lexical tokens can span
chunks and remain bounded by source bytes, not the DOM-node limit. Parser refusal
must never return a partial DOM or use panic as ordinary control flow.

A product-owned selector Element adapter charges inner navigation and predicates,
retains one cache for each immutable matching pass, and checks sticky exhaustion
after matching. Filtered serialization walks the original DOM without mutation.
Logical work limits apply at owned boundaries, not every upstream instruction;
none of these limits provides process CPU/RSS isolation.

## Skeptical review

TreeSink refusal alone is unsafe: the retained audit counterexample `<i></i>`
under elements=3 can leave adoption bookkeeping inconsistent. The token guard
only stops later tokens. An independently authored `<b><p>X</b>Y` with elements=5
panics inside adoption bookkeeping even behind that guard. Retained same-token stop
hooks now refuse before inconsistent bookkeeping. Templates, foster parenting,
adoption, reparenting, foreign content and unfinished tokens require direct
rejection controls. An outer candidate counter cannot bound one :has or nth walk.
Exhaustion inside a predicate must remain a resource error even if upstream
matching returns false or negation returns true. Cached and cold paths must have
identical complete results.

Upstream version numbers do not prove pointer safety. Strict-provenance Miri and
ownership controls decide whether servo_arc/tendril corrections can be removed.
Preserve the narrow necessary safeguards if their replacement fails; publication
and registry installability cannot be claimed while local safety carriers remain.

Delivery uses the existing bounded writer and atomic single-file staging. Encoding
must finish before staging or stdout. Failure before delivery emits no data; a
stdout write failure can expose a prefix, and atomic rename is not a multi-file
transaction. No replacement archive/manifest framework is needed. Direct Cargo
checks and focused native/package/release scripts replace xtask administration.
Retain four native targets, attribution, integrity, source binding, immutable
publication and anonymous downloaded-binary execution. Exact-source CI remains
an external prerequisite; local success cannot establish it.
