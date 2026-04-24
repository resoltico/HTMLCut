# Changelog

Notable changes to this project are documented in this file. The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [5.0.0] - 2026-04-24

### Changed
- Tightened the public `htmlcut_cli::run` library contract to return `std::io::Result<i32>`,
  updated the binary and maintainer tooling to surface writer failures explicitly, and revised the
  maintained CLI-library docs so broken pipes and other output-sink failures are no longer treated
  as silent success cases.
- Replaced the maintainer gate's filesystem-first inventory lookups with one maintained Git-backed
  worktree inventory for Markdown docs, shell scripts, and tracked coverage sources, removed the
  local-only ignore rule for the repository-root `AGENTS.md`, and added canonical metadata so the
  agent entry protocol is validated like the rest of the maintainer docs surface.
- Split the checked-in libFuzzer targets into a finite default Cargo mode plus an explicit
  `fuzzing` harness mode, updated `cargo xtask check`, `cargo xtask fuzz-smoke`, and the public
  maintainer docs around that contract, and synchronized the docs metadata gate with the canonical
  AFAD protocol version instead of a stale hardcoded literal.
- Made the repository-root `AGENTS.md` and the `.codex/` maintainer protocol directory explicit
  Git-tracked surfaces while keeping them out of shipped source archives through canonical
  `.gitattributes` `export-ignore` rules, and documented that archive contract in the release
  publishing guide.
- Narrowed the public `htmlcut-core` surface so CLI help documents, command registries, and related
  contract types now live under the explicit `htmlcut_core::cli_contract` namespace instead of
  being re-exported from the crate root, and promoted the workspace to `5.0.0` to reflect those
  intentional breaking changes.
- Made built-in HTTP(S) loading an explicit `htmlcut-core/http-client` feature, kept the published
  CLI opted in by default, and added a default-feature-free core test gate so fetch-free embedders
  keep a small dependency graph and a verified failure mode for URL requests.

### Fixed
- Catalog, schema, and inspection output modes now use a dedicated text/json-only type instead of
  relying on parser filtering plus `unreachable!` panic guardrails for impossible `html`/`none`
  variants.
- Broad verification commands such as `cargo test --workspace --all-targets --locked` now stay
  finite even with the maintained fuzz package in the workspace, while live fuzz-smoke and explicit
  fuzz compile-smoke still exercise the real libFuzzer harnesses.
- HEAD-first URL loading now treats GET as authoritative whenever HEAD fails or returns a
  non-success status, so HTMLCut no longer rejects servers that serve the page correctly but send
  `403` or similar responses to `HEAD`.
- The docs-contract parser now reports malformed `htmlcut inspect` examples as normal lint errors
  instead of panicking inside `xtask`.

## [4.4.1] - 2026-04-23

### Changed
- Tightened the maintainer release protocol so post-merge and closeout sync steps use explicit
  `git fetch ...` plus `git merge --ff-only origin/main` instead of implicit `git pull`, documented
  the required `main` branch-ownership handoff when a disposable release worktree is used, and
  added explicit cleanup for stale `release-prep/X.Y.Z` branches after the shipped history absorbs
  them.
- Filled the remaining documentation gaps around workspace topology and publication metadata by
  adding dedicated maintained guides for the `htmlcut_cli` library surface and the internal
  `htmlcut_tempdir` helper crate, plus explicit release-doc coverage for GitHub build-provenance
  attestations alongside the downloadable asset inventory.
- Recut the root `htmlcut --help` experience around a single-sourced package banner, workflow-first
  guidance, and reusable-request examples, while promoting the polished `HTMLCut` display name
  across human-facing version, catalog, and schema text banners without changing the machine JSON
  `tool` field. HTMLCut now also matches the maintained `ffhn` help/version contract by restoring
  clap's `help` subcommand and limiting `--version` to top-level use instead of accepting it under
  subcommands.
- Corrected the standalone-package release smoke script and the maintainer publishing protocol to
  verify the canonical `HTMLCut X.Y.Z` first version line, and added release-doc plus release-script
  regression tests so future help/version banner changes cannot silently break tag publication.

## [4.4.0] - 2026-04-23

### Changed
- Moved the checked-in `fuzz/` package into the main Cargo workspace, restored one shared
  `Cargo.lock`, and re-enabled root Cargo Dependabot updates now that the maintained libFuzzer
  targets no longer live behind a second lockfile and duplicate maintenance gates.
- Expanded the maintained `cargo deny` license allowlist to include `NCSA`, which keeps the
  first-class `libfuzzer-sys` workspace dependency enforceable by policy instead of having to hide
  fuzzing from the normal dependency gate.
- Localized the LLVM compiler requirement to the maintained `cargo xtask coverage` and
  `cargo xtask fuzz-smoke` flows, replacing the old repo-wide `CC=clang` override with explicit
  clang/clang++ preflight checks plus per-command toolchain injection where it is actually needed.
- Broke up the remaining core help-contract, slice-extraction, frozen interop execution, and
  oversized CLI contract-test god-files into focused modules so command help ownership, delimiter
  extraction, adapter compilation/projection/error mapping, and contract assertions no longer share
  multi-responsibility files.
- Broke up the remaining maintainer docs-contract runner, coverage gate, and CLI help-rendering
  god-files into focused modules so example parsing/runtime checks, coverage command/report/file
  tracking, and help caching/rendering no longer share single mixed-responsibility files.
