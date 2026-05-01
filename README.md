[![HTMLCut Art](images/HTMLCut.png)](https://github.com/resoltico/HTMLCut)

[![Release](https://img.shields.io/github/v/release/resoltico/HTMLCut?label=release)](https://github.com/resoltico/HTMLCut/releases)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-macOS%20%7C%20Linux%20%7C%20Windows-lightgrey.svg)](docs/platform-support.md)

# HTMLCut — repeatable HTML extraction from files, URLs, and stdin

HTMLCut pulls the exact piece you need out of any web page or HTML file — a title, a price, a link,
a block of text. Point it at the right element and it extracts like a clean shot: nothing you didn't
ask for, nothing left behind.

Every page you return to for the same fragment is a page you've been brewing from scratch. Save the
extraction once; pour the same result whenever you need it.

- Pull text, links, or any piece of a page a CSS selector can reach
- Cut out a section between any two strings or patterns
- Preview the pour before it runs
- Save the extraction as a file; replay it unchanged, any time
- Collect results into a folder when you're working through many pages

[Getting started](docs/getting-started.md) · [Command guide](docs/cli.md)

## Brew Once, Pour Again

```bash
htmlcut select ./page.html \
  --css 'article a.more' \
  --value attribute \
  --attribute href \
  --emit-request-file ./article-links.json \
  --overwrite

htmlcut select --request-file ./article-links.json
```

The first command captures the extraction while the page is still warm. The second pours the same
result — same selector, same options — without rebuilding a thing.

## Where It Fits

The recurring page with something worth pulling: a price you track, a headline you monitor, a link
you always need. Works on any HTML already in hand — a saved file, a live URL, or passed from
another tool.

## Get It

[Download for macOS, Linux, or Windows →](https://github.com/resoltico/HTMLCut/releases)

New here? [Getting started](docs/getting-started.md) walks you from install to first saved
extraction. The full [command guide](docs/cli.md) covers everything after that.

## Legal

HTMLCut is released under the [MIT License](LICENSE). See [NOTICE](NOTICE) and
[PATENTS](PATENTS.md) for the remaining legal files.
