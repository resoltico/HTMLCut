# Changelog

Notable changes to this project are documented in this file. The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
