---
afad: "4.0"
version: "19.0.0"
domain: MAINTAINER
updated: "2026-10-05"
route:
  keywords: [versioning, extraction schema, semantics, semver baseline]
  questions: ["How are package, schema and semantics versions maintained?"]
---

# Versioning policy

The workspace package version in Cargo.toml is the sole release-version authority. Manifests, lockfiles, docs and native packages follow it. Stable publication uses an immutable annotated release tag after explicit authorization.

The current extraction wire family has schema version 5; extraction semantics independently have version 5. Incompatible wire shapes require a schema change, and selection/projection meaning changes require a semantics change. Packaging alone does not change extraction identity. Unknown versions/fields/enums and obsolete envelopes are rejected, without adapters or migration shims.

The checked-in API baseline comes from the published 19.0.0 tag. Future refreshes come only from an actual immutable release, using the maintained mechanism. Never refresh from a worktree to hide an API change.

Numeric major increases permit major changes; minor increases permit minor; patch increases and equal versions enforce patch protection. Downgrades, malformed versions and prerelease/build metadata on stable publication are rejected. Future patch/minor releases cannot bypass protection through an unconditional major override.

See [Schemas](schema.md) and [Release Protocol](release-protocol.md).
