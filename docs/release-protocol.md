# Release protocol

One CI workflow runs direct checks, immutable published-core API checks, targeted
Miri and all four native package boundaries. Its required aggregate remains `Check`
and rejects failed/skipped mandatory jobs. One release workflow binds every artifact
to one immutable annotated tag/source on main, admits only completed same-source CI,
serializes publication, generates attribution/checksums/build provenance, refuses
published asset replacement and executes anonymous downloads on native runners.

After explicit publication authorization, create the annotated version tag and run
the release workflow. `build-source-archives.sh REF` archives exact committed source;
`build-release-artifact.sh TARGET` requires a clean source/version and bundles complete
legal notices. `native-package-evidence.py` executes the extracted binary through
semantic and actual OS I/O checks. `publish-github-release.sh` refuses changes to an
already published release; `verify-github-release.sh` verifies immutable bytes.

External steps remain: run authoritative native CI, inspect branch protection before
changing required names, confirm registry owners/access, notify consumers only when
authorized, and publish only after authorization. Source/path install works now;
registry packaging remains blocked by unpublished safety carriers. Do not publish
carriers or remove pointer corrections merely to make a dry run pass. Preserve tags
and published assets. No local check establishes remote publication or protection.

Read-only inspection on 8 October 2026 found that GitHub main branch protection
requires both `Check` and `cargo-mutants pull-request summary`, with strict checks.
`Check` is retained. The mutation workflow is retired, so remote merges remain
blocked until an explicitly authorized protection update removes its required
summary. Do not report unrelated checks under the retired mutation status name.
No repository rulesets were returned, and no protection settings were changed.
Reinspect the remote settings before rollout. Local merging does not update them.
