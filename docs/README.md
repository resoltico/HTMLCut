---
afad: "3.5"
version: "4.4.0"
domain: INDEX
updated: "2026-04-23"
route:
  keywords: [docs index, developer setup, architecture guide, cli guide, core guide, interop guide, release protocol, quality gates]
  questions: ["where are the HTMLCut maintainer docs?", "which doc explains the CLI surface?", "where is the HTMLCut release protocol overview?", "where is the interop v1 guide?"]
---

# Docs

HTMLCut keeps its maintained developer-facing and maintainer-facing documentation under `docs/`.

Use these documents as a system, not as isolated reference pages.

The maintainer docs contract walks the maintained public Markdown set recursively, excluding
`changelog.md`, skipping every hidden directory, and also skipping generated/internal trees such as
`tmp/`, `target/`, and `semver-baseline/`.

Concrete fenced `htmlcut ...` examples are executed in a fixture-backed sandbox through the docs
contract. Public Rust fences in the maintained architecture/core/interop/schema guides are executed
through `htmlcut-core` doctest harnesses, so those examples fail the normal workspace doc-test gate
when they drift.

## Product Surfaces

- [Developer Setup](developer-setup.md)
- [Architecture Guide](architecture.md)
- [CLI Developer Guide](cli.md)
- [Core Developer Guide](core.md)
- [Schema Guide](schema.md)
- [Interop v1 Guide](interop-v1.md)
- [Operation Matrix](operations.md)
- [Platform Support](platform-support.md)

## Maintainer Workflow

- [Quality Gates](quality-gates.md)
- [Release Protocol Overview](release-protocol.md)
- [Release Preflight](release-preflight.md)
- [Release Publishing](release-publishing.md)
- [Release Closeout](release-closeout.md)
- [Versioning Policy](versioning-policy.md)
- [Contributing Guide](../CONTRIBUTING.md)

## Adjacent Docs

- [Fuzz Target Inventory](../fuzz/README.md)
- [Patent Notes](../PATENTS.md)

The core crate also ships a runnable namespace example at
`crates/htmlcut-core/examples/request_and_result_namespaces.rs`.

Reusable extraction-definition workflows are illustrated in
`crates/htmlcut-core/examples/reusable_extraction_definition.rs`.