- Broke the remaining `xtask` command-execution/preflight seam and oversized CLI preparation
  construction test into focused modules, so maintainer command launching, prerequisite detection,
  and raw-argv/builder/rendering assertions no longer live in mixed-responsibility files.
- Broke the remaining `xtask` gate-plan seam, CLI extraction-preparation seam, and CLI discovery
  rendering seam into focused modules, so gate assembly/path resolution/semver helpers, preview vs
  extraction preparation, and catalog vs schema text rendering no longer share mixed-responsibility
  files.
- Broke the remaining oversized CLI rendering, execution-path, request-file-builder, inspect, and
  parity test seams into focused modules, so rendering coverage, request-file loading/preparation,
  preview/error integration flows, and core-vs-CLI parity matrices no longer hide unrelated
  assertions inside a handful of monolithic test files.
- Broke the remaining oversized `htmlcut-core` catalog, document, extraction, source, and
  interop regression suites into focused modules, so core contract validation, document/source
  behavior, extractor coverage, and frozen interop assertions no longer share a few 500+ line
  test files.
- Replaced the generic CLI help-cache dispatchers and the empty `xtask::coverage` re-export shell
  with explicit façade accessors, so impossible `document.parse` help paths and zero-line module
  glue no longer survive inside the maintained public helper layer.
- Tightened the maintainer docs contract so `PATENTS.md` must stay aligned with the live
  `deny.toml` license allowlist, documented the actual docs-contract scope as “all maintained
  public Markdown except changelog,” and updated the release preflight guide to match the live
  GitHub conversation-resolution branch-protection rule.
- Default repository search now excludes the frozen `semver-baseline/` snapshot through `.ignore`,
  so normal symbol search stays on the maintained live tree unless a maintainer explicitly opts
  into baseline inspection.
- Added a maintained `cli_parse_error_surface` libFuzzer target plus balanced checked-in seeds so
  short fuzz-smoke runs cover the raw-argv error-format seam that previously drifted.

### Fixed
- Concrete fenced `htmlcut ...` examples in the maintained Markdown docs are now executed inside a
  fixture-backed temp sandbox instead of only being shell-split and clap-parsed, so broken
  request-file/output-file flows and other non-runnable examples now fail the docs contract.
- The maintained public Rust examples in `docs/architecture.md`, `docs/core.md`,
  `docs/interop-v1.md`, and `docs/schema.md` now run through `htmlcut-core` doctest harnesses, so
  those Markdown examples fail the normal workspace doc-test gate when they drift.
- Missing-argument CLI parse failures no longer switch to JSON just because a positional token is
  literally named `inspect`; only the real parsed inspect command path or an explicit structured
  output request can opt those failures into JSON formatting.
- Root README quick-start examples now start from an explicit demo page, and the documented
  request-file/output-file CLI flows are covered by integration smoke so those concrete examples
  stay runnable instead of only parsing.
- The Windows standalone ZIP PowerShell packaging fallback now writes forward-slash archive entry
  names instead of backslash-separated members, so future published ZIPs unpack cleanly with
  standard ZIP tooling outside Windows as well as with `Expand-Archive`.
- The Windows standalone ZIP PowerShell packaging fallback now preloads both compression
  assemblies before it opens the archive, so the native Windows release-target smoke job no
  longer fails on missing `ZipArchiveMode` type resolution during packaging.
- Windows standalone packaging and smoke verification now normalize temporary paths through the
  runner's real temp root and prefer bash-native ZIP extractors before the PowerShell fallback, so
  the Windows CI smoke job no longer depends on raw `/tmp` handling or `Expand-Archive` path
  translation matching Git Bash's extracted-package lookup.
- Repaired the shared release-shell helpers so `scripts/publish-github-release.sh` and related
  maintainer scripts no longer trip over caller-owned `readonly` variables when they resolve the
  repo root, workspace version, or release tag; the `xtask` release test suite now includes a
  regression smoke that exercises those helpers under the same readonly naming pattern that broke
  the `v4.3.0` publication job.
- `cargo xtask refresh-semver-baseline --git-ref ...` now strips test-only `dev-dependencies` tables
  from the released snapshot before it repackages `htmlcut-core`, so workspace-local maintainer
  helpers like `htmlcut-tempdir` do not break post-release semver baseline refresh.
- `cargo xtask check` now resolves the `cargo deny` target list from the canonical release-target
  registry and `deny.toml` now mirrors that shipped matrix, while the one remaining Windows-only
  build-time `getrandom 0.3` duplicate is tracked as an exact documented exception instead of
  hiding behind an incomplete policy graph.
- CLI help rendering and operation-preparation error paths no longer panic on missing core-owned
  CLI contract entries; they now surface internal CLI-contract errors and fall back to stable error
  text instead of aborting the process.

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
- Windows release ZIP creation and smoke verification moved onto native Windows ZIP handling and
  `Expand-Archive`-based verification, but the published archives still used backslash entry
  separators; a follow-up fix is required before those ZIPs are portable to non-Windows unzip
  tooling.
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
- The release protocol now treats open Dependabot PRs as first-class release hygiene. Release-time
  pre-flight now requires explicitly identifying open Dependabot work, and after the public
  release is verified each Dependabot PR must be merged, closed, or consciously kept open with a
  stated reason; stale automation branches are no longer acceptable release leftovers.
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
