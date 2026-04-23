---
afad: "3.5"
version: "4.4.0"
domain: RELEASE
updated: "2026-04-23"
route:
  keywords: [release preflight, gh auth, release branch, release pr, primary checkout, check gate]
  questions: ["how do I prepare an HTMLCut release checkout?", "what must pass before tagging an HTMLCut release?", "how do I open the HTMLCut release PR?"]
---

# Release Preflight

Use this guide for Step 0 through Step 4 of the HTMLCut release flow.

The phase ends only after the release PR is merged into `main` and the release branch is deleted.

## 0. GitHub CLI Gate

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

If the primary checkout is dirty because it already contains the intended release-candidate work,
capture that state explicitly before creating the clean release worktree. One maintained pattern
is:

```bash
PRIMARY_CHECKOUT=$(git rev-parse --show-toplevel)
git checkout -b release-prep/X.Y.Z
git add <every already-intended release file>
git commit -m "chore: prepare X.Y.Z release candidate"
git fetch origin --prune --tags
RELEASE_WORKTREE="$(mktemp -d -t htmlcut-release-XXXXXX)"
git worktree add -b release/X.Y.Z "$RELEASE_WORKTREE" release-prep/X.Y.Z
cd "$RELEASE_WORKTREE"
```

That keeps the release worktree clean without discarding the real unpublished state that must ship.
Do not hand-copy a dirty diff into a temporary checkout and hope it still matches later.

Install the local maintainer toolchain if it is not already available by following
[developer-setup.md](developer-setup.md). That guide owns the exact bootstrap commands for
`rustup`, the cargo QA tools, `shellcheck`, and the macOS compiler-override safeguard.

Rust `1.95.0` is the pinned HTMLCut repository toolchain. Nightly is installed alongside it for
the coverage gate and for live `cargo-fuzz` campaigns, because `cargo +nightly llvm-cov --branch`
and `cargo +nightly fuzz ...` both need nightly.

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

The gate verifies the maintained Markdown docs, the checked-in fuzz targets, the CLI/core
contract-lint coverage, the exact pinned stable-toolchain prerequisites before the Rust gate
starts, and the nightly coverage prerequisites before coverage work begins.

Then verify:

- `Cargo.toml` `[workspace.package] version` equals the target release version exactly. This is the
  single version source of truth for both crates and for `htmlcut --version`.
- the release commit also updates every other maintained version-bearing surface that is expected
  to match the workspace release:
  - the path dependency versions for `htmlcut-cli`, `htmlcut-core`, and `htmlcut-tempdir` in
    `Cargo.toml`
  - the maintained Markdown metadata `version` fields in `README.md`, `CONTRIBUTING.md`,
    `PATENTS.md`, `fuzz/README.md`, and the maintained `docs/*.md` set
  - the concrete release-version literals in `README.md` install snippets
  - the local path-package entries in `Cargo.lock`, so the subsequent locked gate reflects the
    release version truthfully
- `Cargo.toml` `[workspace.package] rust-version` still matches the pinned repository compiler
  contract, and the workspace crates still inherit it through
  `rust-version.workspace = true`.
- `Cargo.toml` `[workspace.package] description` still reflects the current product in task-facing
  language. `htmlcut-cli` inherits it for CLI help and for the second line of `htmlcut --version`.
- `docs/operations.md` still reflects the current canonical operation catalog exposed by
  `htmlcut-core`.
- `changelog.md` has a `## [X.Y.Z] - YYYY-MM-DD` section with at least one entry.
- `README.md` still documents the current user-facing install flow, CLI model, and release assets.
- `CONTRIBUTING.md` still matches the maintained contributor workflow, fixture-update flow, and
  release expectations.
- `docs/README.md` still points at the maintained developer and maintainer docs.
- `docs/versioning-policy.md` still matches the shipped contract policy, frozen interop model, and
  semver-baseline rules.
- `docs/cli.md`, `docs/core.md`, `docs/schema.md`, and `docs/interop-v1.md` still match the shipped
  surfaces.
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
  - `main` requires conversation resolution before merge
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

## 2. Release Branch

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
- `git diff --cached --stat` must reflect versioning, changelog, docs, workflow, and release-script
  updates only.
- the staged release diff must include the full version-bearing surface described in Step 1, not
  just the workspace manifest line by itself

## 3. Pull Request And CI

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

If `gh pr diff <N> --name-only` fails with HTTP `406` because the diff exceeds GitHub's
line-limit for that endpoint, enumerate the changed file set through the pull-files API instead:

```bash
REPO=$(gh repo view --json nameWithOwner -q .nameWithOwner)
gh api "repos/$REPO/pulls/<N>/files" --paginate --jq '.[].filename'
```

Do not continue until the required job in workflow `CI` is green:

- `Check`

`Check` is the aggregate branch-protection gate. It must reflect both the Rust maintainer gate and
the release-target smoke matrix.

## 4. Merge Handoff

```bash
gh pr merge <N> --merge --delete-branch --subject "release: bump version to X.Y.Z (#N)"
git checkout main
git fetch origin --prune --tags
git merge --ff-only origin/main
gh pr view <N> --json number,state,mergedAt,headRefName,baseRefName,url
```

Verify:

- PR state is `MERGED`
- `mergedAt` is populated
- local `main` contains the merge you expect
- the remote release branch is deleted

If a green PR is blocked only because conversations are unresolved, resolve or close those threads
and then merge normally. If it is blocked by review requirements or admin-enforced branch
protection, repository settings have drifted away from this protocol and must be corrected before
the release proceeds. Do not work around that drift by adding manual review steps to the normal
release path.

If the local `release/X.Y.Z` branch still exists:

```bash
git branch -d release/X.Y.Z
```
