# Design and guarantees

## 1. Purpose

HTMLCut returns exactly the data an extraction contract declares from one HTML document,
or refuses with a diagnosis that is sufficient to repair the contract.

It serves one loop:

1. **Outline** a document cheaply to see its structure.
2. **Author** a contract: selectors, counts, value shapes and page expectations.
3. **Run** the contract deterministically, as often as needed, without a model.
4. **Refuse** when the document no longer satisfies the contract.
5. **Repair** the contract from the refusal's facts.

Authors are usually language models, so the surface must be guessable, the diagnostics
must carry the observed facts, and a contract must be data that is safe to execute.

### Non-goals

Fetching, cookies and sessions; JavaScript execution; article or main-content inference;
Markdown or other document conversion; type conversion and transformation pipelines;
joins across documents; a server or protocol mode; operating-system sandboxing. Each is
served better by an existing tool that composes with HTMLCut through files and pipes.

## 3. Outline

`outline` prints a bounded skeleton for a reader. It is the discovery step and replaces
`inspect`.

- Without `--select` the root is the document element. With it, the first line is
  `N matches for SELECTOR` (`1 match for …`), and the first match is outlined. Zero
  matches is a successful answer.
- One line per element, indented two spaces per level:
  `tag#id.class.class [attr attr] ×N "own text" (+K inside)`.
- `#id` appears only when the line stands for one element. The first four classes
  appear, then `.+K`. Attribute names other than `id` and `class` appear in source order,
  the first six, then `…`. With `--attrs` each is `name="value"`, the value collapsed and
  clipped to 40 characters with `…`.
- Element children are grouped by signature: tag, class list, and the distinct tags of
  their own element children in order of first appearance. A group prints once, where its
  first member stands, with `×N`; only the first member is expanded.
- `own text` is the element's direct text, collapsed, trimmed and clipped to 60
  characters with `…`; it is omitted when empty and for `script` and `style`.
- An element at `--depth` (default 6), and any `svg` or `math` element, is not expanded;
  it carries `(+K inside)`, K being its descendant element count.
- Lines beyond `--max-lines` (default 80) are replaced by one final line:
  `… K more lines; narrow --select, lower --depth or raise --max-lines`.

## 4. Document preparation

**Input.** A path or standard input, read completely before any work. Any readable file
kind is accepted, including pipes and process substitution. More than `max_input_bytes`
refuses with `limit_exceeded`.

**Decoding.** The encoding is the first that applies: a UTF-8, UTF-16LE or UTF-16BE byte
order mark; the caller's `--encoding` label; a declaration found by the HTML standard's
prescan of the first 1,024 bytes; otherwise UTF-8. Labels and decoding follow the WHATWG
Encoding standard through `encoding_rs`. An unknown caller label is `invalid_encoding`; the standard prescan ignores unknown in-document declarations. Malformed
input is never replaced: it refuses with `undecodable_input`, naming the encoding and the
byte offset of the malformed sequence.

**Parsing.** html5ever through scraper's tree sink with default options, fed in chunks of
at most 64 KiB split at character boundaries. After each chunk, more than `max_nodes`
tree nodes refuses with `limit_exceeded`. After parsing, a tree deeper than 1,024 levels
refuses with `limit_exceeded` (`limit` `depth`); the depth bound is fixed because
upstream matching recurses on depth and browsers themselves cap nesting at 512.

## 5. Limits and time

Limits are operator policy. They are never part of a contract.

| Limit | Default | Bounds |
|---|---|---|
| `max_input_bytes` | 64 MiB | bytes read |
| `max_nodes` | 2,000,000 | tree nodes during parsing, hence memory |
| `max_output_bytes` | 64 MiB | encoded result, counted as it is built |
| `time_limit` | 10 s | decoding, parsing, evaluation and encoding |
| depth | 1,024, fixed | tree depth |

The library checks the time limit cooperatively: between parse chunks and at least every
1,024 iterations of its own selection, reading and outline loops. The command line adds
a watchdog thread that covers upstream code the library cannot interrupt. Publication is
a single transition: the main thread claims it before writing, the watchdog claims it to
report `limit_exceeded` and exit 4, and only one claim succeeds. Waiting for standard
input is not timed.

Operator policy consists of the four bounds above and the fixed tree-depth admission.

## 6. Errors

A contract, preparation or limit refusal is one JSON object on standard error, on one line, and nothing on standard output. A transport failure after stdout publication begins can expose a prefix; the command still reports output_unwritable and exits 5. Named file output is atomic. It always has `error` and `message`. Other members appear when known:

