---
afad: "4.0"
version: "20.0.0"
domain: RELEASE
updated: "2026-10-05"
route:
  keywords: [release publishing, git tag, release workflow, release assets, checksum verification, host-native smoke]
  questions: ["how do I publish an HTMLCut release tag?", "how do I verify the GitHub release object?", "how do I verify the downloaded HTMLCut package locally?"]
---

# Release Publishing

Use this guide for Step 5 through Step 9 of the HTMLCut release flow.

This phase begins after the release PR has merged into `main` and ends only after the published
GitHub release object and the downloaded host-native package have both been verified.

Every maintained release helper script is self-describing. Run `./scripts/<name>.sh --help`
locally before using a helper you do not already know.

## 5. Tag And Push

```bash
RELEASE_TAG=vX.Y.Z
RELEASE_COMMIT=$(git rev-parse origin/main)
git tag -a "$RELEASE_TAG" "$RELEASE_COMMIT" -m "HTMLCut ${RELEASE_TAG#v}"
git push origin "$RELEASE_TAG"

TAG_OBJECT_TYPE=$(git cat-file -t "$RELEASE_TAG")
TAG_COMMIT=$(git rev-parse "$RELEASE_TAG^{commit}")
[ "$TAG_OBJECT_TYPE" = "tag" ]
[ "$TAG_COMMIT" = "$RELEASE_COMMIT" ]

REPO=$(gh repo view --json nameWithOwner -q .nameWithOwner)
gh api "repos/$REPO/git/ref/tags/$RELEASE_TAG"
```

Do not continue until the remote tag ref exists, the local tag is annotated, and its peeled commit
is the exact merged `origin/main` commit. This prevents a release from being accidentally cut from
an unmerged, detached, or stale local checkout.

The tag push is what triggers `release.yml`. The PR merge alone does not publish anything.

If the release workflow later needs a targeted rerun against the existing tag:

```bash
gh workflow run release.yml -f release_tag=vX.Y.Z
```

Never create a second tag or move an existing release tag just to retry publication.
The rerun is expected to execute the maintained workflow and release scripts from `main`, while
the build jobs still check out the existing tag payload identified by `release_tag`. Publication
scripts resolve the release version and asset inventory from that tag's `Cargo.toml`, rather than
from the potentially newer `main` checkout that supplies repaired workflow logic.

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

The release workflow also emits GitHub build-provenance attestations for the source archives,
standalone packages, and checksum manifest. Those attestations are part of the maintained
publication story, but they live in GitHub's attestation system rather than in the named release
asset inventory above.

The maintained `htmlcut-source-X.Y.Z.*` assets are produced with `git archive`, so repository
paths marked `export-ignore` in `.gitattributes` stay committed in GitHub while remaining absent
from the downloadable source archives.

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
  grep "${HOST_TARGET}" "./htmlcut-X.Y.Z-${HOST_TARGET}/README.md"
  ! grep -q "From source" "./htmlcut-X.Y.Z-${HOST_TARGET}/README.md"
  "./htmlcut-X.Y.Z-${HOST_TARGET}/htmlcut" --version | tr -d '\r' | grep "^htmlcut X.Y.Z$"
  printf '%s\n' '<article><a class="more" href="../guide.html">Read more</a></article>' > ./page.html
  FIRST_OUTPUT="$("./htmlcut-X.Y.Z-${HOST_TARGET}/htmlcut" extract --file ./page.html --css 'article a.more' --read attribute:href --bundle ./article-link.htmlcut.tar)"
  [ -f ./article-link.htmlcut.tar ]
  printf '%s' "${FIRST_OUTPUT}" | python3 -c 'import json,sys; assert json.load(sys.stdin) == ["../guide.html"]'
  rm ./page.html
  REPLAY_OUTPUT="$("./htmlcut-X.Y.Z-${HOST_TARGET}/htmlcut" run ./article-link.htmlcut.tar)"
  [ "${REPLAY_OUTPUT}" = "${FIRST_OUTPUT}" ]
)

rm -rf "$TMP_DIR"
```

Do not declare the release complete until the checksum manifest validates, the packaged README
identifies the target package without leaking source-build instructions, and the downloaded
host-native binary completes one extraction-plus-bundle replay after deleting the original input from the extracted package.

The release workflow already performs runtime smoke on each target's native runner. The local
post-release command above is an additional asset-integrity check plus a host-native runtime
verification step for the maintained Unix-like maintainer hosts: Apple Silicon macOS, Intel macOS,
and x86_64 Linux.

Release jobs resolve the requested immutable tag commit once and every archive/native/publication job checks out that SHA. Source packaging accepts an explicit ref through `scripts/build-source-archives.sh`. Notes come exclusively from the exact version section of the source revision's changelog through `scripts/release-notes.py`; absent, duplicate and empty sections fail. Unreleased preview is explicitly nonpublishing. Publication remains separately authorized.

Dispatch tag input is canonicalized before checkout and never inserted into shell program text.
The producer requires an annotated tag reachable from main, then checks successful same-source
main CI and a full mutation workflow before distributing one source/tag/version binding. Every
downstream release helper receives `RELEASE_SOURCE_SHA` and rejects a tag that moved afterward.

Before publishing a draft, `release-assets.py` verifies immutable notes/title, the exact registry
asset set and prepared checksum manifest, and hashes the bytes of every existing uploaded asset.
Missing draft assets can be uploaded; mismatched assets fail without clobbering them. A matching
public release is verification-only, with no latest/title/status edit on a retry. New drafts below
an existing stable release are rejected. Publication workflows serialize across tags and preserve
in-progress work rather than cancelling it for a newer dispatch.

Post-publication consumer verification streams the published checksum file and assets without
requiring all four native builds locally. Downloads are bounded and hashed as exact raw bytes;
terminal escape processing is explicitly disabled for the private hash pipe. This verifies provider
bytes/checksum consistency; GitHub build attestations separately carry source provenance. The
initial publication additionally compares uploads to the locally prepared package bytes.

After public publication, the release workflow anonymously downloads each target's package and
checksum on its matching native runner, verifies its checksum and source-bound build attestation,
and executes the downloaded binary through the complete native package and OS I/O controls.
All four downstream jobs must succeed before release closeout. Their retained artifacts include
the downloaded bytes, attestation verification, matrix and Windows proofs referenced by the
manifest. A prepublication build smoke alone does not establish published download execution.
