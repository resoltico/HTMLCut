# Changelog

Notable changes to this project are documented in this file. The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [11.0.1] - 2026-07-16

### Changed
- `htmlcut-v1` now publishes `htmlcut.result@8` and `htmlcut.error@3`. The revised documents bound every public error and diagnostic message to 1024 UTF-8 bytes and reject invalid selector-error payloads before canonical JSON or digests are produced.
- Invalid CSS selector errors now use a stable safe message and carry a closed, one-based source position plus an HTMLCut-owned parse-error class in both the diagnostic and error-detail carriers; the two copies must agree exactly.

### Fixed
- Interop error finalization now sanitizes rejected diagnostics into bounded structured rejection evidence instead of reusing an invalid payload or aborting while constructing its fallback error.

## [11.0.0] - 2026-07-15

### Added
- `htmlcut-v1` CSS-selector text and structured plans can now declare `dom_canonicalization` to remove explicitly named attributes or whitespace-only text nodes from a detached selected clone, while candidate selection and original DOM evidence remain unchanged.
- Added validator-grade `htmlcut.plan@7` and `htmlcut.result@7` JSON Schema exports for the DOM-canonicalization and evidence-boundary rules that standard JSON Schema can express; runtime validation continues to enforce cross-field equality and canonical digest invariants.

### Changed
- The `htmlcut-v1` result model now separates original evidence from comparison output: `text_output`, HTML fields, match metadata, and structured payloads retain their original-DOM values, while canonicalized CSS comparison text is exposed separately as `comparison_text_output` and becomes text output's `output_value` when configured.
- Advanced the interop plan and result schema revisions to `7` and `HTMLCUT_EXTRACTION_SEMANTICS_VERSION` to `3`. Canonicalization is valid only for CSS text or structured output; delimiter, raw-HTML, and direct-attribute outputs reject it rather than accepting an inert policy, and direct-attribute measurement continues to use original match metadata.
- Refreshed the checked-in `htmlcut-core` semver baseline from `v10.3.1`, so subsequent compatibility checks compare against the released interop error contract rather than an older public snapshot.

## [10.3.1] - 2026-07-15

### Fixed
- Interop extraction errors now always retain the pre-selection `candidateCount` in `details.core_details`, including `NO_MATCH`, so downstream policy engines can persist the exact candidate count together with HTMLCut's diagnostic reason.

## [10.3.0] - 2026-07-15

### Changed
- HTMLCut now publishes an independently versioned extraction-semantics identity for interop consumers: `HTMLCUT_EXTRACTION_SEMANTICS_VERSION` and `HtmlInput::extraction_identity_sha256(&Plan)` bind the complete input, complete plan, and the monotonic semantics counter without coupling persisted identity to crate, core-specification, or dependency versions.
- Raised every shipped crate's published Rust floor and the pinned stable toolchain to Rust `1.97`/`1.97.0`; refreshed the Cargo graph, QA tools, and immutable GitHub Actions pins.
- Release closeout now says the quiet but required sequencing step explicitly: when the session itself changes maintainer tooling or release docs after publication, commit the refreshed `semver-baseline/htmlcut-core` snapshot first and rerun the full maintainer gate from that committed `main` tree, because `cargo xtask check` intentionally rejects a dirty checked-in semver baseline.
- Release publication now uses annotated tags pinned to the merged `origin/main` commit, and post-tag reruns derive their version and asset inventory from the immutable tagged manifest rather than from potentially newer `main` release tooling.

### Fixed
- `cargo xtask refresh-semver-baseline --git-ref ...` now rewrites the released workspace's vendored selector/parser dependency aliases back to registry coordinates before packaging the checked-in baseline snapshot, so post-release semver closeout works even though downstream git/path consumers intentionally ship the patched `htmlcut-*` crates instead of root-only patch overrides.

## [10.2.0] - 2026-05-19

### Changed
- HTMLCut now vendors the full downstream-safe selector/parser stack under `patches/rust/` (`scraper`, `selectors`, `html5ever`, `markup5ever`, `servo_arc`, and `tendril`) so git/path consumers inherit the same strict-provenance dependency graph that `htmlcut-core` verifies locally instead of relying on root-only `[patch.crates-io]` overrides that downstream Cargo builds do not see.
- The vendored runtime stack now trims upstream-only cargo surfaces that HTMLCut does not ship (`selectors` bench/shared-memory hooks and `servo_arc` Gecko refcount logging), so the maintained `--all-features` gate, doctests, and downstream consumers all verify the same supported dependency contract instead of accidentally exercising integration-only feature flags.
- Release preflight now states the missing but required maintainer step explicitly: if the version-bearing release surface changes after the first gate pass, rerun the full gate before cutting or pushing the release branch so the shipped candidate is the exact verified tree.

### Fixed
- `cargo xtask refresh-semver-baseline` now packages the published snapshot into an explicit temp-owned Cargo target/build root instead of assuming `snapshot/target/package`, so release closeout works correctly even when the released tree carries repo-owned Cargo artifact layout settings or the operator has ambient Cargo artifact overrides.
- The ordinary maintainer and cross-platform CI Rust gates no longer inject `CC=clang CXX=clang++` into every Cargo command, so Windows CI keeps using its native MSVC toolchain for normal builds while the dedicated LLVM-backed coverage and fuzz flows retain the explicit clang override they actually need.
- `cargo xtask miri` now proves both selector validation and delimiter slice extraction, including document-title parsing on the slice path, and `cargo xtask outdated-check` now rewrites the repo-owned vendored selector/parser dependencies back to registry coordinates inside its temporary workspace so dependency-freshness checks keep working after the downstream-safe stack moved out of root-only patch tables.
- `ParsedDocument` and `ParseDocumentResult` now preserve their historical unwind-safety contract explicitly, so the downstream-safe vendored parser stack does not force a semver-breaking change onto the public `parse_document` wrapper types.

## [10.1.0] - 2026-05-17

### Changed
- Tightened the maintainer release protocol so release PR merge handoff no longer relies on `gh pr merge --delete-branch` from a disposable checked-out release worktree, avoiding false local merge-failure reports after a successful remote merge and branch deletion.
- Promoted build-artifact hygiene to a first-class maintained system: Cargo now routes normal workspace artifacts outside the repo root through committed `build.target-dir` and `build.build-dir` ownership, `cargo xtask` gained `hygiene report|verify|clean` subcommands, the maintainer Rust gates now enforce repo-local artifact hygiene before and after the command plan, coverage now uses separate sibling managed roots instead of nesting inside the main Cargo trees, hygiene reports count unique artifact bytes without double-counting diagnostic aggregates, nested `cargo llvm-cov` worktrees are tagged and verified as managed coverage artifacts too, ambient caller Cargo env no longer overrides the repo-owned artifact layout truth, and the docs/devcontainer helpers now describe and honor the same artifact policy.
- HTMLCut now canonically vendors the unreleased `servo_arc 0.4.3` and `tendril 0.5.0` selector and parser-stack safety fixes inside `patches/rust`, `cargo xtask` gained a maintained strict-provenance `miri` command plus a selector-safety Miri proof inside the main `check` gate, and the contributor/devcontainer bootstrap now installs and validates the nightly `miri` and `rust-src` components as part of the repository-owned maintainer toolchain contract.
- The document text renderer now lives behind an explicit `document::text` subsystem with shared structural vocabularies plus a dedicated reader-cleanup policy layer, and the CLI now exposes in-memory HTML as a first-class `--input-html <HTML>` source instead of forcing literal-source workflows through stdin.
- `cargo xtask check` now treats the final coverage pass as the canonical maintained Rust package execution owner, so `xtask`, `htmlcut-core`, `htmlcut-cli`, and `htmlcut-tempdir` test targets run once under coverage instead of being replayed earlier in the gate and then rerun again for scoring.
- The coverage gate now distinguishes declarative Rust sources from executable modules by parsing tracked file shape, so module routers and constant vocabulary files remain part of the maintained source inventory without being falsely reported as missing executable coverage.
- `cargo xtask check` now mirrors the raw contributor-devcontainer validator automatically when the current branch differs from `origin/main` under the devcontainer gate's watched paths, so local maintainer verification catches contributor-container regressions before PR CI does while `./scripts/devcontainer-check.sh` remains the dedicated full host-side container proof.

