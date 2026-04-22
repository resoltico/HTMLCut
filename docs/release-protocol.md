---
afad: "3.5"
version: "4.2.1"
domain: RELEASE
updated: "2026-04-22"
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

Before any quality gate or version edit, identify the checkout the user will keep using after the
release. Call it the primary checkout.

Run:

```bash
git rev-parse --show-toplevel
git branch --show-current
git status --short
git fetch origin --prune --tags
git rev-list --left-right --count HEAD...origin/main
```

Requirements:

- the primary checkout path is known explicitly
- the primary checkout must not be left behind `origin/main` at release closeout
- if the primary checkout is already clean and current, release from it directly
- if the primary checkout has local work, is intentionally dirty, or lives on a problematic or slow
  filesystem, create a clean release worktree from the same repository and do the release there:

```bash
PRIMARY_CHECKOUT=$(git rev-parse --show-toplevel)
git fetch origin --prune --tags
RELEASE_WORKTREE="$(mktemp -d -t htmlcut-release-XXXXXX)"
git worktree add "$RELEASE_WORKTREE" origin/main
cd "$RELEASE_WORKTREE"
```

Use a Git worktree, not a disconnected clone, whenever possible. A worktree shares refs with the
primary checkout and makes post-release reconciliation mechanically obvious. A separate clone is a
last resort and, if used, must still be reconciled back into the primary checkout before the
release session ends.

If the primary checkout has unpublished local work, decide before the release whether that work is
real or stale. Real work must move onto a named branch or exported patch before closeout. Stale
work must be dropped. Never leave the primary checkout on stale `main` plus unpublished overlays.

Install the local maintainer toolchain if it is not already available by following
[developer-setup.md](developer-setup.md). That guide owns the exact bootstrap commands for
`rustup`, the cargo QA tools, `shellcheck`, and the macOS compiler-override safeguard.

Stable remains the default HTMLCut toolchain. Nightly is installed alongside it only for the
coverage gate, because `cargo +nightly llvm-cov --branch` is currently required for true branch
coverage.

Run the single local quality gate first:

```bash
./check.sh
```

or equivalently:

```bash
cargo xtask check
```

That gate must succeed before any release commit or tag. The maintained definition of that gate
lives in [quality-gates.md](quality-gates.md).

The gate now also verifies that the maintained Markdown docs pass docs-contract lint, that the
checked-in fuzz targets still compile, that the core/CLI contract-lint tests pass, and that the
nightly coverage toolchain prerequisites are present before coverage work begins.

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
  - `htmlcut-source-X.Y.Z.zip`
  - `htmlcut-source-X.Y.Z.tar.gz`
  - `htmlcut-X.Y.Z-aarch64-apple-darwin.tar.gz`
  - `htmlcut-X.Y.Z-x86_64-apple-darwin.tar.gz`
  - `htmlcut-X.Y.Z-x86_64-unknown-linux-musl.tar.gz`
  - `htmlcut-X.Y.Z-x86_64-pc-windows-msvc.zip`
  - `htmlcut-X.Y.Z-checksums.txt`
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

Before cutting the release branch, enumerate open PRs so dependency-automation work is never
surprise-discovered after publication:

```bash
gh pr list --state open \
  --json number,title,url,headRefName,mergeStateStatus,isDraft,author,statusCheckRollup
```

If any open PR is authored by `dependabot[bot]`, decide up front whether it changes release
machinery, release assets, or release-critical dependencies. If it does, land or reject it before
cutting the release branch. If it does not, carry that decision forward and complete Step 10
before ending the release session.

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

The release workflow now follows a draft-first publication model: it creates or reuses a draft
release, uploads the full maintained asset inventory, writes the checksum manifest, and only then
publishes the release. A rerun may repair an in-progress draft release. It must not backfill
missing assets into an already-published release.

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

Open maintenance branches such as Dependabot are handled separately in Step 10. Do not treat a
non-`release/` branch as automatically acceptable just because Step 6 only hard-fails
`release/*` leftovers.

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
  - `htmlcut-source-X.Y.Z.zip`
  - `htmlcut-source-X.Y.Z.tar.gz`
  - `htmlcut-X.Y.Z-aarch64-apple-darwin.tar.gz`
  - `htmlcut-X.Y.Z-x86_64-apple-darwin.tar.gz`
  - `htmlcut-X.Y.Z-x86_64-unknown-linux-musl.tar.gz`
  - `htmlcut-X.Y.Z-x86_64-pc-windows-msvc.zip`
  - `htmlcut-X.Y.Z-checksums.txt`

Workflow success is not authoritative. The release object and its assets are authoritative.

GitHub will also render `Source code (zip)` and `Source code (tar.gz)` links on the release page.
Those links are GitHub-generated convenience downloads and are not part of HTMLCut's maintained
asset inventory.

## 9. Verify the public binary

Download the maintained release assets, verify the checksum manifest, and execute the host-native
binary from its extracted package:

