# Architecture

HTMLCut accepts saved/rendered UTF-8 HTML, prepares one immutable private DOM,
compiles reusable closed queries, and returns complete owned typed values/counts.
Acquisition, rendering, source trust and business interpretation belong to callers.
Results hold no source, DOM, query, residual work or eager encoded payload. Core
success establishes query completion; the CLI separately bounds JSON before output.

## Implemented boundaries

`dom/parser.rs` admits element/node allocations and attachments before mutation;
`dom/tree_sink.rs` supplies the public html5ever TreeSink operations over ego-tree.
UTF-8 feeds are at most 4096 bytes, and failures discard the partial document.
Unfinished text/comment/raw tokens can span feeds and are bounded by source bytes.
Same-token refusal still requires narrow html5ever/markup5ever stop hooks.

`dom/selector.rs` supplies static CSS grammar and a budget-carrying public Element
adapter. Navigation charges every visited node, including skipped non-elements;
name/type/attribute/class predicates charge callbacks and inspected byte batches.
A matcher holds one SelectorCaches for its immutable document/selector/scope pass.
It checks sticky exhaustion after matching, so negation cannot turn refusal into a
successful non-match or match. No selector-engine internal budget hooks remain.

`dom/serialization.rs` streams filtered immutable HTML through the upstream public
Serialize trait. The private DOM has no clone/filter/mutation-helper product API.
Core callers retain original context for row scopes, guards and projections.
Logical limits apply at declared boundaries; they are not exact upstream instruction
counts or process CPU/RSS isolation. See [contract](core.md).

The CLI bounds complete deterministic JSON before single-file atomic staging or
stdout. A query/encoding failure publishes no data; a stdout write failure can
expose a prefix. Atomic rename is not a multi-file transaction. Reproduction uses
[ordinary files](operations.md); receipts/bundles/replay are removed.

## Design and separate skeptical challenge

Separate DOM admission, immutable serialization and matching/accounting from the
parser/pointer graph. Use one aligned html5ever 0.40 / selectors 0.41 type graph.
Moving these product responsibilities permits scraper retirement without deleting
its required behavior or replacing mature parsing with a handwritten parser.

A guarded public TokenSink cannot interrupt the current token: `<b><p>X</b>Y` at
five elements panics in upstream adoption bookkeeping. Keep the demonstrated
same-token stop hooks. Registry tendril/servo_arc also fail their independent
ownership/provenance controls; preserve corrected transitive package identities.

Challenge matching independently of those parser failures. The upstream nth-cache
debug recomputation panics after a public navigation callback refuses, and recursive
relative traversal overflows at supported depth 4000. Retain only iterative relative
traversal and removal of that redundant diagnostic recomputation, with independent
cached/cold, scope, ancestor/sibling/negation, depth and refusal controls. The wrapper
owns all work and pass caches; outer candidate counts alone cannot bound inner work.

Five necessary carrier responsibilities and their specific blockers are documented
in [dependencies](dependencies.md). Verified distribution of corrected packages is
separate from full upstream adoption and actual crates.io publication. Local and
required CI commands remain direct tools, with bounded smoke and focused native
package/release controls. Authoritative CI and external rollout remain separate
from locally executed evidence; see [release handoff](release-protocol.md).
