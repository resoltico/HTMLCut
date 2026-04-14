---
afad: "3.5"
version: "4.0.0"
domain: RELEASE
updated: "2026-04-14"
route:
  keywords: [release protocol, gh cli, tag push, release workflow, semver baseline, verification]
  questions: ["how do I release HTMLCut?", "what must be verified before tagging a release?", "when do I refresh the HTMLCut semver baseline?"]
---

# Release Protocol

The release flow is driven by the GitHub CLI (`gh`). Every step that touches GitHub uses `gh`,
not the GitHub web UI.

Release choreography lives here. Contract-versioning policy lives in
[versioning-policy.md](versioning-policy.md).

## 0. GitHub CLI gate

Before doing anything else:

```bash
gh --version
gh auth status
```

If either command fails, stop immediately.

## 1. Pre-flight

Install the local maintainer toolchain if it is not already available:

```bash
rustup toolchain install stable --profile minimal
rustup toolchain install nightly --profile minimal --component llvm-tools-preview
cargo install cargo-nextest cargo-audit cargo-deny cargo-semver-checks cargo-outdated cargo-llvm-cov --locked
```

Stable remains the default HTMLCut toolchain. Nightly is installed alongside it only for the
coverage gate, because `cargo +nightly llvm-cov --branch` is currently required for true branch
coverage.

Install `shellcheck` from your system package manager, for example:

```bash
brew install shellcheck
```

Run the single local quality gate first:

```bash
cargo xtask check
```

That gate must succeed before any release commit or tag. The maintained definition of that gate
lives in [quality-gates.md](quality-gates.md).

The gate now also verifies that the checked-in fuzz targets still compile and that the nightly
coverage toolchain prerequisites are present before coverage work begins.

Then verify:

- `Cargo.toml` `[workspace.package] version` equals the target release version exactly. This is the single version source of truth for both crates and for `htmlcut --version`.
- `Cargo.toml` `[workspace.package] description` still reflects the current product in task-facing language. `htmlcut-cli` inherits it for CLI help and for the second line of `htmlcut --version`.
- `docs/operations.md` still reflects the current canonical operation catalog exposed by `htmlcut-core`.
- `changelog.md` has a `## [X.Y.Z] - YYYY-MM-DD` section with at least one entry.
- `README.md` still documents the current user-facing install flow, CLI model, and release assets.
- `CONTRIBUTING.md` still matches the maintained contributor workflow, fixture-update flow, and release expectations.
- `docs/README.md` still points at the maintained developer and maintainer docs.
- `docs/versioning-policy.md` still matches the shipped contract policy, frozen interop model, and semver-baseline rules.
- `docs/cli.md`, `docs/core.md`, `docs/schema.md`, and `docs/interop-v1.md` still match the shipped surfaces.
- `docs/platform-support.md` still matches the shipped release target matrix and deployment floors.
- `docs/quality-gates.md` still matches the maintained `cargo xtask` gate.
- `Cargo.toml` still defines the `dist` Cargo profile used for shipped public binaries.
- `README.md` still documents the release asset names:
  - `htmlcut-X.Y.Z.zip`
  - `htmlcut-X.Y.Z.tar.gz`
  - `htmlcut-aarch64-apple-darwin`
  - `htmlcut-aarch64-apple-darwin.sha256`
  - `htmlcut-x86_64-apple-darwin`
  - `htmlcut-x86_64-apple-darwin.sha256`
  - `htmlcut-x86_64-unknown-linux-musl`
  - `htmlcut-x86_64-unknown-linux-musl.sha256`
  - `htmlcut-x86_64-pc-windows-msvc.exe`
  - `htmlcut-x86_64-pc-windows-msvc.exe.sha256`
- repository settings are still aligned with this protocol:
  - default branch is `main`
  - `delete_branch_on_merge` is enabled
  - `main` is protected
  - `main` does not require approving reviews
  - `main` does not enforce branch protection for admins
  - required status checks are exactly:
    - `Check`

The intended release architecture is a single-owner, CI-gated repository. Required review on
`main` adds a non-existent human dependency and is therefore release-hostile technical debt.

## 2. Release branch

Do release commits on a release branch, not directly on `main`.

```bash
git checkout -b release/X.Y.Z
git add <every intended release file>
git status --short
git diff --cached --name-status
git diff --cached --stat
git commit -m "release: bump version to X.Y.Z"
git push origin release/X.Y.Z
```

Before committing:

- `git status --short` must show no intended release file left unstaged.
- `git diff --cached --name-status` must show the exact release file set.
- `git diff --cached --stat` must reflect versioning, changelog, docs, workflow, and release-script updates only.

## 3. Pull request and CI

```bash
gh pr create \
  --title "release: bump version to X.Y.Z" \
  --base main \
  --head release/X.Y.Z \
  --body "Release X.Y.Z"
```

Then verify:

```bash
gh pr diff <N> --name-only
gh pr view <N> --json number,state,mergeStateStatus,statusCheckRollup,url
gh pr checks <N>
```

Do not continue until the required job in workflow `CI` is green:

- `Check`

`Check` is the aggregate branch-protection gate. It must reflect both the Rust maintainer gate and
the release-target smoke matrix.

## 4. Merge handoff

```bash
gh pr merge <N> --merge --delete-branch --subject "release: bump version to X.Y.Z (#N)"
git checkout main
git pull
gh pr view <N> --json number,state,mergedAt,headRefName,baseRefName,url
```

Verify:

- PR state is `MERGED`
- `mergedAt` is populated
- local `main` contains the merge you expect
- the remote release branch is deleted

If a green PR is blocked by review requirements or admin-enforced branch protection, repository
settings have drifted away from this protocol and must be corrected before the release proceeds.
Do not work around that drift by adding manual review steps to the normal release path.

