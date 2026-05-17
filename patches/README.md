<!--
AFAD:
  afad: "4.0"
  version: "10.0.0"
  domain: DEPENDENCY
  updated: "2026-05-16"
RETRIEVAL_HINTS:
  keywords: [local dependency patch, servo_arc, tendril, miri, selector safety, strict provenance]
  questions: ["why does HTMLCut vendor selector-stack crates locally?", "how do I verify the local dependency patches?", "when can the local overrides be removed?"]
-->

# Local Dependency Patches

This repository carries focused dependency patches only when a blocking defect has no published
upstream release that resolves it yet.

## `rust/servo_arc`

- Source: crates.io `servo_arc` `0.4.3`
- Scope: pointer-provenance fixes on the selector stack used by `scraper` and `htmlcut-core`
- Reason: the maintained selector-validation and selector-execution path trips a Miri provenance
  failure through `scraper -> selectors -> servo_arc`
- Current state: the local patch preserves tail provenance through `HeaderSlice` construction and
  drop

## `rust/tendril`

- Source: crates.io `tendril` `0.5.0`
- Scope: strict-provenance fixes on the HTML parser stack used by `markup5ever`, `html5ever`,
  `scraper`, and `htmlcut-core`
- Reason: the maintained selector-validation and selector-execution path trips a strict-provenance
  Miri failure through `scraper -> html5ever -> markup5ever -> tendril`
- Current state: the local patch preserves heap-header provenance separately from the tagged pointer
  bits, so HTMLCut's maintained selector-safety Miri proof now passes under strict provenance

## Verification

- `cargo xtask miri`

When upstream publishes a clean fix, replace this patch with the released dependency and remove the
local override in [Cargo.toml](../Cargo.toml).