### Fixed
- The shipped selector-validation and selector-execution path no longer relies on the known Miri provenance failures in upstream `servo_arc 0.4.3` and `tendril 0.5.0`, the maintained proof now runs under strict provenance, and `NOTICE` now reflects the local patched dependencies truthfully instead of listing `servo_arc` under the MPL-only Servo crates.
- Standalone release packaging now resolves compiled binaries from Cargo's canonical target directory instead of hardcoding repo-local `target/` paths, so the release smoke matrix stays in lockstep with the managed artifact-root contract, the Windows cross-platform Rust gate no longer trips over Unix-only hygiene test helpers, and the repo-owned `./scripts/xtask.sh` stable launcher now builds `xtask`, copies the host binary outside the mutable Cargo artifact roots, and runs that temporary copy so Windows does not deadlock on a live `xtask.exe` file lock.
- Text extraction for explicit selections no longer discards the selected root merely because its tag, role, or class looks like utility chrome, so `select` and `slice` now return readable text for intentionally selected fragments such as status or pricing blocks while preserving reader-cleanup behavior for descendants and whole-document review.
- The contributor devcontainer bootstrap no longer fails when one sourced shell helper inherits readonly `script_dir` or `repo_root` bindings from another, because the shared Rust tooling helper now keeps its internal path state namespaced instead of colliding with caller shell locals during release and CI validation, and the bootstrap now retries transient Rustup network failures instead of treating one dropped TLS connection as a hard gate failure.
- The contributor devcontainer maintainer gate now keeps `cargo xtask` on the same cache-root contract as the surrounding container shell: explicit `CARGO_TARGET_DIR` and `CARGO_BUILD_BUILD_DIR` overrides from the container entrypoint now win over the repo-default sibling artifact layout, so the strict-provenance Miri preflight no longer falls back to an unwritable sibling root during release and CI container runs.
- The contributor devcontainer release gate now survives single-ref and shallow CI checkouts cleanly: the watched-path probe falls back to `HEAD` when `origin/main` is absent locally, and the shared shell helpers no longer trip ShellCheck on their Cargo-metadata fallback path during the maintainer gate. The `xtask` command-execution proofs now run against isolated managed test artifact roots instead of assuming the repo's sibling cache root is writable, so contributor CI and local maintainer runs verify the same artifact-layout contract.

## [10.0.0] - 2026-05-14

### Changed
- Promoted the workspace and current unpublished contract line to `10.0.0` because the request, interop, and runtime boundary cleanups in this development line intentionally break the published `9.0.0` surface and therefore require a new major release identity.
- Bumped the request-side public contract to `htmlcut.extraction_definition@4` and core request spec version `7`, made persisted request identities explicit and mandatory, and rewrote slice request documents to carry a nested `request.extraction.pattern` object instead of the older flatter boundary layout.
- Refined release closeout guidance so overlapping Dependabot PRs that touch the same write set are consolidated through one maintainer-owned `main` update instead of churny sequential merges that immediately put sibling branches behind.
- Updated the workspace dependency graph to `assert_cmd 2.2.2` and `scraper 0.27.0` after the `9.0.0` publication, keeping post-release `main` aligned with the reviewed Dependabot payloads without leaving superseded bot branches open.
- Refreshed the checked-in `semver-baseline/htmlcut-core` snapshot from the published `v9.0.0` tag so future API comparisons use the actual released 9.0.0 surface and its recorded provenance.
- Reworked the CLI help and discovery presentation so long help stays grammar-first with examples after the option surface, and `catalog` / `schema` now group discovery filters under explicit filter headings instead of burying them under filesystem-output controls.

### Fixed
- Reusable request files now form one strict closed loop: emitted request files replay through the same maintained schema, legacy slice `include_start` / `include_end` booleans are rejected instead of being silently ignored, and request-file emission refuses URL inputs that would persist userinfo, query strings, or fragments.
- Exported request schemas are now feature-stable across `http-client` builds, no-HTTP builds reject URL execution at the capability boundary instead of at the schema boundary, and public display/result URLs only allow the explicit `?[redacted]` query marker in serialized artifacts.
- Help text, contract metadata, and CLI docs now describe bundle output truthfully: extraction bundles write `selection.json`, `selection.html`, `selection.txt`, and `report.json`, and each command family scopes `--overwrite` to the writable targets that command actually owns instead of claiming unrelated request-file or bundle surfaces.
- `inspect source` now demotes broad wrappers that preserve nearly all text while adding large heading-only shells, so wiki and documentation pages promote the real article body ahead of repository or page chrome in both extraction and reading suggestions.
- The getting-started guide now states that `inspect select` and `inspect slice` default to human-readable text previews and require `--output json` for structured preview reports, keeping the published quick-start flow aligned with the actual CLI surface.

## [9.0.0] - 2026-05-13

### Changed
- Promoted the workspace and published contract surface to `9.0.0` because this release makes intentional hard breaks across `htmlcut-core`, the CLI request/output vocabulary, and the interop planning contracts in pursuit of a cleaner public model.
- Hardened the core request/runtime contract around explicit validated boundary types: reusable extraction-definition files and exported schemas now flow through a dedicated `htmlcut_core::wire::v1` document layer, slice requests serialize named `boundary_retention` modes, slice HTML outputs distinguish `selected-html` from true `inner-html`, and runtime options now use validated non-zero byte/timeout wrappers plus an explicit TLS trust policy.
- Consolidated repository toolchain ownership so `rust-toolchain.toml` owns the exact stable pin, `[workspace.package] rust-version` carries the published compatibility floor, the contributor bootstrap scripts expose that toolchain contract canonically, and cross-platform CI now runs a shared `cargo xtask ci-rust-gate` plan instead of maintaining a second hard-coded Rust gate in GitHub Actions.
- Consolidated the full `htmlcut-core` example, extraction, interop acceptance, and interop property suites into the crate's in-library all-features test harness and taught the maintained Rust gates to verify that surface through `cargo test -p htmlcut-core --lib --all-features --locked`, eliminating pathological macOS startup/discovery overhead from standalone core test binaries.
- Reworked the CLI help and discovery surfaces so grammar comes first and operator guidance comes second: short `-h` output now stays concise, long `--help` output carries examples plus an explicit operator guide, `inspect` previews share the same value-mode vocabulary as final extraction, and `catalog` / `schema` text now use public contract-family labels instead of leaking internal Rust type or module spellings.
- Rewrote the root README in direct onboarding language, replaced metaphor-heavy reusable-request wording with literal command guidance, linked the storefront README to the complete `docs/README.md` documentation index, and tightened the docs contract so that index must keep covering every maintained Markdown document under `docs/`.

