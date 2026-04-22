---
afad: "3.5"
version: "4.3.0"
domain: RELEASE
updated: "2026-04-22"
route:
  keywords: [release protocol, release overview, gh cli, primary checkout, release phases, semver baseline]
  questions: ["how is the HTMLCut release flow organized?", "which release doc covers GitHub publication?", "what invariants must hold for an HTMLCut release?"]
---

# Release Protocol

The HTMLCut release flow is driven by the GitHub CLI (`gh`). Every step that touches GitHub uses
`gh`, not the GitHub web UI.

Release choreography lives in this document set. Contract-versioning policy lives in
[versioning-policy.md](versioning-policy.md).

## Phase Map

- [Release Preflight](release-preflight.md) covers Step 0 through Step 4: tool access, checkout
  choice, local verification, release branch creation, PR creation, CI, and merge handoff.
- [Release Publishing](release-publishing.md) covers Step 5 through Step 9: tagging, workflow
  monitoring, release-object verification, and host-native package verification.
- [Release Closeout](release-closeout.md) covers Step 10 through Step 12: Dependabot hygiene,
  semver-baseline refresh, and primary-checkout reconciliation.

## Shared Release Invariants

- `Cargo.toml` `[workspace.package] version` is the single release-version source of truth.
- The local maintainer gate must pass before any release commit or tag.
- Release commits happen on a `release/X.Y.Z` branch, not directly on `main`.
- Tag publication is authoritative for release automation. PR merge alone does not publish.
- The GitHub release object and its asset inventory are the authoritative publication record.
- The checked-in semver baseline is refreshed only after the corresponding release is published.
- If a separate worktree is used, the primary checkout must still end the session truthful and
  synchronized with `origin/main`.

## Shared Inputs

- [Quality Gates](quality-gates.md) defines the maintained local gate.
- [Platform Support](platform-support.md) defines the release-target matrix and deployment floors.
- [Versioning Policy](versioning-policy.md) defines generic-contract versioning, frozen interop
  rules, and semver-baseline policy.
- `scripts/release-targets.sh`, `.github/workflows/ci.yml`, and `.github/workflows/release.yml`
  implement the published target matrix and release asset inventory.

## Working Rule

If a release fact cannot be verified from the worktree, the GitHub release object, or the current
checkout state, do not infer it from memory. Verify it directly in the relevant phase doc before
continuing.

If the primary checkout is dirty but already contains the intended release-candidate work, do not
pretend that dirty `main` is release-ready. Capture that state onto a named prep branch first,
then create the clean `release/X.Y.Z` worktree from the captured commit so the release branch
still has one truthful source commit chain.
