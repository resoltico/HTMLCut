# Release protocol

The CI workflow runs direct checks, immutable published-core API checks, targeted
Miri, bounded parser/relational-selector smokes and all four native package
boundaries. Its required aggregate remains `Check` and rejects failed, cancelled,
missing or skipped mandatory jobs. The release workflow binds every artifact to
one immutable annotated tag/source on main, admits only completed same-source CI,
serializes publication, generates attribution/checksums/build provenance, refuses
published asset replacement and executes anonymous downloads on native runners.

Local registry verification establishes that normalized Cargo archives resolve,
build and install through a dependency-complete registry graph without checkout
path dependencies. It does not establish crates.io name availability, account
access, publication or authoritative native CI. Local source-archive installation
is separate evidence. Retained safety dependencies remain maintained HTMLCut
packages; their publication does not establish upstream adoption. See the
[dependency responsibilities and replacement controls](dependencies.md).

## Registry handoff

Registry publication follows the protected main rollout and successful exact-main
push-event CI described below. After explicit publication authorization, use that
exact `MAIN_SHA` and pinned toolchain. Confirm a clean checkout and workspace
version `21.0.0`. Regenerate/review the archived package graph and checksums for
`MAIN_SHA`; local branch-bound archives do not automatically bind a server merge
commit, even when source trees match.
The dependency publication order is:

1. `htmlcut-tendril 0.5.1-htmlcut.2`
2. `htmlcut-markup5ever 0.40.0-htmlcut.2`
3. `htmlcut-html5ever 0.40.1-htmlcut.3`
4. `htmlcut-servo-arc 0.5.0-htmlcut.3`
5. `htmlcut-selectors 0.41.0-htmlcut.3`
6. `htmlcut-core 21.0.0`
7. `htmlcut-cli 21.0.0`

Read-only crates.io package and owner requests on 8 October 2026 returned HTTP 404
for all seven names. This permits local preparation but does not reserve names or
prove publication access. An authorized crates.io account must create each name
on first publication and verify owners afterward. Recheck names before upload;
if another account has claimed a name, stop that publication and resolve ownership.
Keep upstream legal authors, notices and licenses in every carrier archive.

For each name in order, run `cargo publish --locked --dry-run -p NAME`, inspect
`target/package/NAME-VERSION.crate` against the reviewed archive/checksum, then
`cargo publish --locked -p NAME`. Never substitute `--no-verify` for verification.
Wait for the exact version to appear on the official registry before preparing
its dependents. For example:

```sh
curl --fail --silent --show-error \
  https://crates.io/api/v1/crates/htmlcut-tendril/0.5.1-htmlcut.2
cargo owner --list htmlcut-tendril
```

Repeat those exact API and owner checks with each package/version above. Official
dry runs before the owned dependencies are published cannot resolve that graph;
record their failures separately from successful temporary-registry verification.
While still in the verified checkout, preserve its pinned toolchain with
`export RUSTUP_TOOLCHAIN="$(rustup show active-toolchain | cut -d ' ' -f1)"`.
After all publications, from a fresh directory with a fresh `CARGO_HOME`, run
`cargo install --locked htmlcut-cli --version '=21.0.0'`, then execute strict
extraction and rejection controls and compare the installed package identities
with the delivered graph. Registry publication/install remains open until this
real boundary succeeds.

## Authoritative CI and native release handoff

Read-only reinspection on 8 October 2026 confirms main requires a pull request
(review approval count is zero), strict status checks and conversation resolution.
Force pushes/deletions are disabled; linear-history enforcement is disabled.
Merge commits, squash and rebase are enabled at repository level. Administrator
enforcement is disabled, but this handoff does not authorize bypassing protection.
The reviewed route uses a pull request, not a direct main push.

After explicit branch-push, workflow and PR authorization, set `SOURCE_SHA` to the
final locally verified commit from the completion report. From its clean checkout:

```sh
test -z "$(git status --porcelain)"
test "$(git rev-parse HEAD)" = "$SOURCE_SHA"
git fetch origin main
git merge-base --is-ancestor origin/main "$SOURCE_SHA"
git push origin "$SOURCE_SHA:refs/heads/codex/dependency-boundaries"
gh workflow run ci.yml --ref codex/dependency-boundaries
gh run list --workflow ci.yml --branch codex/dependency-boundaries --commit "$SOURCE_SHA"
```

If origin/main is not an ancestor, stop before pushing: integrate its changes into
a reviewed branch without rewriting published history, resolve conflicts, run the
required local checks and repeat CI for that resulting SHA. Never treat identical
file trees as identical source revisions. Inspect `gh run view RUN_ID --log` and
`gh run download RUN_ID --dir verification-SOURCE_SHA`; require completed `Check`,
bounded smoke and nonempty evidence for all four native jobs at that branch SHA.

Create a PR with the reviewed title/body (for example, the prepared external
handoff body accompanies the completion evidence):