### Fixed
- Core URL handling now rejects non-HTTP schemes and credential-bearing userinfo at the typed boundary, redacts URL display values in diagnostics and serialized metadata, leaves `effective_base_url` absent until document parsing actually resolves it, and enforces local-file size limits on the same open handle that is read.
- Selector extraction no longer re-runs selectors after URL rewriting, HTML/CSS URL rewriting now covers supported HTML URL-bearing attributes plus CSS `url(...)` and quoted `@import` references, attribute extraction canonicalizes HTML attribute names, CLI JSON parse failures no longer guess the wrong command label from raw argv, and the rendered-whitespace contract is now named and documented truthfully.
- Maintainer tooling now parses workspace/toolchain TOML with typed deserializers, `xtask` uses a typed error surface, crates that do not require unsafe code now forbid it explicitly, and the contributor cargo-tool installer fails fast on missing macOS `pkgconf` / `openssl@3` prerequisites instead of aborting deep inside a native dependency build. The semver-baseline refresh path now also writes `semver-baseline/htmlcut-core/BASELINE.toml` so the checked-in API snapshot records its published Git ref and packaged crate version.
- The maintained Rust gate now runs the full `htmlcut-cli` package suite through package-level `cargo test` instead of `cargo nextest`, eliminating nondeterministic macOS hangs in CLI test enumeration while preserving the same strict package-level verification surface.
- CLI success paths now confirm written artifacts instead of ending in silent file-only success, `inspect source --output text` no longer drags DOM-path noise into selector and link previews, request-file previews honor the saved value mode instead of silently forcing `structured`, and preview parameter inventories now group value-shaping flags under extraction semantics instead of mislabeling them as selection controls.
- Plain-text rendering now resolves displayed link destinations to absolute URLs whenever an effective base is known, strips inline citation/backreference noise, and replaces hidden MathML fallback junk with cleaner reader text so live article extraction is more portable and readable.
- `inspect source` now promotes precise reading descendants over broader wrappers when the outer container mostly adds chrome links, which fixes real-world pages such as Wikipedia articles and GitHub wiki layouts where narrower content roots were being buried under shell selectors.
- Verbose stderr for successful extraction and inspection runs now includes selected-match context, effective-base reporting, and source-load stage traces at the first verbose level instead of a lone summary line.
- Bundle `report.json` artifacts now omit duplicate per-match `html` and `text` sidecar payloads because those bodies already live in `selection.html` and `selection.txt`, substantially shrinking forensic bundles without dropping the canonical extracted value or match metadata.
- Reader-text extraction now suppresses empty decorative list bullets and collapses immediately repeated adjacent headings, which cleans up duplicated mobile/desktop promo shells and empty marketing-card scaffolding on complex live product pages.

## [8.0.0] - 2026-05-05

### Fixed
- `inspect source` now reports truthful body-text character counts, samples headings and link previews across the ranked content-root candidates instead of relying on one candidate rank, and no longer lets `data-*` metadata or editability markers suppress real headings in rendered text.
- Bundled `selection.html` wrappers now inherit a discovered source language instead of hardcoding `lang="en"`, the maintainer gate now fails fast if `semver-baseline/htmlcut-core` is dirty, and the saved-request examples now distinguish request contracts from replayed output filenames so the CLI help/docs stop implying a mismatched `links.json` flow.
- CLI parse errors now preserve the actionable clap detail instead of truncating required-argument failures down to a pathless headline, `inspect select --output text` and `inspect slice --output text` now keep multiline preview structure so heading/list noise is visible before extraction, and the help/docs surfaces now teach that HTML output preserves the selected fragment rather than sanitizing it.
- Plain-text rendering now keeps table captions with their tables and drops more auxiliary footer chrome such as print-footers and support-bridge prompts from saved text output.
- `--output text`, `--output-file`, and bundled `selection.txt` artifacts now render HTML-valued extractions into readable plain text instead of dumping raw markup, so one extraction can produce both a faithful saved HTML fragment and a usable human text companion.
- Path-gated the `contributor-devcontainer` CI job so it fires only when devcontainer-relevant files actually change (`.devcontainer/`, the devcontainer lifecycle scripts, or `check.sh`); non-devcontainer PRs now skip the full Docker build-and-run cycle entirely, reducing typical PR wall-clock time significantly.
- Added a `devcontainer-changes` detection job that computes a git diff of the PR's changed files against the devcontainer trigger paths before the gate is evaluated; the aggregate `Check` required-status job now uses `if: always()` with explicit `${{ toJSON(needs.*.result) }}` failure detection so a correctly skipped `contributor-devcontainer` gate does not prevent `Check` from being reported or block merge — only a failed or cancelled gate prevents success.
- Raised the cross-platform Rust gate timeout budget to `150` minutes for Windows and `30` minutes for macOS so the Windows required-check lane can finish a cold `cargo nextest` build, dependency-policy check, and semver verification without expiring mid-run.
- Added a Windows Defender exclusion for the Cargo `target/` directory in `cross-platform-rust-gate` before any Cargo operations begin, eliminating antivirus scan overhead that otherwise scans every file write during compilation.
- Added a per-platform cache key (`cross-platform-${{ matrix.id }}`) to `Swatinem/rust-cache` in `cross-platform-rust-gate` so macOS and Windows build caches do not collide.
- Added `workflow_dispatch:` to the CI trigger so maintainers can manually rerun the aggregate `Check` against a branch when GitHub fails to attach the `pull_request` workflow on the initial PR open.
- Fixed Bash-4-only `mapfile` usage in `scripts/validate-devcontainer.sh` so the devcontainer validator runs correctly under stock macOS `/bin/bash` as well as GNU Bash installs.
- `./scripts/devcontainer-check.sh` now routes Cargo build artifacts into the contributor cache mount, mounts Git worktree metadata into release worktrees, and marks `/workspaces/htmlcut` as a safe Git directory before running `./check.sh`, so the non-root contributor container can complete the full maintainer gate in CI instead of failing on read-only target paths or Git ownership checks.
- CLI tests that exercise parentless output paths and bundle-path canonicalization now serialize working-directory changes through one shared guard and stage those writes inside tempdirs, so the contributor devcontainer gate no longer depends on the repository root being writable.
- `cargo xtask check` now fans the full Rust test gate out by package and discovered CLI test targets instead of invoking one workspace-wide `cargo nextest` inventory pass that can stall late in the maintainer gate on macOS.
- The maintainer release protocol now requires rerunning the full local gate from the `release/X.Y.Z` worktree before pushing any release-only follow-up fix discovered by CI, so the shipped release branch is revalidated on the exact branch payload instead of only on the original prep checkout.

### Changed
- Promoted the workspace to `8.0.0` because this slice intentionally breaks public `htmlcut-core` contracts: `ContractValueError` gains an explicit whitespace-rejection variant and `SchemaStability::Frozen` is removed from the live schema surface.
- The generic extraction contract now treats rendering as an output concern rather than a request sidecar: `ExtractionRequest` carries `output.rendering`, reusable extraction-definition files are now `htmlcut.extraction_definition@2`, and the generic request/result/report families advance to `htmlcut.extraction_request@5`, `htmlcut.extraction_result@6`, and `htmlcut.extraction_report@6`.
- `inspect source` now publishes two selector families instead of one ambiguous `content_candidates` list: `extraction_candidates` for cleaner saved fragments and `reading_candidates` for title-preserving review. The source-inspection schema families advance to `htmlcut.source_inspection_result@5` and `htmlcut.source_inspection_report@5`.
- The interop v1 published language is now owned cleanly inside `htmlcut_core::interop::v1` instead of borrowing core request/result types directly: selector text, delimiter boundaries, output selection, diagnostics, and byte ranges are now explicit interop-owned contracts.
- The interop v1 schema families now publish `htmlcut.plan@4`, `htmlcut.result@5`, and `htmlcut.error@2`; successful result documents now carry a top-level `output` contract plus per-match `output_value`, `text_output`, `selected_html_output`, `inner_html_output`, and `outer_html_output` fields, and the v1 output surface now includes `attribute`, `structured`, and `selected_html`.
- The schema and operation registries now use static slice inventories instead of heap-backed lazy vectors, and `cargo xtask check` runs its preflight Rust test subsets through `cargo nextest` instead of mixing test runners inside one workspace gate.
- Raised the declared workspace dependency floors to `clap 4.6.1` and `schemars 1.2.1`, matching the maintained lockfile and the versions exercised by the current gate.
- HTMLCut-owned HTTPS loading now uses the bundled `webpki-roots` trust set instead of the `platform-verifier` stack, removing a transitive dependency split that the maintained dependency policy rejects.

