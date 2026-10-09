# Retained dependency safeguards

HTMLCut owns five distributable dependency carriers. The parser/atom graph uses
html5ever 0.40.1 and markup5ever 0.40.0; the selector graph uses selectors 0.41.0.
Explicit dependency identities carry necessary safeguards into downstream path,
Git and packaged consumers. A consumer does not inherit root-only Cargo patches.
The carrier versions and dependency edges are authoritative in their manifests.

| Responsibility | Implemented boundary | Reason a carrier remains |
| --- | --- | --- |
| Bounded DOM allocation, attachment, depth and reparenting | Product `crates/htmlcut-core/src/dom/parser.rs` and `dom/tree_sink.rs` | No scraper carrier; the product owns admission and its private DOM. |
| Sticky current-token parser termination | html5ever driver/tokenizer/tree builder and markup5ever TreeSink | Registry html5ever 0.40.1 panics on `<b><p>X</b>Y` at elements=5 during adoption, before a public TokenSink guard regains control. |
| Atom and TreeSink type identities | markup5ever | Carries the stop-request trait and the parser's corrected tendril dependency. |
| Inner navigation/predicate work, shared pass caches | Product `dom/selector.rs` and `budget.rs` | The carrier retains iterative relative traversal (supported-depth stack overflow), removes diagnostic nth recomputation (panic after refusal), and routes its transitive Arc to the demonstrated correction. Product code owns accounting. |
| Filtered immutable serialization | Product `dom/serialization.rs` | No scraper mutation or serialization helpers remain. |
| Heap/shared tendril ownership | tendril | Registry tendril 0.5.1 fails strict-provenance Miri on integer-to-pointer reconstruction. |
| Arc tail, tagged variant and borrowed ownership | servo_arc | Registry servo_arc 0.5.0 fails the independent tail write, borrowed clone and tagged-variant controls. |

The parser delta stops token reprocessing and adoption before a refused allocation
sentinel can enter formatting bookkeeping. The tokenizer stops further processing,
and finish returns the sink's typed failure. A partial DOM never becomes a product
success. Guarding only the next token does not establish this property.

Tendril stores one provenance-preserving tagged `NonNull` pointer. Inline tags have
no referent and are never dereferenced; heap tags use strict pointer-address APIs.
This avoids duplicated tag/header state and retains the upstream 16-byte
representation on 64-bit platforms. Cloning, growth, shared substrings, clearing
and dropping retain their independent ownership controls.

The servo_arc tail pointer is unconditional: normal and Miri builds use the same
layout and ownership behavior. It costs one internal pointer per HeaderSlice.
Header/tail Send/Sync bounds remain required. ArcBorrow preserves the allocation
pointer; ArcUnion tagging preserves its provenance. Unconditionally disabled Gecko
logging bodies have been removed from this owned package.

Strict-provenance Miri rejects the selected upstream tendril integer casts and
servo_arc tagged casts as unsupported operations; that alone is not proof of
undefined behavior. The servo_arc tail/borrow controls produce Stacked Borrows
rejections, whose model remains experimental. These exact failed replacement
proofs justify retaining corrections, without claiming exploitability.

Run the targeted commands in [Contributing](../CONTRIBUTING.md). Full upstream
adoption remains unmet while these carrier responsibilities persist. Preparing
and verifying a dependency-complete distribution does not close that goal or
establish crates.io ownership/publication. The operator handoff in
[release protocol](../docs/release-protocol.md) records those separate prerequisites.
