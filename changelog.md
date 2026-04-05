# Changelog

Notable changes to this project are documented in this file. The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