### Fixed
- URL HEAD-first preflight no longer retries GET after a hard connection failure, now accepts `text/xhtml+xml` as HTML, and the docs/help surfaces describe the narrower fallback behavior accurately.
- Slice markup warnings now use a quote-aware markup scanner instead of raw character backtracking, selector validation preserves parser error details, whitespace-padded attribute names fail validation up front, and stream size limits stop at the configured byte cap.
- CLI output/request-file/bundle flows now preserve diagnostics when file writes fail, bundle reports resolve fresh artifact paths to canonical absolute locations, and request-file writes no longer duplicate an incomplete second overwrite-policy check at the write site. Bundle-path fallback resolution now also normalizes `.` and `..` segments consistently across platforms, including Windows temp directories.
- Interop fallback errors now produce self-validating digests, `meta refresh` URL rewriting no longer reformats separators, and source-load failure metadata no longer pays an unnecessary heap allocation hop.
- The frozen `htmlcut-v1` acceptance corpus now covers selector and delimiter attribute output plus structured output, so the expanded interop contract is fixture-backed across more than the legacy text and HTML paths.
- Plain-text HTML rendering now preserves heading delineation, inline link targets, and nested-list indentation, and opt-in URL rewriting keeps empty self-links unchanged instead of forcing them to resolve against the page URL. Inspection now ignores empty headings while preserving accordion or button-backed heading titles, and structured text output renders table rows plus compact label-value rows as readable plain text instead of flattening them into loose paragraphs.
- `inspect source` now suggests likely content-root selectors, prioritizes sampled headings and link previews from the strongest content candidate on noisy pages, hides placeholder `#` and `javascript:` anchors from plain-text output, and the CLI help surface now teaches that `--output none` requires `--bundle`.
- Content-candidate ranking now prefers stable structural selectors over brittle exact-path fallbacks, recognizes title-bearing wrappers without regressing inner-article pages, and plain text rendering drops more utility chrome on noisy pages while preserving nested heading wrappers such as `<h1><div><div>…`.

## [7.0.0] - 2026-05-01

### Changed
- Promoted the workspace to `7.0.0` after removing the public `htmlcut_core::cli_contract` namespace and making `htmlcut_cli::contract` the sole maintained Rust surface for CLI command contracts, help documents, and command/help discovery helpers.
- `htmlcut-core` now owns only the generic operation catalog and execution contracts; `htmlcut-cli` owns the CLI command/help contract registry, packaged release README generation, and the release smoke flow that exercises one real extraction-plus-request replay from the shipped archive.
- Added a first-class contributor devcontainer on Ubuntu `24.04`, with committed lifecycle scripts that repair named Rust/Cargo cache volumes, bootstrap the pinned Rust toolchains and maintainer QA commands on first create, validate the real devcontainer-client path, and document the container workflow as the preferred contributor path.
- The canonical Linux maintainer gate now runs through that committed contributor devcontainer via a host-side `./scripts/devcontainer-check.sh` entrypoint instead of a second host-native Linux Rust gate.

### Fixed
- The docs contract now honors the repository rule that the root `README.md` is not AFAD-managed, so the root README remains in the maintained docs set for links/examples without being forced to carry AFAD metadata the repository explicitly forbids.
- Operation-ID documentation lint no longer hardcodes the current operation family matrix just to recognize valid catalog entries, and the coverage/reporting docs now describe the curated tracked executable module set honestly instead of overstating the covered surface.
- `cargo xtask --help` now prints a fuzz-smoke example target that belongs to the maintained inventory, the help tests validate those example commands against the live parser surface, invalid fuzz-target errors now fail before tool-preflight checks on every runner, and the release-preflight guide plus docs-test fixtures no longer teach the removed root-README metadata contract.
- The contributor devcontainer validation and bootstrap scripts now emit explicit phase progress, including Rust toolchain and cargo QA tool installation milestones, and CI now reuses the same named contributor volumes across validation and the full maintainer gate while skipping raw-image help probes that `./scripts/devcontainer-check.sh` already proves, so release-time container checks do not pay two unrelated cold-bootstrap costs or duplicate raw contributor-image command compiles.
- Repo-root `cargo run -- ...` now resolves to the public `htmlcut` CLI, `xtask` prints readable non-clap failures without Rust debug quoting, and the help/docs surfaces teach the hard-break overwrite rule for request files, output files, and bundle directories.
- The repo-root `./check.sh` entrypoint now shows Cargo compile progress instead of hiding startup work behind `--quiet`, so first-run maintainer gates and the contributor devcontainer path are observable while they warm a fresh build cache.
- The root README now renders its storefront artwork from the tagged tree rather than loading it from the moving `main` branch, so historical tag views keep their own release-facing art.
- The release preflight guide now pushes `release/X.Y.Z` without `-u`, so the disposable release worktree does not need to mutate the shared repository config just to publish the release branch.
- HTMLCut now refuses to replace existing `--emit-request-file`, `--output-file`, or `--bundle` targets unless `--overwrite` is explicit, while keeping parent-directory creation automatic for fresh paths.
- Contributor-container validation no longer depends on Docker access from inside the contributor shell, so the Ubuntu `24.04` devcontainer now follows a cleaner host-Docker/container-Rust split without mounted-socket permission failures.
- Contributor cargo-tool bootstrap now reads Cargo install metadata before probing binaries, so already-correct QA tool installs are reused on later devcontainer starts instead of being rebuilt unnecessarily.
- CI now validates the committed contributor devcontainer as its own job before the aggregate `Check` result reports success, and that Linux job now runs the full maintainer gate through the committed container instead of duplicating a separate host-native Rust path.

## [6.0.0] - 2026-04-29

### Changed
- Tightened the `htmlcut_core::interop::v1` surface with reusable `prepare_plan(...)` / `execute_validated_plan(...)` APIs, multi-match selection support, enforced size limits for preloaded HTML inputs, and canonical digest validation for interop result/error documents instead of relying on construction order.
- Removed the redundant `validate_plan(...)` preflight from `htmlcut_core::interop::v1`; `prepare_plan(...)` is now the sole maintained validated-plan entrypoint before execution.
- Structured CLI failures emitted on the JSON path now use a first-class `htmlcut.error_report` schema, and the published `htmlcut_cli` Rust surface now exports first-class `CliErrorCode` / `ErrorReportCode` types plus the error-report structs and schema constants alongside the existing command-report types.
- Source loading now exposes an explicit `--fetch-connect-timeout-ms` contract alongside the overall `--fetch-timeout-ms` budget instead of hiding a hardcoded 5-second connect ceiling under the total timeout flag.
- Operation catalog entries, CLI command contracts, and CLI help documents now derive from one canonical core-owned operation surface spec instead of three manually synchronized registries.
- The public schema registry now materializes JSON Schema documents through typed `SchemaExportError` results instead of aborting on serialization failures.
- Promoted the workspace to `6.0.0` to reflect the intentional `htmlcut-core` contract changes in interop validation, diagnostic typing, and regex/default contract cleanup.

### Fixed
- Corrected byte-size rendering and CLI/docs unit labels to use IEC `KiB`/`MiB`/`GiB`, removed the no-op default regex `u` flag from the core/interop pipeline, and refreshed the maintained docs around those public contracts.
- Selector extraction now rewrites URLs from one cloned parsed DOM instead of reparsing inner and outer HTML per selected match, the core/CLI catalog metadata gains live drift assertions for `rust_shape`/surface strings, and `cargo xtask check` now includes `cargo doc --workspace --no-deps` so broken public docs fail the maintainer gate.
- Ordered-list text extraction now preserves real list numbering including `start` / `reversed` / `li[value]` semantics, selector text extraction now preserves selected-node `img[alt]` and preformatted whitespace, slice extraction no longer parses a selected fragment just to emit `inner-html`, and JSON CLI error reports now carry typed code fields plus any captured `source_load_steps` trace.
- Core rendering no longer rescans or reallocates whole output buffers just to decide spacing or trim trailing blank lines, slice-match construction no longer relies on `expect(...)` panics for value/output invariants, and interop plan-digest failures now return structured internal errors instead of aborting.
- File and URL source loaders now take typed inputs from the dispatcher instead of relying on runtime `unreachable!` checks, schema-catalog validation now reports unknown schema refs instead of panicking, HTTP URL loading now caps the connect phase separately inside the existing fetch timeout budget, and the macOS/Windows CI gate now runs dependency freshness, advisory, and policy checks alongside formatting, clippy, and tests.
- Invalid selector and slice requests now fail during request validation before any source I/O or document parsing, and delimiter-pattern compilation is reused across the later extraction phase instead of being rediscovered only after the source is loaded.
- The maintained `cargo xtask` entrypoint and every executable release helper script now explain themselves through `--help`, the canonical `scripts/release-targets.sh` registry is directly inspectable from the shell, and the runnable namespace core example now prints a compact JSON summary instead of exiting silently.
- CLI JSON/report/request rendering now returns structured internal errors instead of aborting on serialization, slice-preview range formatting no longer reasserts required metadata with `expect(...)`, and the exported operation/schema registries now avoid production drift panics by deriving metadata from their maintained owners instead of runtime assertion shims.
- The release preflight and closeout guides now recognize GitHub CLI's current Dependabot author identity surface (`app/dependabot` as well as `dependabot[bot]`), so post-release dependency hygiene matches the live GitHub metadata instead of a stale login spelling.
- Release packaging and smoke helpers now keep their temporary staging roots alive for the full script lifetime, so standalone macOS release builds no longer abort after writing the artifact during the release-target smoke gate.
- Maintained CI now retries Rustup toolchain and target installation in the Rust and release-target smoke jobs, so transient upstream DNS/download failures no longer derail an otherwise healthy release candidate.
- The maintainer release registry helpers now normalize canonical shell-script paths before they source `scripts/release-targets.sh` through Bash, so temporary release-doc validation repos and the Windows cross-platform gate use the same release-target registry behavior instead of diverging on native `D:\\...` paths.
- Release-helper shell failures now report captured stderr in `xtask`, and the maintained Windows release gate no longer depends on a duplicate temp-repo shell-smoke that overlapped the stronger canonical registry coverage already present in the release test suite.
- Release tooling now normalizes Windows-style script paths before Bash sources the canonical release registry or sibling shell helpers, and docs-contract inventory checks now normalize repo-relative Markdown paths to forward slashes so temporary validation repos and Windows CI enforce the same release/documentation contract as Unix.

