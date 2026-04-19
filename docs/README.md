<!--
AFAD:
  afad: "3.5"
  version: "4.1.0"
  domain: INDEX
  updated: "2026-04-19"
RETRIEVAL_HINTS:
  keywords: [docs index, developer setup, architecture guide, interop v1 guide, release protocol, quality gates, versioning policy, contributing, fuzz inventory]
  answers: [where are the HTMLCut maintainer docs?, where is the HTMLCut developer setup guide?, which doc explains htmlcut interop v1?, where is the release protocol?, where is the HTMLCut versioning policy?]
  related: [docs/developer-setup.md, docs/architecture.md, docs/cli.md, docs/core.md, docs/schema.md, docs/interop-v1.md, docs/versioning-policy.md, ../CONTRIBUTING.md, ../crates/htmlcut-core/examples/request_and_result_namespaces.rs, ../crates/htmlcut-core/examples/reusable_extraction_definition.rs, ../fuzz/README.md]
-->

# Docs

HTMLCut keeps its maintained developer-facing and maintainer-facing documentation under `docs/`.

Use these documents as a system, not as isolated reference pages:

- [Developer Setup](developer-setup.md)
- [Architecture Guide](architecture.md)
- [CLI Developer Guide](cli.md)
- [Core Developer Guide](core.md)
- [Schema Guide](schema.md)
- [Interop v1 Guide](interop-v1.md)
- [Versioning Policy](versioning-policy.md)
- [Operation Matrix](operations.md)
- [Platform Support](platform-support.md)
- [Quality Gates](quality-gates.md)
- [Release Protocol](release-protocol.md)
- [Contributing Guide](../CONTRIBUTING.md)
- [Fuzz Target Inventory](../fuzz/README.md)

The core crate also ships a runnable namespace example at
`crates/htmlcut-core/examples/request_and_result_namespaces.rs`.

Reusable request-file workflows are illustrated in
`crates/htmlcut-core/examples/reusable_extraction_definition.rs`.
