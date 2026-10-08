# Retained dependency safeguards

The current six path packages carry parser/selector resource hooks and pointer
corrections through downstream git/path consumers. SHA-2 and archive machinery are
removed. A root-only Cargo patch would not carry safeguards to downstream consumers.

The aligned upstream experiment uses scraper 0.27.0, html5ever/markup5ever 0.39,
selectors 0.38, servo_arc 0.4.3 and tendril 0.5.1. Authored low-limit adoption,
template, foster-parenting and foreign-content controls initially passed the guarded public
TokenSink adapter. A separate `<b><p>X</b>Y` with elements=5 control then panicked
inside its current adoption token, before the public guard regained control.
The retained parser now stops at same-token reprocessing/adoption/mutation boundaries. Strict Miri failed in tendril 0.5.1 `header()` at its integer-to-
pointer cast. That graph has not met replacement proof. No upstream adoption or
registry installability is claimed. servo_arc also requires independent ownership
proof before removal. Run the targeted commands in [Contributing](../CONTRIBUTING.md).

Retained responsibilities: construction/attachment admission, same-token mutation
refusal and parser stopping; inner selector work/shared immutable caches; filtered
serialization; servo_arc tail/tag/borrow provenance; tendril heap-header provenance.
The public accounting contract is logical, not exact fork-instruction compatibility.
These carriers remain a specific unmet simplification requirement. Do not replace
this graph until both stopping and pointer proofs pass on the shipped implementation.

The servo_arc tail pointer is unconditional, so normal and Miri paths have identical
layout and ownership behavior. This costs one internal pointer per HeaderSlice;
header/tail Send/Sync bounds remain required. The direct tail mutation/lifetime/drop
control is included in its full six-test Miri library run.