## [5.0.0] - 2026-04-24

### Changed
- Tightened the public `htmlcut_cli::run` library contract to return `std::io::Result<i32>`, updated the binary and maintainer tooling to surface writer failures explicitly, and revised the maintained CLI-library docs so broken pipes and other output-sink failures are no longer treated as silent success cases.
- Replaced the maintainer gate's filesystem-first inventory lookups with one maintained Git-backed worktree inventory for Markdown docs, shell scripts, and tracked coverage sources, removed the local-only ignore rule for the repository-root `AGENTS.md`, and added canonical metadata so the agent entry protocol is validated like the rest of the maintainer docs surface.
- Split the checked-in libFuzzer targets into a finite default Cargo mode plus an explicit `fuzzing` harness mode, updated `cargo xtask check`, `cargo xtask fuzz-smoke`, and the public maintainer docs around that contract, and synchronized the docs metadata gate with the canonical AFAD protocol version instead of a stale hardcoded literal.
- Made the repository-root `AGENTS.md` and the `.codex/` maintainer protocol directory explicit Git-tracked surfaces while keeping them out of shipped source archives through canonical `.gitattributes` `export-ignore` rules, and documented that archive contract in the release publishing guide.
- Narrowed the public `htmlcut-core` surface so CLI help documents, command registries, and related contract types now live under the explicit `htmlcut_core::cli_contract` namespace instead of being re-exported from the crate root, and promoted the workspace to `5.0.0` to reflect those intentional breaking changes.
- Made built-in HTTP(S) loading an explicit `htmlcut-core/http-client` feature, kept the published CLI opted in by default, and added a default-feature-free core test gate so fetch-free embedders keep a small dependency graph and a verified failure mode for URL requests.

### Fixed
- Catalog, schema, and inspection output modes now use a dedicated text/json-only type instead of relying on parser filtering plus `unreachable!` panic guardrails for impossible `html`/`none` variants.
- Broad verification commands such as `cargo test --workspace --all-targets --locked` now stay finite even with the maintained fuzz package in the workspace, while live fuzz-smoke and explicit fuzz compile-smoke still exercise the real libFuzzer harnesses.
- HEAD-first URL loading now treats GET as authoritative whenever HEAD fails or returns a non-success status, so HTMLCut no longer rejects servers that serve the page correctly but send `403` or similar responses to `HEAD`.
- The docs-contract parser now reports malformed `htmlcut inspect` examples as normal lint errors instead of panicking inside `xtask`.

## [4.4.1] - 2026-04-23

### Changed
- Tightened the maintainer release protocol so post-merge and closeout sync steps use explicit `git fetch ...` plus `git merge --ff-only origin/main` instead of implicit `git pull`, documented the required `main` branch-ownership handoff when a disposable release worktree is used, and added explicit cleanup for stale `release-prep/X.Y.Z` branches after the shipped history absorbs them.
- Filled the remaining documentation gaps around workspace topology and publication metadata by adding dedicated maintained guides for the `htmlcut_cli` library surface and the internal `htmlcut_tempdir` helper crate, plus explicit release-doc coverage for GitHub build-provenance attestations alongside the downloadable asset inventory.
- Recut the root `htmlcut --help` experience around a single-sourced package banner, workflow-first guidance, and reusable-request examples, while promoting the polished `HTMLCut` display name across human-facing version, catalog, and schema text banners without changing the machine JSON `tool` field. HTMLCut now also matches the maintained `ffhn` help/version contract by restoring clap's `help` subcommand and limiting `--version` to top-level use instead of accepting it under subcommands.
- Corrected the standalone-package release smoke script and the maintainer publishing protocol to verify the canonical `HTMLCut X.Y.Z` first version line, and added release-doc plus release-script regression tests so future help/version banner changes cannot silently break tag publication.

## [4.4.0] - 2026-04-23

### Changed
- Moved the checked-in `fuzz/` package into the main Cargo workspace, restored one shared `Cargo.lock`, and re-enabled root Cargo Dependabot updates now that the maintained libFuzzer targets no longer live behind a second lockfile and duplicate maintenance gates.
- Expanded the maintained `cargo deny` license allowlist to include `NCSA`, which keeps the first-class `libfuzzer-sys` workspace dependency enforceable by policy instead of having to hide fuzzing from the normal dependency gate.
- Localized the LLVM compiler requirement to the maintained `cargo xtask coverage` and `cargo xtask fuzz-smoke` flows, replacing the old repo-wide `CC=clang` override with explicit clang/clang++ preflight checks plus per-command toolchain injection where it is actually needed.
- Broke up the remaining core help-contract, slice-extraction, frozen interop execution, and oversized CLI contract-test god-files into focused modules so command help ownership, delimiter extraction, adapter compilation/projection/error mapping, and contract assertions no longer share multi-responsibility files.
- Broke up the remaining maintainer docs-contract runner, coverage gate, and CLI help-rendering god-files into focused modules so example parsing/runtime checks, coverage command/report/file tracking, and help caching/rendering no longer share single mixed-responsibility files.
- Broke the remaining `xtask` command-execution/preflight seam and oversized CLI preparation construction test into focused modules, so maintainer command launching, prerequisite detection, and raw-argv/builder/rendering assertions no longer live in mixed-responsibility files.
- Broke the remaining `xtask` gate-plan seam, CLI extraction-preparation seam, and CLI discovery rendering seam into focused modules, so gate assembly/path resolution/semver helpers, preview vs extraction preparation, and catalog vs schema text rendering no longer share mixed-responsibility files.
- Broke the remaining oversized CLI rendering, execution-path, request-file-builder, inspect, and parity test seams into focused modules, so rendering coverage, request-file loading/preparation, preview/error integration flows, and core-vs-CLI parity matrices no longer hide unrelated assertions inside a handful of monolithic test files.
- Broke the remaining oversized `htmlcut-core` catalog, document, extraction, source, and interop regression suites into focused modules, so core contract validation, document/source behavior, extractor coverage, and frozen interop assertions no longer share a few 500+ line test files.
- Replaced the generic CLI help-cache dispatchers and the empty `xtask::coverage` re-export shell with explicit façade accessors, so impossible `document.parse` help paths and zero-line module glue no longer survive inside the maintained public helper layer.
- Tightened the maintainer docs contract so `PATENTS.md` must stay aligned with the live `deny.toml` license allowlist, documented the actual docs-contract scope as “all maintained public Markdown except changelog,” and updated the release preflight guide to match the live GitHub conversation-resolution branch-protection rule.
- Default repository search now excludes the frozen `semver-baseline/` snapshot through `.ignore`, so normal symbol search stays on the maintained live tree unless a maintainer explicitly opts into baseline inspection.
- Added a maintained `cli_parse_error_surface` libFuzzer target plus balanced checked-in seeds so short fuzz-smoke runs cover the raw-argv error-format seam that previously drifted.