| Member | Meaning |
|---|---|
| `path` | Contract location, such as `$.fields.price` or `$.expect[0]` |
| `select` | The selector at that location |
| `row` | One-based indexes of the enclosing `all` matches, outermost first |
| `found`, `expected` | Observed count and accepted range, such as `"1"`, `"0..1"`, `"3..20"`, `"4.."` |
| `value` | At most 120 characters of the observed value |
| `limit`, `bound` | The exceeded limit and its configured bound |
| `encoding`, `offset` | Decoding facts |
| `file`, `detail` | Path and operating-system error text for input and output failures |

`message` is a complete English sentence that repeats the facts; its wording is not part
of the frozen surface. The facts each code carries are fixed:

| Code | Facts |
|---|---|
| `count_mismatch` | `path`, `select`, `row`, `found`, `expected` |
| `value_mismatch` | `path`, `row`, `value` |
| `expectation_failed` | `path`, `select`, `row`, and either `found` with `expected` or `value` |
| `missing_attribute` | `path`, `row` |
| `unresolvable_url` | `path`, `row`, `value` |
| `limit_exceeded` | `limit` (`input_bytes`, `nodes`, `depth`, `output_bytes`, `time`), `bound` |
| `undecodable_input` | `encoding` (the WHATWG name in lower case), `offset` |
| `invalid_contract`, `invalid_pattern` | `path` when the failure has a location |
| `invalid_selector` | `path`, `select` |
| `input_unreadable`, `output_unwritable` | `file`, `detail` |

`row` is present only inside an `all` match, and `select` only where the contract gives
one.

| Exit | Codes |
|---|---|
| 2 | `invalid_arguments`, `invalid_contract`, `invalid_selector`, `invalid_pattern`, `invalid_encoding`, `invalid_base_url` |
| 3 | `count_mismatch`, `value_mismatch`, `expectation_failed`, `missing_attribute`, `unresolvable_url` |
| 4 | `limit_exceeded` |
| 5 | `input_unreadable`, `undecodable_input`, `output_unwritable` |
| 6 | `internal` |

Exit 3 means the document does not satisfy the contract: re-author. Exit 2 means the
request itself is wrong: fix it without looking at the document.

## 8. Library

`htmlcut-core` exposes the same engine. Public enums and option structs are
`#[non_exhaustive]` so that additions are not breaking.

```rust
pub struct Limits { pub max_input_bytes: usize, pub max_nodes: usize,
                    pub max_output_bytes: usize, pub time_limit: Option<Duration> }
pub struct Source<'a> { pub bytes: &'a [u8], pub encoding: Option<&'a str>,
                        pub base_url: Option<&'a str> }

impl Document {
    pub fn parse(source: Source<'_>, limits: &Limits) -> Result<Document, Error>;
    pub fn extract(&self, contract: &Contract) -> Result<Value, Error>;
    pub fn outline(&self, options: &OutlineOptions) -> Result<String, Error>;
}
impl Contract {
    pub fn from_json(text: &str) -> Result<Contract, Error>;
    pub fn schema() -> serde_json::Value;
}
pub enum Value { Null, String(String), List(Vec<Value>), Record(BTreeMap<String, Value>) }
pub struct Error { /* code, message and the facts of section 6 */ }
```

A `Document` keeps its limits; each `extract` and `outline` call gets a fresh time limit.
The library does not start threads, read files or write output.

## 11. Stability

The following are frozen within a major version: contract members and their meaning; the
readings; the commands, options and exit classes of section 7; error codes and fact
names. Additions are allowed: new optional members, readings, options, codes and facts.

A contract carries no version. The engine's major version is the semantics version, and
`htmlcut --version` is what reproduction records. An older engine refuses a newer
contract because unknown members are refused.

`conformance/contract/` is append-only within a major version. Changing or removing an
existing case there is a breaking change by definition, and continuous integration checks
it against the latest release tag of the current major. `conformance/outline/` pins the
reader-facing format and may change in a minor release.

`rust-version` is the lowest toolchain that builds and passes the suite, verified in
continuous integration. `rust-toolchain.toml` pins the development toolchain separately.

## Encoding and publication boundaries

The output limit counts the complete canonical JSON result, excluding its command-line framing LF. Raw output uses the same extraction policy and is no larger. Outline bytes include their line terminators. Output publication is outside the work deadline. Temporary-file rename is atomic replacement, not a durability/fsync guarantee. Memory grows in proportion to admitted input and tree nodes; operators requiring a physical cap provide it externally.

Outline identifiers use CSS escaping and quoted previews use JSON escaping. Headers containing control characters quote the selector as JSON to retain line boundaries.

## Implementation

The [core modules](../crates/htmlcut-core/src/lib.rs) own contract compilation, document preparation, scoped selection/evaluation, readings, outline, values and failures. The [CLI](../crates/htmlcut-cli/src/main.rs) owns argument parsing, input, watchdog and publication. The library starts no threads and performs no filesystem or stream I/O.
