---
afad: "4.0"
version: "19.2.0"
domain: INDEX
updated: "2026-10-05"
route:
  keywords: [documentation index, extraction contract, maintainer guides]
  questions: ["Where is the maintained documentation index?"]
---

# Documentation Index

This file is the complete index of every maintained Markdown document under `docs/`.

HTMLCut keeps its maintained developer-facing and maintainer-facing documentation there. Use this
page as the directory, then follow the linked guides for the detailed contract or workflow.

The maintainer docs contract walks the maintained public Markdown set recursively, excluding
`changelog.md`, skipping every hidden directory, and also skipping generated/internal trees such as
`tmp/`, `target/`, and `semver-baseline/`.

Concrete fenced `htmlcut ...` examples are executed in a fixture-backed sandbox through the docs
contract. Public Rust fences in the maintained architecture/core/schema guides are executed
through `htmlcut-core` doctest harnesses, so those examples fail the normal workspace doc-test gate
when they drift.

## Product Surfaces

- [Getting Started](getting-started.md)
- [Developer Setup](developer-setup.md)
- [Contributor Devcontainer](developer-devcontainer.md)
- [Architecture Guide](architecture.md)
- [Workspace Layout](workspace-layout.md)
- [CLI Developer Guide](cli.md)
- [Core Developer Guide](core.md)
- [Schema Guide](schema.md)
- [Operation Matrix](operations.md)
- [Platform Support](platform-support.md)

## Maintainer Workflow

- [Quality Gates](quality-gates.md)
- [Dependency Refresh](dependency-refresh.md)
- [Configuration and Workflow Audit](configuration-audit.md)
- [Artifact Hygiene](hygiene.md)
- [Release Protocol Overview](release-protocol.md)
- [Release Preflight](release-preflight.md)
- [Release Publishing](release-publishing.md)
- [Release Closeout](release-closeout.md)
- [Versioning Policy](versioning-policy.md)
- [Contributing Guide](../CONTRIBUTING.md)

## Internal Helpers

- [Tempdir Helper Guide](tempdir.md)

## Adjacent Docs

- [Fuzz Target Inventory](../fuzz/README.md)
- [Patent Notes](../PATENTS.md)

The core crate also ships a runnable snapshot-reuse example at
[crates/htmlcut-core/examples/snapshot_reuse.rs](../crates/htmlcut-core/examples/snapshot_reuse.rs).
Run `cargo run -q -p htmlcut-core --example snapshot_reuse` to print compact JSON
results that reuse one prepared snapshot.

Reusable extraction-definition workflows are illustrated in
[crates/htmlcut-core/examples/extraction_plan.rs](../crates/htmlcut-core/examples/extraction_plan.rs).
Run `cargo run -q -p htmlcut-core --example extraction_plan` to print the reusable
JSON definition that the example round-trips before extraction.

- [Extraction contract implementation authority](extraction-contract-spec.md)
- [Extraction contract implementation status](extraction-contract-status.md)