### Fixed
- Concrete fenced `htmlcut ...` examples in the maintained Markdown docs are now executed inside a fixture-backed temp sandbox instead of only being shell-split and clap-parsed, so broken request-file/output-file flows and other non-runnable examples now fail the docs contract.
- The maintained public Rust examples in `docs/architecture.md`, `docs/core.md`, `docs/interop-v1.md`, and `docs/schema.md` now run through `htmlcut-core` doctest harnesses, so those Markdown examples fail the normal workspace doc-test gate when they drift.
- Missing-argument CLI parse failures no longer switch to JSON just because a positional token is literally named `inspect`; only the real parsed inspect command path or an explicit structured output request can opt those failures into JSON formatting.
- Root README quick-start examples now start from an explicit demo page, and the documented request-file/output-file CLI flows are covered by integration smoke so those concrete examples stay runnable instead of only parsing.
- The Windows standalone ZIP PowerShell packaging fallback now writes forward-slash archive entry names instead of backslash-separated members, so future published ZIPs unpack cleanly with standard ZIP tooling outside Windows as well as with `Expand-Archive`.
- The Windows standalone ZIP PowerShell packaging fallback now preloads both compression assemblies before it opens the archive, so the native Windows release-target smoke job no longer fails on missing `ZipArchiveMode` type resolution during packaging.
- Windows standalone packaging and smoke verification now normalize temporary paths through the runner's real temp root and prefer bash-native ZIP extractors before the PowerShell fallback, so the Windows CI smoke job no longer depends on raw `/tmp` handling or `Expand-Archive` path translation matching Git Bash's extracted-package lookup.
- Repaired the shared release-shell helpers so `scripts/publish-github-release.sh` and related maintainer scripts no longer trip over caller-owned `readonly` variables when they resolve the repo root, workspace version, or release tag; the `xtask` release test suite now includes a regression smoke that exercises those helpers under the same readonly naming pattern that broke the `v4.3.0` publication job.
- `cargo xtask refresh-semver-baseline --git-ref ...` now strips test-only `dev-dependencies` tables from the released snapshot before it repackages `htmlcut-core`, so workspace-local maintainer helpers like `htmlcut-tempdir` do not break post-release semver baseline refresh.
- `cargo xtask check` now resolves the `cargo deny` target list from the canonical release-target registry and `deny.toml` now mirrors that shipped matrix, while the one remaining Windows-only build-time `getrandom 0.3` duplicate is tracked as an exact documented exception instead of hiding behind an incomplete policy graph.
- CLI help rendering and operation-preparation error paths no longer panic on missing core-owned CLI contract entries; they now surface internal CLI-contract errors and fall back to stable error text instead of aborting the process.

## [4.3.0] - 2026-04-22

### Changed
- Pinned the repository toolchain and published compiler contract to Rust `1.95.0`, updated the workspace and fuzz manifests to match, and aligned maintainer docs plus CI/release automation around that exact compiler version instead of an open-ended `stable` channel.
- Removed the stale exact `tempfile = "=3.15.0"` workspace pin and then replaced `tempfile` entirely with a tiny internal `htmlcut-tempdir` helper, which keeps the workspace on current direct dependency floors without violating the duplicate-crate ban in `cargo deny`; the maintained lockfiles were refreshed accordingly.
- `cargo xtask check` now reads the exact pinned toolchain contract from `rust-toolchain.toml` and fails fast with one actionable preflight message when the pinned compiler is missing, when `clippy`/`rustfmt` components are absent, or when those tool binaries are still broken despite rustup claiming they are installed.
- Removed the fake stable-toolchain `llvm-tools-preview` requirement from the checked-in bootstrap surface, and routed release-tag validation through one canonical shell helper instead of re-implementing the same tag/version checks across workflows and release scripts.
- Tightened the standalone `fuzz/` package from compile-smoke only into a first-class maintained surface by giving it an explicit Rust floor and lint policy, splitting the shared fuzz drivers into focused modules, and extending `cargo xtask check` to run fuzz-specific `fmt`, `clippy`, `outdated`, and `audit` checks.
- Refreshed both maintained lockfiles to `rustls-webpki 0.103.13`, clearing the current RustSec advisory in the default URL-fetch stack instead of leaving the newly failing audit gate behind.
- `cargo xtask check` now runs the full `xtask` library test suite, so manifest policy, release-target registry helpers, and other maintainer invariants are enforced by the gate instead of compiling unused beside it.
- The maintainer docs-contract now validates release target triples, workflow runner mappings, macOS deployment floor, and release asset names in the maintained docs against the canonical `scripts/release-targets.sh` registry.
- Corrected the fuzzing docs to the real working `cargo +nightly fuzz run --fuzz-dir fuzz ...` form after live verification showed the previously documented `--manifest-path` usage did not work with `cargo-fuzz 0.13.1`.
- Broke up the frozen `htmlcut_core::interop::v1` type-definition god-file into focused shared, plan, and result/error modules so the frozen interop contract stays easier to audit without mixing unrelated concerns into one monolith.
- Reorganized the maintainer release documentation into an overview plus focused preflight, publishing, and closeout guides so the release process is easier to audit and keep in sync with the scripts and workflows.
- The release protocol now explicitly covers dirty primary checkouts that already contain real unpublished release-candidate work: move that state onto a named prep branch first, then create the clean `release/X.Y.Z` worktree from that captured commit instead of guessing from a dirty `main`.
- Rewrote the maintained docs in current-state language, removing release-note phrasing like "now" from stable behavior descriptions and standardizing on the canonical `extraction-definition` terminology.
- Re-verified the concrete README command examples against the live CLI surface and refreshed the behavior notes to match the verified output, diagnostics, and release-asset workflows.
- The core-owned CLI contract catalog now records the real default stdout override for `--value inner-html` and `--value outer-html`, so generated help, `htmlcut catalog`, and the maintained CLI docs match live extraction behavior instead of silently implying text output.
- Corrected the documented slice regex-flag surface so generated help, catalog text, and the maintained CLI guide now include `U`, and explain that `g` is accepted for compatibility but ignored.
- Refreshed the root README install and quick-start examples so the maintained release-install commands use the current release version, create the target install directory explicitly, verify checksums portably on macOS/Linux, and present the reusable request-file flow in runnable order.
- Tightened the maintainer docs so developer setup no longer tells contributors to run the full gate twice, the release protocol now verifies the host-native standalone package instead of hardcoding Apple Silicon in the local post-release smoke step, and the architecture/core guides use more precise terminology around the frozen interop surface and non-exhaustive root exports.
- The coverage gate now derives its tracked module inventory from the live `htmlcut-core`, `htmlcut-cli`, and `xtask` source trees with an explicit declarative-only exclusion list, so future seam splits cannot silently fall outside the enforced 100% line-and-branch bar.
- Added `cargo xtask fuzz-smoke`, which stages each checked-in fuzz corpus into temporary scratch before running libFuzzer so short maintainer smoke campaigns no longer mutate the repository-owned seed corpora.
- `cargo xtask fuzz-smoke` now preflights nightly plus `cargo-fuzz` before it launches and forces the documented `CC=clang CXX=clang++` toolchain environment for fuzz-driver invocations on the maintained macOS path.
- Broke up the remaining `htmlcut-cli` parser and report god-files into focused `args`, `model`, and `prepare::reports` modules while preserving the public CLI contract and existing test matrix.
- Broke up the remaining `htmlcut-cli` request-building and inspection-rendering god-files into focused `prepare::build::{extraction,output,selection,source}`, `prepare::definition::{conflicts,loading}`, and `render::inspection::{preview,shared,source}` seams.
- Added focused regression tests for CLI suggestion recovery, core help-contract validation, result metadata accessors, schema-reference constructors, and xtask coverage inventory edge cases so the newly surfaced logic branches are asserted directly instead of being hidden behind broad integration coverage.
- `--max-bytes` parsing now uses exact decimal unit scaling and rejects values that do not resolve to a whole positive byte count instead of silently truncating fractional bytes.

