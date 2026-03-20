# HTMLCut

HTMLCut is a CLI for cutting repeatable fragments out of HTML or plain text sources.
You point it at a URL, a local file, or stdin, give it a start pattern and an end pattern, and it returns the matching slice as:

- plain text
- raw HTML
- structured JSON

HTMLCut is stdout-first, strict, and automation-friendly:

- no surprise files unless you ask for a bundle
- no success chatter unless you ask for it
- deterministic bundle output when you do want files
- strict failure on incomplete matches

## Requirements

- Node.js `24+`
- npm available in the same active Node environment

## Install

Choose one install mode and use the matching update steps later.

### Install The Published Package

Use this when you want the released `htmlcut` command on your machine.

1. Confirm which Node environment is active:

```bash
node -v
npm prefix -g
```

2. Install `htmlcut` globally in that active environment:

```bash
npm install -g htmlcut
```

3. Refresh your shell's command cache and verify the command:

```bash
hash -r
command -v htmlcut
htmlcut --version
htmlcut --help
```

### Link A Local Source Checkout

Use this when you are developing `htmlcut` from source and want the global `htmlcut` command to point at your working tree.

1. Go to the repository and install dependencies:

```bash
cd /path/to/htmlcut
npm install
```

2. Create the global link in the currently active Node environment:

```bash
npm link
```

3. Refresh your shell's command cache and verify the command:

```bash
hash -r
command -v htmlcut
htmlcut --version
htmlcut --help
```

## Update

### Update A Published Install

Reinstalling the latest published version is the update path for a normal global install:

```bash
npm install -g htmlcut@latest
hash -r
htmlcut --version
```

### Update A Linked Source Checkout

If your global `htmlcut` command points at a source checkout, update that checkout and refresh the link:

```bash
cd /path/to/htmlcut
git pull
npm install
npm link
hash -r
htmlcut --version
```

If you only changed source files in an already linked checkout, you usually do not need to run `npm link` again. Run it again after switching Node environments, after reinstalling Node, or whenever the global link disappears.

## Node Version Changes And Troubleshooting

If you use `fnm`, `nvm`, `asdf`, `volta`, or any setup that switches the active Node installation, both global installs and `npm link` links belong to that active Node environment.

That means:

- switching Node versions can make `htmlcut` disappear even though it is still installed or linked under a different Node version
- upgrading Node can change the active global prefix
- your shell can cache an old command path until you run `hash -r`

Use this sequence after a Node switch, Node upgrade, or a sudden `command not found`:

1. Inspect the active environment:

```bash
node -v
npm prefix -g
command -v htmlcut || true
```

2. If you use the published package, reinstall it in the active environment:

```bash
npm install -g htmlcut@latest
```

3. If you use a linked source checkout, recreate the link from that checkout in the active environment:

```bash
cd /path/to/htmlcut
npm install
npm link
```

4. Refresh the shell cache and verify:

```bash
hash -r
command -v htmlcut
htmlcut --version
htmlcut --help
```

If you want `htmlcut` available under more than one installed Node version, repeat the appropriate install or link steps once per version.

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

Bundle behavior:

- file names are deterministic
- existing files are overwritten
- `selection.html` is wrapped in a minimal HTML document unless the match already is a full HTML document
- `report.json` contains the same JSON report emitted by `--format json`
- `--format none` is the cleanest bundle mode for scripts and AI agents that only want files

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
