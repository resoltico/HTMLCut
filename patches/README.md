<!--
AFAD:
  afad: "4.0"
  version: "12.0.0"
  domain: DEPENDENCY
  updated: "2026-07-16"
RETRIEVAL_HINTS:
  keywords: [local dependency patch, vendored dependency stack, scraper, selectors, html5ever, markup5ever, servo_arc, tendril, miri, strict provenance]
  questions: ["why does HTMLCut vendor the selector and parser stack locally?", "how do I verify the local dependency patches?", "when can the local overrides be removed?"]
-->

# Local Dependency Patches

This repository carries a vendored selector/parser stack only when a blocking defect has no
published upstream release that resolves it yet.

## Downstream-Safe Stack Carriers

HTMLCut no longer relies on root-only `[patch.crates-io]` entries for this safety line, because
downstream git consumers do not inherit those root patches. The workspace therefore carries
repo-owned local copies of these crates so `htmlcut-core` exports the fixed stack in its own
dependency graph:

- `rust/scraper`
- `rust/selectors`
- `rust/html5ever`
- `rust/markup5ever`

Those vendored manifests route downstream consumers onto the patched `servo_arc` and `tendril`
sources below. The source-level strict-provenance fixes themselves still live in those two crates.
HTMLCut intentionally ships only the runtime subset of that stack: upstream-only bench,
shared-memory, and Gecko refcount-logging feature surfaces stay trimmed so the maintained
`--all-features` and doctest gates prove the same contract that downstream consumers receive.

## `rust/servo_arc`

- Source: crates.io `servo_arc` `0.4.3`
- Scope: pointer-provenance fixes on the selector stack used by `scraper` and `htmlcut-core`
- Reason: the maintained selector-validation path, plus the document-title lookup reached from
  delimiter slice extraction, trips a Miri provenance failure through
  `scraper -> selectors -> servo_arc`
- Current state: the local patch preserves tail provenance through `HeaderSlice` construction and
  drop

## `rust/tendril`

- Source: crates.io `tendril` `0.5.0`
- Scope: strict-provenance fixes on the HTML parser stack used by `markup5ever`, `html5ever`,
  `scraper`, and `htmlcut-core`
- Reason: the maintained delimiter-slice execution path trips a strict-provenance Miri failure
  through `scraper -> html5ever -> markup5ever -> tendril`
- Current state: the local patch preserves heap-header provenance separately from the tagged pointer
  bits, so HTMLCut's maintained selector-and-slice Miri proof now passes under strict provenance

## Verification

- `cargo xtask miri`

When upstream publishes a clean stack, restore the registry-backed `scraper` dependency in
[Cargo.toml](../Cargo.toml), remove the vendored `htmlcut-*` path packages under `patches/rust/`,
and confirm that `cargo xtask miri` still passes.
