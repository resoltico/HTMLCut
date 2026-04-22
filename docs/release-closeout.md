---
afad: "3.5"
version: "4.3.0"
domain: RELEASE
updated: "2026-04-22"
route:
  keywords: [release closeout, dependabot hygiene, semver baseline refresh, primary checkout reconciliation, release cleanup]
  questions: ["how do I close out an HTMLCut release cleanly?", "when do I refresh the semver baseline?", "how do I reconcile the primary checkout after an HTMLCut release?"]
---

# Release Closeout

Use this guide for Step 10 through Step 12 of the HTMLCut release flow.

This phase begins after the public release object and host-native package are verified and ends
only after the repository, the semver baseline, and the primary checkout are all truthful again.

## 10. Triage Dependabot PRs And Clear Dependency-Automation Leftovers

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

## 11. Refresh The Semver Baseline

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

## 12. Reconcile The Primary Checkout

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
- if unpublished local work from the primary checkout is still needed, replay it deliberately onto
  a named branch based on current `main`; do not leave it only in a stash or mixed back into `main`
- if that unpublished local work is stale, superseded, or regresses the shipped release state,
  delete it instead of preserving misleading debris

Do not declare the release complete until every condition above is true at the same time.

If a disposable release worktree was created and is no longer needed:

```bash
git worktree remove "$RELEASE_WORKTREE"
```
