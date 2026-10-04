---
afad: "4.0"
version: "18.0.0"
domain: MAINTAINER
updated: "2026-10-04"
route:
  keywords: [versioning, extraction schema, semantics, semver baseline]
  questions: ["How are package, schema and semantics versions maintained?"]
---

# Versioning policy

The workspace package version in Cargo.toml is the sole release-version authority. Manifests, lockfiles, docs and native packages follow it. Stable publication uses an immutable annotated release tag after explicit authorization.

The current extraction wire family has schema version 3; extraction semantics independently have version 3. Incompatible wire shapes require a schema change, and selection/projection meaning changes require a semantics change. Packaging alone does not change extraction identity. Unknown versions/fields/enums and obsolete envelopes are rejected, without adapters or migration shims.

The frozen published 17.0.0 API baseline remains immutable during the intentional 18.0.0 major break. A baseline refresh comes only from an actual authorized immutable release, using the maintained mechanism. Never refresh from a worktree to hide an API change.

Numeric major increases permit major changes; minor increases permit minor; patch increases and equal versions enforce patch protection. Downgrades, malformed versions and prerelease/build metadata on stable publication are rejected. Future patch/minor releases cannot bypass protection through an unconditional major override.

See [Schemas](schema.md) and [Release Protocol](release-protocol.md).
