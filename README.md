# HTMLCut

HTMLCut is a CLI for cutting repeatable fragments out of HTML or plain text sources.
You point it at a URL, a local file, or stdin, give it a start pattern and an end pattern, and it returns the matching slice as:

- plain text
- raw HTML
- structured JSON

Version `2.x` is intentionally stdout-first, strict, and automation-friendly:

- no timestamped surprise files
- no hidden history database
- no success chatter unless you ask for it
- deterministic bundle output when you do want files
- strict failure on incomplete matches

## Requirements

- Node.js `24+`

## Installation

Basic install:

```bash
npm install
npm link
htmlcut --help
```

### Node Version Managers And Upgrades

If you use `fnm`, `nvm`, `asdf`, `volta`, or another setup where the active Node installation can change, `htmlcut` is linked into that specific Node installation's global prefix.

That means:

- run `npm link` from the same active Node version you plan to use for `htmlcut`
- if you switch Node versions, you may need to run `npm link` again
- if Node is upgraded and the active global prefix changes, the old link may no longer be visible

If `htmlcut` suddenly becomes `command not found` after a Node switch or upgrade, re-establish the link in the currently active Node environment:

1. Confirm which Node installation is active:

```bash
node -v
npm prefix -g
```

2. Recreate the global link from the HTMLCut repository:

```bash
cd /path/to/HTMLCut
npm unlink -g htmlcut || true
npm link
hash -r
```

3. Verify the command:

```bash
command -v htmlcut
htmlcut --help
```

If you want `htmlcut` available under more than one installed Node version, repeat those steps once per version.

Note: npm `11` removed `npm bin -g`. Use `npm prefix -g` instead.

## Quick Start

Extract the first article from a URL and print readable text:

```bash
htmlcut https://example.com --from '<article>' --to '</article>'
```

Extract every heading from stdin and emit machine-readable JSON:

```bash
curl -sL https://example.com | htmlcut - --from '<h2>' --to '</h2>' --all --format json
```

Keep the delimiters instead of cutting them away:

```bash
htmlcut ./page.html --from '<section>' --to '</section>' --capture outer --format html
```

Write a deterministic bundle to disk:

```bash
htmlcut ./page.html --from '<main>' --to '</main>' --bundle ./cut --format json --verbose
```

## CLI Contract

```text
htmlcut <input> --from <pattern> --to <pattern> [options]
```

`<input>` can be:

- an `http://` or `https://` URL
- a local file path
- `-` for stdin

### Pattern Options

| Option | Meaning | Default |
| --- | --- | --- |
| `--from`, `-f` | Start delimiter | required |
| `--to`, `-t` | End delimiter | required |
| `--pattern`, `-p` | `literal` or `regex` | `literal` |
| `--flags` | JavaScript RegExp flags, without `g` | `u` |
| `--all`, `-a` | Return every non-overlapping match | `false` |
| `--capture`, `-c` | `inner` or `outer` | `inner` |

### Output Options

| Option | Meaning | Default |
| --- | --- | --- |
| `--format`, `-F` | `text`, `html`, `json`, or `none` | `text` |
| `--bundle`, `-o` | Write `selection.html`, `selection.txt`, and `report.json` into a directory | off |
| `--base-url`, `-b` | Absolute URL used to rewrite relative links | off |
| `--verbose`, `-v` | Print status lines to stderr | `false` |

### Limits

| Option | Meaning | Default |
| --- | --- | --- |
| `--max-bytes` | Maximum input size, e.g. `512kb`, `10mb` | `50mb` |
| `--fetch-timeout-ms` | Timeout for URL fetches | `15000` |

## Output Modes

### `--format text`

Prints readable plain text to stdout.

HTML is rendered into text with lightweight structure:

- `h1`/`h2` render as underlined headings for better plain-text scanning
- `h3` and deeper headings render with `###`-style prefixes
- links become `text [url]`
- lists stay indented
- tables render as `|`-separated rows
- blockquotes become `> ...`

### `--format html`

Prints the raw matched HTML fragment(s) to stdout.

If `--base-url` is set, or the input itself is a URL, relative link-like attributes such as `href` and `src` are rewritten to absolute URLs first.

### `--format json`

Prints a structured JSON report:

```json
{
  "tool": "htmlcut",
  "version": "2.0.0",
  "input": {
    "kind": "stdin",
    "value": "-"
  },
  "baseUrl": "https://example.com/docs/start.html",
  "documentTitle": "example.com",
  "pattern": {
    "from": "<article>",
    "to": "</article>",
    "mode": "literal",
    "flags": null,
    "capture": "inner",
    "all": false
  },
  "stats": {
    "bytesRead": 52,
    "durationMs": 3,
    "matchCount": 1
  },
  "matches": [
    {
      "index": 1,
      "range": { "start": 9, "end": 47 },
      "innerRange": { "start": 9, "end": 47 },
      "outerRange": { "start": 0, "end": 57 },
      "html": "<a href=\"https://example.com/guide.html\">Guide</a>",
      "text": "Guide [https://example.com/guide.html]"
    }
  ],
  "bundle": null
}
```

### `--format none`

Suppresses stdout completely.

Use this when you want deterministic bundle files without also streaming a payload to stdout.

## Bundle Output

When `--bundle ./some-dir` is set, HTMLCut writes:

```text
some-dir/
  selection.html
  selection.txt
  report.json
```

Design notes:

- file names are deterministic
- existing files are overwritten
- `selection.html` is wrapped in a minimal HTML document unless the match already is a full HTML document
- `report.json` contains the same JSON report emitted by `--format json`
- `--format none` is the cleanest bundle mode for scripts and AI agents that only want files

## Behavior Changes in `2.x`

These are deliberate hard breaks:

- stdout is now the primary interface
- success logging is silent by default
- incomplete trailing matches are fatal, including in `--all` mode
- the old SQLite history feature is gone
- the old timestamped output-pair workflow is gone
- long options use hyphenated names like `--base-url`, `--max-bytes`, `--fetch-timeout-ms`

## Exit Codes

| Code | Meaning |
| --- | --- |
| `0` | success |
| `1` | unexpected internal error |
| `2` | invalid usage or invalid patterns |
| `3` | source could not be read or exceeded limits |
| `4` | extraction failed or was incomplete |
| `5` | bundle files could not be written |

## Development

```bash
npm test
npm run lint:check
npm run coverage
```

Current quality gates:

- tests pass
- lint passes with `--max-warnings 0`
- `npm audit` is clean