## [4.2.1] - 2026-04-22

### Changed
- Reworked release assets into explicit, versioned source archives and platform packages: source snapshots are now published as `htmlcut-source-X.Y.Z.{zip,tar.gz}`, macOS/Linux packages are published as versioned `.tar.gz` archives, Windows is published as a versioned `.zip`, and the release now carries one `htmlcut-X.Y.Z-checksums.txt` manifest instead of per-asset checksum sidecars.
- Standalone release packages now include the platform binary together with `README.md`, `LICENSE`, `NOTICE`, and `PATENTS.md`, and the macOS/Linux package format preserves executable metadata through extraction instead of relying on a post-download `chmod`.
- The release workflow now follows a draft-first publication flow, generates provenance attestations for source archives, standalone packages, and the checksum manifest, and refuses to backfill missing assets into an already-published release.
- CI and release automation now smoke-test the extracted release packages themselves, so the required gate validates the actual shipped archives rather than only the compiled binary inside the build tree.
- Windows release ZIP creation and smoke verification moved onto native Windows ZIP handling and `Expand-Archive`-based verification, but the published archives still used backslash entry separators; a follow-up fix is required before those ZIPs are portable to non-Windows unzip tooling.
- Refreshed the README, platform-support doc, and maintainer release protocol to document binary-package install, clarify the maintained asset inventory, and explicitly distinguish HTMLCut-owned source archives from GitHub's auto-generated `Source code` links.

## [4.2.0] - 2026-04-20

### Added
- Added docs-contract lint to the maintainer gate so Markdown metadata/version drift and broken local links are caught automatically.
- Added docs-contract validation for concrete fenced `htmlcut ...` examples, so stale non-parsing command examples now fail the maintainer gate instead of surviving in Markdown.

### Changed
- The maintainer docs-contract now walks the maintained public Markdown tree recursively, skips hidden/internal/generated directories, requires retrieval `keywords` and `questions` in both frontmatter and HTML-comment metadata, and parses fenced concrete `htmlcut ...` examples with shell-compatible tokenization instead of a homegrown splitter.
- Broke up the remaining docs-contract and CLI-contract god-files into focused modules, and split the oversized CLI library test seams into thematic modules so contract behavior no longer hides inside multi-hundred-line monoliths.
- Broke up the remaining core request, source-loading, document-rendering, extraction-runtime, and frozen `htmlcut_core::interop::v1` god-files into focused modules so canonical contracts, adapter execution, and stable-JSON logic no longer share giant mixed-owner files.
- Tightened the canonical contract-lint and curated coverage proof so optional output defaults, output-less command contracts, and empty-target value restrictions are all asserted directly, and the frozen selector interop adapter now reuses one canonical `html` field read instead of re-reading the same structured payload slot twice.
- Removed the undocumented `htmlcut analyze` alias so the CLI keeps one canonical command surface: `catalog`, `schema`, `inspect`, `select`, and `slice`.
- Removed the stale `scripts/qa-gate.sh` duplicate entrypoint; `./check.sh` and `cargo xtask check` remain the maintained gate surfaces.
- Request-definition failures now keep recovery guidance across missing files, unsupported schema revisions, and strategy mismatches, always pointing back to the maintained extraction-definition schema plus the matching catalog contract.
- Subcommand help now renders conditional output-default overrides from the same canonical core-owned CLI contract registry that already owns modes, notes, and examples.
- Root `htmlcut --help` examples and command-count wording now derive from maintained command/example data instead of a separate hardcoded help-only copy.
- `htmlcut-core` now also owns the canonical help documents for root discovery, non-operation commands, and CLI-visible operations, so `htmlcut-cli` renders help summaries and analysis prose from the same contract owner that already owns modes, notes, examples, and command paths.
- `htmlcut-cli` now parses the core-owned choice domains for match, value, output, pattern, whitespace, and fetch-preflight modes directly instead of keeping duplicate local enums that had to be mapped back into `htmlcut-core`.
- Broke up the remaining CLI preparation and execution god-files into focused submodules so request building, raw-arg heuristics, and output/request-file I/O no longer hide inside single multi-domain files.
- The curated 100% coverage gate now follows the live executable module layout after the seam splits, including the frozen interop adapter and the refactored core engine modules, instead of silently tracking deleted monolith paths.
- Contract-lint now renders the real clap help, catalog/schema text summaries, and representative recovery errors and fails if any of those user-facing surfaces mention operation IDs or schema names that are not registered in `htmlcut-core`.

### Fixed
- `cargo xtask check` now honors `CARGO_TARGET_DIR` consistently for the dist-binary smoke step and semver-check scratch cleanup, so clean `/tmp` gate runs no longer fail after a successful optimized build.

## [4.1.0] - 2026-04-19

### Added
- Added a core-owned `OperationCliContract` registry so operation command paths, mode inventories, defaults, parameter rules, notes, and examples now have one canonical owner in `htmlcut-core`.
- Added a typed `DiagnosticCode` registry in `htmlcut-core` and switched CLI error classification plus interop error mapping onto that canonical code owner.
- Added explicit contract-lint coverage for operation catalogs, CLI parser enums, command examples, and help/schema discovery examples, plus a checked-in `./check.sh` maintainer entrypoint that runs the full xtask gate.
- Added contract-lint coverage that parses the real clap surfaces and fails if command names or applied default values drift away from the canonical core-owned CLI contract.
- Added checked-in fuzz seed corpora for all maintained cargo-fuzz targets so local fuzz smoke runs start from known balanced cases instead of an empty corpus.
- Added structured `SourceMetadata.load_steps` traces for successful source loads, including visible HEAD-to-GET fallback reporting for URL inputs.
- Added exact `matched_start` and `matched_end` metadata to delimiter-pair matches so slice previews and structured outputs can show what was actually consumed.
- Added `--emit-request-file <PATH>` across `select`, `slice`, `inspect select`, and `inspect slice`, so inline request discovery can be promoted into a reusable normalized `ExtractionDefinition` without hand-authoring JSON.
- Added nearest-match and available-version suggestions for unknown catalog operation IDs and schema lookups.
- Added a dedicated `SLICE_SPLITS_MARKUP` warning when selected slice ranges appear to start or end inside HTML markup.

### Changed
- The release protocol now treats open Dependabot PRs as first-class release hygiene. Release-time pre-flight now requires explicitly identifying open Dependabot work, and after the public release is verified each Dependabot PR must be merged, closed, or consciously kept open with a stated reason; stale automation branches are no longer acceptable release leftovers.
- `htmlcut catalog`, CLI command-name normalization, and schema/help discovery examples now render from the same canonical core registries instead of duplicating operation IDs, schema names, mode strings, and default values in `htmlcut-cli`.
- `htmlcut catalog --output text` now renders every operation in detail instead of reserving the rich contract view for filtered single-operation output.
- `htmlcut catalog --output text` and `htmlcut schema --output text` now start with a short registry summary plus the exact follow-up JSON command to inspect one entry precisely.
- invalid `--request-file` definitions now produce self-recovery guidance that points directly at the extraction-definition schema and the matching catalog entry.
- request-file deserialization failures now report the failing JSON path instead of only surfacing a pathless serde error.
- successful `inspect` and extraction runs now surface source-load traces through verbose stderr, and `inspect source --output text` prints the same load trace inline.
- successful `catalog`, `schema`, extraction, and inspection runs now acknowledge `--output-file` writes in verbose stderr, and extraction/preview commands do the same for `--emit-request-file`.
- CLI execution now emits normal stderr diagnostics before the final successful file-write acknowledgement, so verbose output preserves real execution order.
- Refreshed the workspace lockfile to `rustls-webpki 0.103.12`, clearing the current RustSec advisories in the default URL-fetch stack.

### Fixed
- Failed URL inspections and extractions no longer drop the attempted HEAD/GET trace; the structured report and human stderr path now preserve the load steps that led to the failure.
- Human error output now replays preserved source-load traces instead of reducing URL load failures to a single line.

