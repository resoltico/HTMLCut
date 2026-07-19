---
afad: "4.0"
version: "12.0.0"
domain: INDEX
updated: "2026-07-16"
route:
  keywords: [docs index, documentation index, getting started, developer setup, devcontainer, architecture guide, workspace layout, cli guide, cli library guide, core guide, interop guide, release protocol, quality gates, artifact hygiene, tempdir helper]
  questions: ["where is the HTMLCut documentation index?", "where is the complete list of Markdown docs under docs/?", "where is the HTMLCut getting started guide?", "where are the HTMLCut maintainer docs?", "which doc explains the CLI surface?", "which doc explains the published htmlcut_cli crate?", "where is the workspace layout guide?", "where is the HTMLCut release protocol overview?", "where is the interop v1 guide?", "where is the HTMLCut artifact hygiene guide?", "where is the htmlcut_tempdir helper documented?", "where is the HTMLCut devcontainer workflow documented?"]
---

# Documentation Index

This file is the complete index of every maintained Markdown document under `docs/`.

HTMLCut keeps its maintained developer-facing and maintainer-facing documentation there. Use this
page as the directory, then follow the linked guides for the detailed contract or workflow.

The maintainer docs contract walks the maintained public Markdown set recursively, excluding
`changelog.md`, skipping every hidden directory, and also skipping generated/internal trees such as
`tmp/`, `target/`, and `semver-baseline/`.

Concrete fenced `htmlcut ...` examples are executed in a fixture-backed sandbox through the docs
contract. Public Rust fences in the maintained architecture/core/interop/schema guides are executed
through `htmlcut-core` doctest harnesses, so those examples fail the normal workspace doc-test gate
when they drift.

## Product Surfaces

- [Getting Started](getting-started.md)
- [Developer Setup](developer-setup.md)
- [Contributor Devcontainer](developer-devcontainer.md)
- [Architecture Guide](architecture.md)
- [Workspace Layout](workspace-layout.md)
- [CLI Developer Guide](cli.md)
- [CLI Library Guide](cli-library.md)
- [Core Developer Guide](core.md)
- [Schema Guide](schema.md)
- [Interop v1 Guide](interop-v1.md)
- [Operation Matrix](operations.md)
- [Platform Support](platform-support.md)

## Maintainer Workflow

- [Quality Gates](quality-gates.md)
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

The core crate also ships a runnable namespace example at
[crates/htmlcut-core/examples/request_and_result_namespaces.rs](../crates/htmlcut-core/examples/request_and_result_namespaces.rs).
Run `cargo run -q -p htmlcut-core --example request_and_result_namespaces` to print a compact JSON
summary that shows the `htmlcut_core::request` / `htmlcut_core::result` namespace split in action.

Reusable extraction-definition workflows are illustrated in
[crates/htmlcut-core/examples/reusable_extraction_definition.rs](../crates/htmlcut-core/examples/reusable_extraction_definition.rs).
Run `cargo run -q -p htmlcut-core --example reusable_extraction_definition` to print the reusable
JSON definition that the example round-trips before extraction.