```bash
TMP_DIR="$(mktemp -d)"
gh release download vX.Y.Z \
  -p 'htmlcut-source-X.Y.Z.zip' \
  -p 'htmlcut-source-X.Y.Z.tar.gz' \
  -p 'htmlcut-X.Y.Z-aarch64-apple-darwin.tar.gz' \
  -p 'htmlcut-X.Y.Z-x86_64-apple-darwin.tar.gz' \
  -p 'htmlcut-X.Y.Z-x86_64-unknown-linux-musl.tar.gz' \
  -p 'htmlcut-X.Y.Z-x86_64-pc-windows-msvc.zip' \
  -p 'htmlcut-X.Y.Z-checksums.txt' \
  -D "$TMP_DIR"

(
  cd "$TMP_DIR"
  shasum -a 256 -c htmlcut-X.Y.Z-checksums.txt
  tar -xzf ./htmlcut-X.Y.Z-aarch64-apple-darwin.tar.gz
  ./htmlcut-X.Y.Z-aarch64-apple-darwin/htmlcut --version | grep "^htmlcut X.Y.Z$"
  ./htmlcut-X.Y.Z-aarch64-apple-darwin/htmlcut --help | grep "inspect"
)

rm -rf "$TMP_DIR"
```

Do not declare the release complete until the checksum manifest validates and the downloaded
host-native binary reports the target version.

The `grep "^htmlcut X.Y.Z$"` check intentionally validates only the first line because
`htmlcut --version` is multi-line: it prints the version line first, then the product description
from the workspace manifest.

The release workflow itself already performs runtime smoke on each target's native runner. The
local post-release command above is an additional asset-integrity check plus a host-native runtime
verification step.

## 10. Triage Dependabot PRs and clear dependency-automation leftovers

After the public release is verified, do not end the release session while open Dependabot PRs are
still sitting untriaged. Release hygiene includes dependency-automation hygiene.

Re-enumerate all open PRs and identify Dependabot-owned entries directly from GitHub metadata:

```bash
gh pr list --state open \
  --json number,title,url,headRefName,mergeStateStatus,isDraft,author,statusCheckRollup
```

Treat any PR whose `author.login` is `dependabot[bot]` as in scope for this step, even if it was
already reviewed during Step 1. Step 1 creates the release-time decision; Step 10 closes the loop
before the release session is allowed to end.

For each open Dependabot PR, inspect the exact payload and its current gate status:

```bash
gh pr diff <N> --name-only
gh pr view <N> --json number,title,state,mergeStateStatus,statusCheckRollup,url
```

Rules:

- If the PR is wanted, mergeable, and already green on the required `CI` checks, merge it
  immediately and delete its branch:

```bash
gh pr merge <N> --merge --delete-branch --subject "<title> (#<N>)"
```

- If the PR is stale, superseded by `main`, intentionally rejected, or replaced by a different
  change path, close it explicitly and delete its branch:

```bash
gh pr close <N> --comment "Superseded or intentionally rejected during release hygiene." --delete-branch
```

- If the PR needs follow-up work before it is acceptable, do that work as a normal post-release
  change on `main` and then land or replace the Dependabot PR. Do not leave a green but
  unattended Dependabot PR parked indefinitely just because the release itself already shipped.

- Never retag, amend, or move the just-published release tag to absorb a Dependabot change. The
  published release remains immutable. Dependabot resolution is post-release `main` hygiene.

- There is no "ignore it and leave the branch there" option. Every open Dependabot PR must end
  this step in exactly one of these states:
  - merged and branch deleted
  - closed and branch deleted
  - consciously kept open with an explicit still-valid reason

After each merge or close, resync and re-check GitHub branch state:

```bash
git checkout main
git pull
git remote prune origin
REPO=$(gh repo view --json nameWithOwner -q .nameWithOwner)
gh api "repos/$REPO/branches" --paginate --jq '.[].name'
```

Requirements before declaring the release session complete:

- No stale Dependabot PR may remain open without an explicit keep-open decision.
- No merged or closed Dependabot branch may remain on GitHub.
- Any remaining non-`main` branch on GitHub must correspond to an intentional still-open PR that
  was reviewed during this step and deliberately kept alive.

## 11. Refresh the semver baseline

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

## 12. Reconcile the Primary Checkout

If the release used a dedicated release worktree or any checkout other than the primary checkout,
the session is not complete until the primary checkout is truthful again. This is a blocking
release closeout gate, not an advisory cleanup reminder.

If unpublished local work from the primary checkout is still needed, move it onto a named branch
based on current `main` first, then return the primary checkout itself to `main`.

Run:

```bash
git -C "$PRIMARY_CHECKOUT" fetch origin --prune --tags
git -C "$PRIMARY_CHECKOUT" checkout main
git -C "$PRIMARY_CHECKOUT" rev-list --left-right --count HEAD...origin/main
git -C "$PRIMARY_CHECKOUT" merge --ff-only origin/main
git -C "$PRIMARY_CHECKOUT" rev-parse HEAD
git -C "$PRIMARY_CHECKOUT" status --short
```

Requirements before declaring the release session complete:

- the primary checkout `HEAD` equals `origin/main`
- the primary checkout `Cargo.toml` and `changelog.md` reflect the released version
- no stale release-only checkout may be left behind with the appearance of being authoritative
- if unpublished local work from the primary checkout is still needed, replay it deliberately onto a
  named branch based on current `main`; do not leave it only in a stash or mixed back into `main`
- if that unpublished local work is stale, superseded, or regresses the shipped release state,
  delete it instead of preserving misleading debris

Do not declare the release complete until every condition above is true at the same time.

If a disposable release worktree was created and is no longer needed:

```bash
git worktree remove "$RELEASE_WORKTREE"
```