## [4.0.1] - 2026-04-14

### Changed
- Updated the pinned GitHub Actions SHAs for `Swatinem/rust-cache`, `taiki-e/install-action`, and `actions/upload-artifact`.
- Bumped `sha2` to `0.11.0` and refreshed both the workspace and fuzz lockfiles together.

### Fixed
- Stopped cargo Dependabot from opening incomplete PRs for this dual-lockfile repository; Cargo dependency refreshes are now maintainer-run so `Cargo.lock` and `fuzz/Cargo.lock` stay in sync with the maintained `--locked` fuzz gate.
- URL inputs using the default `head-first` preflight now fall back to GET when a server rejects HEAD or breaks the HEAD exchange, instead of failing before the real fetch can run.

## [4.0.0] - 2026-04-14

### Added
- Added property-based interop regression coverage for canonical stable JSON ordering, digest determinism, self-digest exclusion, and stable JSON round-trips across `htmlcut_core::interop::v1` plan, result, and error documents.
- Added a checked-in `fuzz/` package with libFuzzer targets for decoded HTML parsing, selector extraction, delimiter-boundary extraction, and frozen interop request building through public `htmlcut-core` surfaces.
- Added a first-class `htmlcut.extraction_definition@1` schema plus `htmlcut_core::ExtractionDefinition` for reusable serialized extraction runs.
- Added CLI support for `--request-file <PATH>` across `select`, `slice`, `inspect select`, and `inspect slice`.
- Added `--output-file <PATH>` so callers can write exactly the stdout payload to one file without bundle scaffolding.
- Added URL `HEAD` preflight with a `--fetch-preflight head-first|get-only` escape hatch for servers that do not tolerate HEAD.
- Added a runnable core example at `crates/htmlcut-core/examples/reusable_extraction_definition.rs`.
- Added a focused maintainer versioning policy doc plus a repo-root contributing guide so release, semver-baseline, frozen-interop, fixture-update, and documentation-sync rules no longer depend on oral history.

### Changed
- Renamed the interop module from `htmlcut_core::interop::ffhn_v1` to `htmlcut_core::interop::v1`, removing all consumer-specific type and constant prefixes (`Ffhn*`, `FFHN_*`). The interop profile identifier changed from `ffhn-htmlcut-v1` to `htmlcut-v1`, and the three frozen JSON schema names changed from `htmlcut.ffhn_plan`, `htmlcut.ffhn_result`, `htmlcut.ffhn_error` to `htmlcut.plan`, `htmlcut.result`, `htmlcut.error`.
- Reduced the `htmlcut-core` crate-root surface to the stable high-level API and moved detailed request/result contract types behind `htmlcut_core::request` and `htmlcut_core::result` namespaces.
- Renamed the ambiguous `Html` extraction value mode to `InnerHtml` across Rust contracts, CLI parsing, catalog/schema output, and user-facing docs. The CLI spelling is now `--value inner-html`.
- Bumped the versioned request/result and CLI report schema revisions so the serialized contract change is explicit, and documented the structured-match metadata union emitted by extraction results.
- Taught `cargo xtask refresh-semver-baseline` to refresh the semver baseline from an explicit published Git ref instead of repackaging the live worktree, preventing unreleased API drift from contaminating future semver checks.
- Added an early coverage preflight that stops immediately when nightly or `llvm-tools-preview` is missing, and documented the 100% curated line-and-branch coverage policy as an intentional contract.
- Expanded URL rewriting to cover structured URL-bearing HTML attributes and tokens including `srcset`, `poster`, `action`, `formaction`, `ping`, and `meta refresh`.
- Expanded plain-text rendering for document-shaped HTML so inline `code`, blockquotes, and definition lists preserve more readable structure.
- Structured extraction values now carry collection context fields (`matchIndex`, `matchCount`, `candidateIndex`, `candidateCount`) alongside the surrounding report stats.
- `htmlcut --version` now prints the engine identity, schema profile, and repository metadata, and `--quiet` now suppresses non-fatal stderr diagnostics on successful runs.

### Fixed
- Corrected the pinned `Swatinem/rust-cache` workflow SHA so the pinned commit and `v2.9.1` comment now describe the same upstream action release.
- Fixed the `cargo xtask` semver-mode decision so release preparation follows the checked-in semver baseline rather than the presence of a release heading in `changelog.md`, which keeps the documented release flow and the gate aligned.

## [3.0.0] - 2026-04-05

### Changed
- Overhauled and rebuilt HTMLCut in Rust, with explicit architectural provisions introduced specifically for [`FFHN`](https://github.com/resoltico/ffhn) interop.

## [2.0.0] - 2026-03-18

### Changed
- Rebuilt HTMLCut around a stdout-first CLI contract: `htmlcut <input> --from ... --to ...`.
- Replaced the old write-files-by-default workflow with explicit output formats: `text`, `html`, and `json`.
- Added deterministic bundle output via `--bundle`, which writes `selection.html`, `selection.txt`, and `report.json`.
- Switched long option names to hyphenated forms such as `--base-url`, `--max-bytes`, and `--fetch-timeout-ms`.
- Made `--all` strict: a trailing unmatched start delimiter is now a hard extraction error instead of a silent partial success.
- Simplified the extraction engine to operate behind a bounded input-size guard rather than maintaining the previous streaming state machine.
- Rebuilt HTML-to-text rendering and relative-URL rewriting around `parse5` instead of a large handwritten entity/HTML formatter module.
- Upgraded the `parse5` integration to be document-aware, using full-document parsing for complete pages and `serializeOuter()` for `html`/`head`/`body` root fragments so structure is preserved during URL rewriting.
- Improved structural text rendering for lists and tables by using parsed HTML attributes and sectioning more faithfully: ordered lists now respect `start`, `reversed`, `type`, and `li[value]`, while tables now preserve captions, header rows, basic span structure, and aligned column output.
- Improved plain-text document readability by rendering `h1` and `h2` as underlined headings and by preserving block structure through wrapper-heavy markup instead of flattening nested headings, paragraphs, and lists into single walls of text.
- Improved agent-facing surfaces by adding `--format none` for file-only workflows and by exposing `documentTitle` in the JSON report and bundle report.

### Fixed
- Removed duplicate stderr lines on CLI usage failures where a wrapped `cause` had the same message as the top-level error.
- Stopped full-document text rendering from leaking `<head>` content into plain-text output.
- Stopped URL rewriting from collapsing complete HTML documents or special root fragments into flattened fragment output.

### Removed
- Removed SQLite history tracking and the warning-suppression layer that existed only to support it.
- Removed timestamped implicit output files and the old `--stdout`, `--json`, `--quiet`, `--track`, and `--history` workflow.
- Removed backwards compatibility with the `1.x` flag surface and file-writing behavior.

## [1.1.0] - 2026-03-05

### Added
- `--base` (`-b`) CLI option to manually override the base URL used when resolving relative links.
- `--stdout` (`-O`) CLI option to stream extracted text directly to stdout instead of writing files, enabling shell pipelines.
- `--json` CLI option to stream a structured JSON array of `{html, text}` objects to stdout, optimized for programmatic consumption by scripts and AI agents.
- `--quiet` (`-q`) CLI option to suppress all diagnostic log output, ensuring stdout contains only the data payload.

### Changed
- **Breaking:** Diagnostic output (`✓ Successfully extracted…`, `→ path/to/file`) is now written to **stderr** instead of stdout. Stdout is now strictly the data payload — reserved for `--stdout` / `--json` output. This follows standard Unix convention and means all diagnostic messages are invisible to pipes and tools like `jq` by default, without needing `--quiet`.
- Relative links (`href` and `src`) in extracted HTML and TXT outputs are now automatically resolved and expanded into absolute URLs based on the source location of the input or the `--base` override.

### Fixed
- Improved `toPlainText` output formatting by fixing a bug where blank lines between block elements wouldn't collapse properly if there were intermediate structural spaces in the HTML sequence.

## [1.0.0] - 2026-03-02

### Added
- Initial release.
