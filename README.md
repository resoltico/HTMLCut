# HTMLCut

HTMLCut is a command-line tool that extracts specific pieces of text or HTML from a web page or local file. You provide a starting phrase and an ending phrase, and HTMLCut saves everything in between into a new file.

This is useful for quickly saving an article, a list of links, or a specific data table without downloading an entire webpage — and it handles large files efficiently via streaming (50 MB limit).

## Requirements

You must have **Node.js 24 or higher** installed.

## Installation

After cloning the repository, either link it globally or run it directly.

```bash
# Option A: Link it globally so you can type "htmlcut" from any folder
npm link

# Option B: Just make the script executable to run it directly
chmod +x src/cli.js
```

## How to Use

HTMLCut requires three basic arguments:
1. `--input` — the file path, website URL, or `-` to read from stdin
2. `--start` — the text where the cut begins
3. `--end` — the text where the cut stops

### Example: Basic Extraction

```bash
# Using the globally linked command
htmlcut --input https://nodejs.org --start "<h1[^>]*>" --end "</h1>" --regex

# Or running the file directly
./src/cli.js --input ./my-local-file.html --start "<h2>" --end "</h2>"

# Read from stdin (pipe)
# -L follows HTTP redirects (curl does not follow them by default, unlike htmlcut's built-in fetch)
curl -sL https://nodejs.org | htmlcut --input - --start "<h1[^>]*>" --end "</h1>" --regex
```

When the tool finishes, two files are written to the current directory (or beside the `--output` base path if specified), using a timestamped name by default:
1. An `.html` file containing the exact HTML extracted, wrapped in a full HTML5 document. The `<title>` is derived from the source: the hostname for URLs, the filename stem for local files, or any `<title>` already present in the extracted fragment.
2. A plain `.txt` file with HTML tags stripped and HTML entities decoded (e.g. `&amp;` → `&`, `&lt;` → `<`).

Diagnostic messages (`✓ Successfully extracted…`, `→ output path`) are written to **stderr**, so stdout is always clean and safe for piping.

Any relative URLs (`href`, `src`) encountered in the extracted HTML fragment are automatically expanded into absolute URLs using the source input as the base domain/path.

- **Automatic Translation:** If you run `htmlcut --input https://example.com/page.html ...`, HTMLCut automatically knows the base URL is `https://example.com/page.html` and expands all relative links against it natively. You do not need any further configuration.
- **Manual Override (`--base`):** If you are reading from a local file (e.g. `htmlcut -i local.html`) or reading from piped stdin (`curl https://example.com | htmlcut -i -`), the original web context is inherently lost. In these cases, you can use the `--base` (or `-b`) flag to pass in the intended absolute URL so your links resolve explicitly against that domain instead of mapping to useless local file paths.

### Example: Piping Output for Scripts or AI Agents

Use `--stdout` (or `-O`) to stream text directly to the terminal or pipe it into another command, without writing any files:

```bash
# Print plain text to the terminal
htmlcut -i https://example.com -s "<article>" -e "</article>" --stdout

# Pipe it straight to another tool
htmlcut -i https://example.com -s "<h2>" -e "</h2>" -g --stdout | grep "Feature"
```

Use `--json` to get a structured JSON payload. Because diagnostics always go to stderr, the output is already clean for piping — no extra flags needed:

```bash
# Clean JSON output — pipe directly into jq with no extra flags
htmlcut -i https://example.com -s "<p>" -e "</p>" -g --json | jq '.[0].text'
```

```json
[
  { "html": "<p>First paragraph.</p>", "text": "First paragraph." },
  { "html": "<p>Second paragraph.</p>", "text": "Second paragraph." }
]
```

Use `--quiet` (`-q`) if you want to also silence stderr (e.g. in fully automated pipelines where even stderr noise is undesirable):

### Example: Extracting Multiple Items

By default, HTMLCut stops after the first match. Add `--global` (or `-g`) to extract every occurrence:

```bash
htmlcut -i https://example.com -s "<p>" -e "</p>" --global
```

### Example: Using Regular Expressions

Add `--regex` (or `-r`) to treat start and end patterns as RegExp `v`-flag (unicodeSets) expressions:

```bash
htmlcut -i ./local-file.html -s "<div class=\"[a-zA-Z0-9-]+\">" -e "</div>" --regex --global
```

### Keeping Track of Your Extractions

Add `--track` (or `-t`) to opt-in to logging the extraction to a local SQLite database:

```bash
htmlcut -i https://example.com -s "<h1>" -e "</h1>" --track
```

View your recent extraction history:

```bash
htmlcut --history
```

History is capped at 1000 records and pruned automatically on each insert. The output shows the 50 most recent entries. By default the database is stored at `~/.htmlcut_history.db`. Override this with the `HTMLCUT_DB_PATH` environment variable:

```bash
HTMLCUT_DB_PATH=/path/to/custom.db htmlcut --history
```

## Command Options

| Flag | Short | What it does | Required |
| :--- | :--- | :--- | :---: |
| `--input` | `-i` | Local file path, website URL, or `-` for stdin. | **Yes** |
| `--base` | `-b` | Overrides the base URL for resolving relative links. | No |
| `--start` | `-s` | The exact text or pattern where the cut starts. | **Yes** |
| `--end` | `-e` | The exact text or pattern where the cut stops. | **Yes** |
| `--regex` | `-r` | Treats your start and end inputs as RegExp `v` expressions. | No |
| `--global` | `-g` | Continues searching and extracts all matches found. | No |
| `--output` | `-o` | Base path for output files (e.g. `--output my_results` → `my_results.html` + `.txt`). Defaults to a timestamped name in the current directory. | No |
| `--stdout` | `-O` | Stream the extracted text directly to stdout (no files written). | No |
| `--json` | | Stream a JSON array of `{html, text}` objects to stdout (no files written). | No |
| `--quiet` | `-q` | Suppress all diagnostic output on stderr (useful in fully automated pipelines). | No |
| `--track` | `-t` | Saves this extraction to your local history log. | No |
| `--history` | `-H` | Prints your recent extraction history log. | No |
| `--version` | `-V` | Prints the version number. | No |
| `--help` | `-h` | Prints the help menu. | No |

## Development

HTMLCut has no external runtime dependencies. All dev tooling is local.

```bash
# Lint
npm run lint -- --fix

# Run all tests
npm test

# Run tests with coverage report
npm run coverage
```

Coverage target: **100% line, branch, and function** across all source files.

## License

MIT License. See `LICENSE`.