If the local `release/X.Y.Z` branch still exists:

```bash
git branch -d release/X.Y.Z
```

## 5. Tag and push

```bash
git tag vX.Y.Z
git push origin vX.Y.Z

REPO=$(gh repo view --json nameWithOwner -q .nameWithOwner)
gh api "repos/$REPO/git/ref/tags/vX.Y.Z"
```

Do not continue until the remote tag ref exists.

The tag push is what triggers `release.yml`. The PR merge alone does not publish anything.

If the release workflow later needs a targeted rerun against the existing tag:

```bash
gh workflow run release.yml -f release_tag=vX.Y.Z
```

Never create a second tag or move an existing release tag just to retry publication.
The rerun is expected to execute the maintained workflow and release scripts from `main`, while
the build jobs still check out the existing tag payload identified by `release_tag`.

## 6. Branch hygiene

After the merge and tag push, clean up stale remote-tracking refs and verify that no historical
release branches remain on GitHub.

```bash
REPO=$(gh repo view --json nameWithOwner -q .nameWithOwner)
git remote prune origin
gh api "repos/$REPO/branches" --paginate --jq '.[].name'
```

Requirements:

- no `release/X.Y.Z` branch may remain on GitHub after the merge
- no historical `release/` branches may remain on GitHub; if any are present, delete them:

```bash
git push origin --delete release/A.B.C
```

- no fully merged local `release/` branches may remain; delete them:

```bash
git branch -d release/A.B.C
```

## 7. Monitor workflow runs

```bash
TAG_SHA=$(git rev-list -n 1 vX.Y.Z)
gh run list --workflow=release.yml --event=push --commit "$TAG_SHA" --limit=20
gh run list --workflow=release.yml --event=workflow_dispatch --commit "$TAG_SHA" --limit=20
```

Inspect failed runs with:

```bash
gh run view <run-id> --log-failed
```

Never treat one failed run as authoritative if another sibling run for the same tag already
converged the release object onto the required state. The authoritative state is the GitHub
release object and its assets, not the first workflow run you happen to inspect.

## 8. Verify the GitHub release object

The release workflow is expected to create or converge the release object idempotently. Verify it
directly:

```bash
gh release view vX.Y.Z --json tagName,isDraft,isPrerelease,publishedAt,url,assets
```

Requirements:

- the release exists for tag `vX.Y.Z`
- `isDraft` is `false`
- `isPrerelease` is `false` unless intentionally prerelease
- assets include:
  - `htmlcut-X.Y.Z.zip`
  - `htmlcut-X.Y.Z.tar.gz`
  - `htmlcut-aarch64-apple-darwin`
  - `htmlcut-aarch64-apple-darwin.sha256`
  - `htmlcut-x86_64-apple-darwin`
  - `htmlcut-x86_64-apple-darwin.sha256`
  - `htmlcut-x86_64-unknown-linux-musl`
  - `htmlcut-x86_64-unknown-linux-musl.sha256`
  - `htmlcut-x86_64-pc-windows-msvc.exe`
  - `htmlcut-x86_64-pc-windows-msvc.exe.sha256`

Workflow success is not authoritative. The release object and its assets are authoritative.

## 9. Verify the public binary

Download the published standalone artifacts, verify every checksum, and execute the host-native
binary directly:

```bash
TMP_DIR="$(mktemp -d)"
gh release download vX.Y.Z \
  -p 'htmlcut-aarch64-apple-darwin' \
  -p 'htmlcut-aarch64-apple-darwin.sha256' \
  -p 'htmlcut-x86_64-apple-darwin' \
  -p 'htmlcut-x86_64-apple-darwin.sha256' \
  -p 'htmlcut-x86_64-unknown-linux-musl' \
  -p 'htmlcut-x86_64-unknown-linux-musl.sha256' \
  -p 'htmlcut-x86_64-pc-windows-msvc.exe' \
  -p 'htmlcut-x86_64-pc-windows-msvc.exe.sha256' \
  -D "$TMP_DIR"

(
  cd "$TMP_DIR"
  shasum -a 256 -c htmlcut-x86_64-apple-darwin.sha256
  shasum -a 256 -c htmlcut-x86_64-unknown-linux-musl.sha256
  shasum -a 256 -c htmlcut-x86_64-pc-windows-msvc.exe.sha256
  shasum -a 256 -c htmlcut-aarch64-apple-darwin.sha256
  chmod +x ./htmlcut-aarch64-apple-darwin
  ./htmlcut-aarch64-apple-darwin --version | grep "^htmlcut X.Y.Z$"
  ./htmlcut-aarch64-apple-darwin --help | grep "inspect"
)

rm -rf "$TMP_DIR"
```

Do not declare the release complete until every checksum file validates and the downloaded
host-native binary reports the target version.

The `grep "^htmlcut X.Y.Z$"` check intentionally validates only the first line because
`htmlcut --version` is multi-line: it prints the version line first, then the product description
from the workspace manifest.

The release workflow itself already performs runtime smoke on each target's native runner. The
local post-release command above is an additional asset-integrity check plus a host-native runtime
verification step.

## 10. Refresh the semver baseline

After the release is complete, refresh the checked-in semver baseline so future minor-version
checks compare against the latest published API:

```bash
git checkout main
git pull
cargo xtask refresh-semver-baseline --git-ref vX.Y.Z
git add semver-baseline/htmlcut-core
git commit -m "chore: refresh htmlcut-core semver baseline"
git push
```

That command repackages the published Git ref into `semver-baseline/htmlcut-core`, so the baseline
cannot silently drift to unreleased local worktree state.
