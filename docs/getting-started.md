---
afad: "4.0"
version: "17.0.0"
domain: SETUP
updated: "2026-10-03"
route:
  keywords: [getting started, quick start, install, release package, cargo install, first extraction, saved run]
  questions: ["how do I install HTMLCut?", "how do I try HTMLCut on a sample page?", "how do I save a reusable extraction run?"]
  related: [../README.md, cli.md, platform-support.md, core.md, schema.md]
---

# Getting started

Use a completed UTF-8 HTML snapshot and an explicit selector. Install the native published package for the exact chosen version from [GitHub Releases](https://github.com/resoltico/HTMLCut/releases), or build this checkout with its pinned toolchain:

```sh
cargo install --path crates/htmlcut-cli --locked
htmlcut describe
htmlcut describe extract
htmlcut extract --file page.html --css article --projection markdown --raw
htmlcut extract --file page.html --css 'article a' --attribute href --match all
htmlcut inspect --file page.html --css article
```

Default output is a JSON array and default selection requires exactly one match. Use a plan file for records, guards, exclusions and transforms. Present empty values are valid; missing required nodes/attributes fail. `--raw` selects one flat string without a final LF.

Save actual source and execution with `--bundle snapshot.htmlcut.tar`, then recompute with `htmlcut run snapshot.htmlcut.tar`. Move the bundle freely; it does not depend on original paths. `--receipt FILE` is the smaller alternative when only evidence is needed. It is mutually exclusive with bundle output. Output replacement requires `--overwrite`.

Callers acquire/render/decode source first. HTMLCut accepts file/stdin UTF-8 and does not fetch URLs, execute JavaScript, use sessions or crawl. The current candidate is 17.0.0; a workspace version is not proof that matching release assets are published. See [CLI](cli.md), [Core](core.md), [Schemas](schema.md), and [Platform Support](platform-support.md).

Select an exact published release before using download/install snippets. Shell operators can set `VERSION=17.0.0`; PowerShell operators can set `$Version = "17.0.0"` after that version is actually published. The release list, not these literals, establishes availability.
