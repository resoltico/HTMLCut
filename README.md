<!--
AFAD:
  afad: "3.5"
  version: "4.4.1"
  domain: PRODUCT
  updated: "2026-04-23"
RETRIEVAL_HINTS:
  keywords: [htmlcut, html extraction, repeatable extraction, request file, quick start, sample page, command guide]
  questions: [what is HTMLCut?, why would I use HTMLCut instead of copying values by hand?, how do I start HTMLCut quickly?, can HTMLCut save a request and run it again later?]
  related: [docs/getting-started.md, docs/cli.md, docs/README.md, PATENTS.md]
-->

# HTMLCut

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

The first line saves the request while the page is still warm. The second line runs the same step
again later without rebuilding it from scratch.

## For Repeated Page Work

HTMLCut fits people who keep revisiting the same page work in scripts or agent workflows and want
something smaller than full browser automation.

- Pull the same title, price, link, or fragment from recurring pages
- Check the match before wiring it into a script
- Save the request as a file you can rerun or hand to someone else

It is for the recurring page task you would otherwise handle with copy-paste, a brittle one-off
command, or a heavier stack.

## Start With One Sample Page

- [Getting Started](docs/getting-started.md) takes you from install to a first saved request.
- [Releases](https://github.com/resoltico/HTMLCut/releases) publish prebuilt packages for macOS,
  Linux, and Windows.
- [The command guide](docs/cli.md) is there when you want more than the quick start.

Try one sample page first. If that saved step feels useful, you will know quickly whether HTMLCut
belongs in the rest of your workflow.

## Easy To Check, Clear About Limits

- The sample page and example commands are tested with each release.
- Releases are public and easy to inspect.
- HTMLCut is open source and MIT-licensed.

HTMLCut works best when the source already contains the content you need. If a page only becomes
useful after JavaScript runs, render it first and then hand the resulting HTML to HTMLCut. If you
need a full browser session, let that tool handle the session and let HTMLCut handle the
extraction afterward.

## Legal

HTMLCut is released under the [MIT License](LICENSE). See [NOTICE](NOTICE) and
[PATENTS](PATENTS.md) for the other legal files.
