<!--
AFAD:
  afad: "4.0"
  version: "19.1.0"
  domain: DEPENDENCY
  updated: "2026-10-05"
RETRIEVAL_HINTS:
  keywords: [local dependency patch, vendored dependency stack, scraper, selectors, html5ever, markup5ever, servo_arc, tendril, miri, strict provenance]
  questions: ["why does HTMLCut vendor the selector and parser stack locally?", "how do I verify the local dependency patches?", "when can the local overrides be removed?"]
-->

# Local Dependency Patches

This repository carries paired runtime forks for enforced parser/selector budgets, filtered
immutable serialization and strict-provenance corrections. Git/path consumers receive the same
safeguards through explicit owned path packages. Refresh upstream sources while retaining the
applicable hooks; removal requires proof that the replacement provides those guarantees.

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

- Source: crates.io `servo_arc` `0.5.0`
- Scope: pointer-provenance fixes on the selector stack used by `scraper` and `htmlcut-core`
- Reason: selector preparation historically exposed a Miri provenance failure through
  `scraper -> selectors -> servo_arc`; bounded DOM extraction retains the correction. Source
  slicing does not prepare a DOM or perform title lookup in v15.
- Current state: the local patch preserves tail provenance through `HeaderSlice` construction and
  drop

## `rust/tendril`

- Source: crates.io `tendril` `0.5.1`
- Scope: strict-provenance fixes on the HTML parser stack used by `markup5ever`, `html5ever`,
  `scraper`, and `htmlcut-core`
- Reason: DOM parsing historically exposed a strict-provenance failure through
  `scraper -> html5ever -> markup5ever -> tendril`; parsing and source slicing are separate.
- Current state: the local patch preserves heap-header provenance separately from the tagged pointer
  bits, with the previous revision verified under strict provenance; the refreshed sources await
  their release Miri proof

## Upstream refresh and verification status

The 1 October refresh uses scraper 0.27.0, selectors 0.41.0, servo_arc 0.5.0,
html5ever 0.40.1, markup5ever 0.40.0 and tendril 0.5.1. Scraper's owned revision advances
to carry the new selector error API and paired stack; owned version suffixes distinguish every
modified carrier from its upstream release. Selector errors no longer borrow discarded token
payloads, while qualified-name diagnostic locations remain explicit. SHA-2 stays on the current
0.11.0 release with its ARM64 fix and refreshed dependency minima.

The previous clean source passed Miri, fuzz and native verification. At the refresh handoff, gates were deferred at the user's request;
release verification now validates the changed sources. See [dependency refresh](../docs/dependency-refresh.md).

## Verification

- `cargo xtask miri`

Only after a registry stack proves the same bounded/provenance/serialization guarantees, restore the registry-backed `scraper` dependency in
[Cargo.toml](../Cargo.toml), remove the vendored `htmlcut-*` path packages under `patches/rust/`,
and confirm that `cargo xtask miri` still passes.

## `rust/sha2`

The direct `htmlcut-sha2` dependency retains RustCrypto SHA-2 0.11.0 and its MIT/Apache licensing. Eight ARM64 SHA-256 NEON constant-load pointers originate from complete four-u32 slices, correcting the borrow-range violation previously detected during snapshot hashing. Algorithm, hardware backend and Miri flags are unchanged. The earlier verified revision passed independent identity vectors and Miri; the refreshed compiler/dependency configuration awaits release verification. See [patch provenance](rust/sha2/HTMLCUT-PATCH.md).


The owned servo_arc tagged-union module preserves allocation provenance when setting and clearing its pointer tag. Its constructors, borrow/clone/drop and identity/value behavior are covered by direct ownership tests and strict-provenance Miri; application unsafe-code prohibitions are unchanged.

The current extraction CLI has no HTTP/charset acquisition stack. The parser no longer builds a document-order element index for retired cursor discovery; targeted inspection traverses the immutable DOM under fresh bounds. Current-source release verification must cover the changed parser and every retained provenance/resource correction.
