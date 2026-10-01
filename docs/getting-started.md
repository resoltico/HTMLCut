---
afad: "4.0"
version: "15.0.0"
domain: SETUP
updated: "2026-10-01"
route:
  keywords: [getting started, quick start, install, release package, cargo install, first extraction, saved run]
  questions: ["how do I install HTMLCut?", "how do I try HTMLCut on a sample page?", "how do I save a reusable extraction run?"]
  related: [../README.md, cli.md, platform-support.md, core.md, schema.md]
---

# Getting started

Choose a saved HTML snapshot and an explicit selector. The default is literal descendant text with exactly one match and compact JSON.

```sh
htmlcut describe
htmlcut describe extract
htmlcut extract --file page.html --css article
htmlcut extract --file page.html --css 'article a.more' --projection attribute --attribute href --raw
htmlcut extract --file page.html --css article --projection document_text --raw
htmlcut inspect --file page.html --page-size 5
```

Use a plan file for guards, exclusions and transforms. Source slices are byte exact and never automatically become DOM fragments. Acquire/render browser snapshots in caller code when JavaScript-created content is needed. HTMLCut does not execute scripts or infer business meaning.

See [CLI](cli.md), [Core](core.md), [Schemas](schema.md) and [Platform support](platform-support.md).

For release packages, select the exact published version from [GitHub Releases](https://github.com/resoltico/HTMLCut/releases). The candidate workspace version is not a claim that its assets are published. Shell download scripts use `VERSION=15.0.0`; PowerShell scripts use `$Version = "15.0.0"` after that release is authorized and published.
