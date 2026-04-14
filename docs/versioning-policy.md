---
afad: "3.5"
version: "4.0.1"
domain: MAINTAINER
updated: "2026-04-14"
route:
  keywords: [versioning policy, frozen interop, generic contracts, semver baseline, schema naming, interop_profile]
  questions: ["how does HTMLCut version generic contracts versus frozen interop profiles?", "when should the semver baseline be refreshed?", "what does interop_profile mean in HTMLCut documents?"]
---

# Versioning Policy

**Purpose**: Define how HTMLCut versions generic contracts, frozen interop profiles, release tags, and schema identities.
**Prerequisites**: [Schema Guide](schema.md), [Interop v1 Guide](interop-v1.md), and [Release Protocol](release-protocol.md).

## 1. Version Sources

HTMLCut keeps one release-version source of truth:

- `Cargo.toml` `[workspace.package] version`

That version feeds:

- both published crates
- `htmlcut --version`
- release tags of the form `vX.Y.Z`
- release asset names

Do not create parallel version sources in crate manifests, docs, scripts, or workflows.

## 2. Two Contract Classes

HTMLCut has two different compatibility models.

### 2.1 Generic versioned contracts

These are the normal HTMLCut surfaces:

- `htmlcut-core` request/result schemas
- `htmlcut-cli` report schemas
- catalog and schema registry surfaces
- stable embeddable core APIs outside frozen interop profiles

These surfaces are allowed to change aggressively when architecture quality requires it.
HTMLCut does not carry backwards-compatibility shims, aliases, or migration layers for generic
surfaces.

When a generic public contract changes:

- update the Rust types
- update the JSON schema version where the serialized contract changed
- update catalog/schema docs and user-facing docs in the same change
- update tests so they assert the new contract explicitly
- document the released effect in `changelog.md`

### 2.2 Frozen interop profiles

Frozen interop profiles are different.

Current frozen profile:

- module: `htmlcut_core::interop::v1`
- profile string: `htmlcut-v1`
- schemas: `htmlcut.plan@1`, `htmlcut.result@1`, `htmlcut.error@1`

Once a frozen profile is released, do not mutate it in place.

If a downstream integration needs new capability:

- add a new interop module
- add a new profile string
- add the new plan/result/error documents
- add a new acceptance-fixture corpus

Do not retrofit the new behavior into `htmlcut-v1`.

## 3. Schema Naming Rules

Generic schema families use stable document names plus explicit integer schema versions.

Examples:

- `htmlcut.extraction_request@4`
- `htmlcut.extraction_result@4`
- `htmlcut.catalog_report@4`

Rules:

- keep schema names generic and product-owned
- do not encode downstream consumer names into generic schema families
- keep `schema_name` and `schema_version` on every maintained public JSON document
- use the schema registry as the validator surface, not prose examples

Frozen interop schemas also use product-owned names:

- `htmlcut.plan`
- `htmlcut.result`
- `htmlcut.error`

Their downstream routing identity is completed by `interop_profile`, not by consumer-branded
schema names.

`HtmlInput` is intentionally excluded from the JSON schema registry because it is a Rust-only
in-process handoff type.

## 4. `interop_profile` Routing

`interop_profile` is part of the frozen interop contract surface.

Maintainer expectations:

- every frozen interop document must carry the expected `interop_profile`
- validators must reject mismatched profile values
- downstream routing must use `interop_profile` together with `schema_name` and `schema_version`
- the module path, profile string, schema set, fixture directory, and acceptance tests must stay aligned

For `htmlcut-v1`, that alignment is:

- module: `htmlcut_core::interop::v1`
- fixtures: `crates/htmlcut-core/tests/fixtures/htmlcut-v1/`
- acceptance runner: `crates/htmlcut-core/tests/v1_acceptance.rs`

## 5. Release-Time Expectations

Release preparation is expected to converge the entire shipped contract, not just bump a version.

Before a release is tagged:

- the workspace version is correct
- changelog, README, and maintained docs describe the same shipped surface
- schema registry output matches the docs
- the maintainer gate passes

HTMLCut optimizes for a coherent released system, not for preserving obsolete shapes.
If a generic contract needs a breaking redesign before the next public release, replace it cleanly
and ship the new contract as the next release line.

## 6. Semver Baseline Policy

The checked-in semver baseline represents the last published `htmlcut-core` API, not the current
worktree.

Rules:

- refresh it only after the corresponding release is actually published
- refresh it from an explicit published Git ref with `cargo xtask refresh-semver-baseline --git-ref vX.Y.Z`
- never regenerate it from unreleased local worktree state
- treat it as the comparison target for future semver checks, not as a staging area during feature work

The baseline exists to keep published compatibility accounting honest. It must not drift ahead of
what users can actually depend on.