```sh
gh pr create --base main --head codex/dependency-boundaries \
  --title 'Reduce HTMLCut dependency boundaries and prepare the package graph' \
  --body-file htmlcut-pull-request-body.md
```

Its required PR checks must complete at the exact reviewed head; satisfy
conversation resolution and any review requirements still in effect. Only after
separate remote-settings authorization apply the narrow required-status change
below, retiring the obsolete mutation status. Recheck the full protection state.
Do not merge while required Check is missing or failing. With separate merge
authorization, use the enabled merge-commit route:

```sh
gh pr checks PR_NUMBER --required --watch
gh pr merge PR_NUMBER --merge --match-head-commit "$SOURCE_SHA"
git fetch origin main
MAIN_SHA="$(git rev-parse origin/main)"
gh run list --workflow ci.yml --branch main --commit "$MAIN_SHA"
```

A server merge creates a new `MAIN_SHA`. Bind fresh source archives, Cargo VCS
metadata/checksums and native evidence to it, even if its tree matches the reviewed
branch. The main push-event CI runs automatically after merging and must complete
successfully at exactly `MAIN_SHA`; dispatch-only CI cannot replace the release
guard's required push-event verification. Intel macOS, Linux musl, Windows MSVC
and authoritative GitHub CI remain externally unverified until those jobs execute.
Local ARM64 execution cannot close them. Before tagging, verify the exact main
run with the existing guard:

```sh
gh api --method GET repos/resoltico/HTMLCut/actions/workflows/ci.yml/runs \
  -f head_sha="$MAIN_SHA" -f branch=main -f per_page=100 > main-ci.json
python3 scripts/check-release-verification.py --source-sha "$MAIN_SHA" --ci main-ci.json
```

After exact-main evidence and explicit release authorization, create the annotated
version tag once and publish through the existing workflow:

```sh
git tag -a v21.0.0 "$MAIN_SHA" -m 'HTMLCut 21.0.0'
git push origin refs/tags/v21.0.0
```

The tag push starts Release. If an existing authorized tag needs a workflow retry,
use `gh workflow run release.yml --ref v21.0.0 -f release_tag=v21.0.0` without
moving/replacing the tag. Inspect its completed run and downloaded artifacts.
`build-source-archives.sh REF` archives exact committed source;
`build-release-artifact.sh TARGET` requires clean source/version and bundles full
legal notices. `native-package-evidence.py` executes the extracted binary through
semantic and actual OS I/O checks. `publish-github-release.sh` refuses changes to
published releases; `verify-github-release.sh` verifies immutable bytes. Preserve
all published tags, releases and assets.

The common builder signs only the staged macOS executable with
`codesign --force --sign - --timestamp=none --identifier htmlcut` before archiving.
Extracted ARM64 and Intel executables undergo strict signature and ad-hoc identifier
verification before execution; native evidence includes their post-sign hashes and
real unsigned/tampered rejection controls. Verify a downloaded macOS executable with
`codesign --verify --strict --verbose=2 ./htmlcut`. Ad-hoc signing checks sealed-code
integrity, supplies no Developer ID identity or Apple notarization, and does not
guarantee Gatekeeper acceptance of a quarantined download. For a trusted download
blocked by macOS, follow [Apple's per-application approval guidance](https://support.apple.com/en-us/102445).
Checksums and GitHub provenance remain separate reference checks. This policy
applies to newly built packages; published assets remain unchanged.

## Required-status handoff

Read-only inspection on 8 October 2026 found strict main protection requiring
`Check` (app ID null) and `cargo-mutants pull-request summary` (app ID 15368).
The mutation workflow is retired. No repository rulesets were returned. The
prepared change removes only the obsolete status and preserves strict checking,
the `Check` context and every other protection setting. The request uses
`app_id: -1` to preserve the current unrestricted provider (reported as null in
the read response), following the [GitHub status-check API](https://docs.github.com/en/rest/branches/branch-protection#update-status-check-protection).
It does not spoof the retired status.

After explicit remote-settings authorization, reinspect immediately before the
change. Preserve the read response and verify it still matches the reviewed
prerequisite. Use the status-check subresource rather than replacing full branch
protection:

```sh
gh api repos/resoltico/HTMLCut/branches/main/protection > protection-before.json
gh api repos/resoltico/HTMLCut/rulesets > rulesets-before.json
printf '%s\n' '{"strict":true,"checks":[{"context":"Check","app_id":-1}]}' \
  > required-status-checks.json
gh api --method PATCH \
  repos/resoltico/HTMLCut/branches/main/protection/required_status_checks \
  --input required-status-checks.json
gh api repos/resoltico/HTMLCut/branches/main/protection > protection-after.json
```

Compare before/after: only the obsolete context/check may disappear. Reviews,
conversation resolution, administrator enforcement, signatures, force-push and
delete permissions, linear history, branch locking and other settings must remain
unchanged. If the remote state has changed, revise the narrow payload from the
current settings before applying it. Run authoritative `Check` before relying on
remote merging. No local merge updates remote protection.
