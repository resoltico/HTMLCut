---
afad: "3.5"
version: "4.4.0"
domain: RELEASE
updated: "2026-04-23"
route:
  keywords: [release publishing, git tag, release workflow, release assets, checksum verification, host-native smoke]
  questions: ["how do I publish an HTMLCut release tag?", "how do I verify the GitHub release object?", "how do I verify the downloaded HTMLCut package locally?"]
---

# Release Publishing

Use this guide for Step 5 through Step 9 of the HTMLCut release flow.

This phase begins after the release PR has merged into `main` and ends only after the published
GitHub release object and the downloaded host-native package have both been verified.

## 5. Tag And Push

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

The release workflow follows a draft-first publication model: it creates or reuses a draft release,
uploads the full maintained asset inventory, writes the checksum manifest, and only then publishes
the release. A rerun may repair an in-progress draft release. It must not backfill missing assets
into an already-published release.

## 6. Branch Hygiene

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

## 7. Monitor Workflow Runs

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

## 8. Verify The GitHub Release Object

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

GitHub renders `Source code (zip)` and `Source code (tar.gz)` links on the release page. Those
links are GitHub-generated convenience downloads and are not part of HTMLCut's maintained asset
inventory.

## 9. Verify The Public Binary

Download the maintained release assets, verify the checksum manifest, and execute the host-native
binary from its extracted package:

```bash
case "$(uname -s):$(uname -m)" in
  Darwin:arm64) HOST_TARGET="aarch64-apple-darwin" ;;
  Darwin:x86_64) HOST_TARGET="x86_64-apple-darwin" ;;
  Linux:x86_64) HOST_TARGET="x86_64-unknown-linux-musl" ;;
  *)
    echo "unsupported host target for local release verification" >&2
    exit 1
    ;;
esac

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
  while read -r EXPECTED ASSET_NAME; do
    [ -n "${ASSET_NAME:-}" ] || continue
    [ -f "${ASSET_NAME}" ] || continue

    if command -v sha256sum >/dev/null 2>&1; then
      ACTUAL="$(sha256sum "${ASSET_NAME}" | awk '{print $1}')"
    else
      ACTUAL="$(shasum -a 256 "${ASSET_NAME}" | awk '{print $1}')"
    fi

    if [ "${ACTUAL}" != "${EXPECTED}" ]; then
      echo "checksum mismatch for ${ASSET_NAME}" >&2
      exit 1
    fi
  done < htmlcut-X.Y.Z-checksums.txt

  tar -xzf "./htmlcut-X.Y.Z-${HOST_TARGET}.tar.gz"
  "./htmlcut-X.Y.Z-${HOST_TARGET}/htmlcut" --version | grep "^htmlcut X.Y.Z$"
  "./htmlcut-X.Y.Z-${HOST_TARGET}/htmlcut" --help | grep "inspect"
)

rm -rf "$TMP_DIR"
```

Do not declare the release complete until the checksum manifest validates and the downloaded
host-native binary reports the target version.

The `grep "^htmlcut X.Y.Z$"` check intentionally validates only the first line because
`htmlcut --version` is multi-line: it prints the version line first, then the product description
from the workspace manifest.

The release workflow already performs runtime smoke on each target's native runner. The local
post-release command above is an additional asset-integrity check plus a host-native runtime
verification step for the maintained Unix-like maintainer hosts: Apple Silicon macOS, Intel macOS,
and x86_64 Linux.
