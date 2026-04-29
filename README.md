<!--
AFAD:
  afad: "4.0"
  version: "6.0.0"
  domain: FRONTDOOR
  updated: "2026-04-29"
RETRIEVAL_HINTS:
  keywords: [htmlcut, html extraction, cli, selectors, slice extraction, request files, html inspection]
  questions: [what is HTMLCut?, how do I extract values from HTML files or URLs?, how do I save a reusable HTMLCut request?]
  related: [docs/getting-started.md, docs/cli.md, docs/core.md, docs/interop-v1.md]
-->

# HTMLCut — repeatable HTML extraction from files, URLs, and stdin

HTMLCut is a command-line tool for people who keep pulling links, text, and fragments out of HTML,
so they can save the step once and run it again instead of rebuilding the same command each time.

When the same title, link, or snippet keeps sending you back to a page, HTMLCut lets you check the
match, save the request as a file, and run it again later instead of copying the value by hand or
piecing the shell command together again.

- Pick out the title, link, or fragment you actually need
- Check the match before you wire it into a script
- Save the request as a file and run the same step again later
- Use it when the HTML is already in hand

[Try one sample page](docs/getting-started.md) ·
[Get the release](https://github.com/resoltico/HTMLCut/releases) ·
[See the command guide](docs/cli.md)

## Save It Once, Use It Again

Here is the moment the repeated step becomes something you can keep:

```bash
htmlcut select ./page.html \
  --css 'article a.more' \
  --value attribute \
  --attribute href \
  --emit-request-file ./article-links.json
htmlcut select --request-file ./article-links.json
```

The first command saves the extraction as a file while the page is still warm. The second runs
the same step again later — same selector, same options — without rebuilding it from scratch.

## Where It Fits

HTMLCut is for recurring page work: pulling the same title, price, link, or fragment from pages
you return to repeatedly. It fits scripts, agent workflows, and anything smaller than full browser
automation.

Skip it when:
- A page only becomes useful after JavaScript runs — render it first, then hand the HTML to HTMLCut
- You need a full browser session, login flow, or crawling — let that tool handle the session; HTMLCut handles the extraction afterward
- A one-off copy-paste is faster than saving and running a request file

Rust developers can embed `htmlcut-core` directly — see [docs/core.md](docs/core.md) and [docs/interop-v1.md](docs/interop-v1.md).

## Install

Prebuilt packages for macOS, Linux, and Windows at [Releases](https://github.com/resoltico/HTMLCut/releases).

From source:

```bash
cargo build --locked -p htmlcut-cli --bin htmlcut
```

- [Getting started — install to first saved request](docs/getting-started.md)
- [Command guide](docs/cli.md)

## What You Can Check

- The sample page and example commands are tested with each release
- Releases are public and checksummed
- MIT-licensed

## Legal

HTMLCut is released under the [MIT License](LICENSE). See [NOTICE](NOTICE) and
[PATENTS](PATENTS.md) for the other legal files.
