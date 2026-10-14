# Design and guarantees

## Purpose

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

## Outline

`outline` prints a bounded skeleton for a reader to inspect the document structure.

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
- An element at `--depth` (default 6), and any `svg`, `math` or `template` element, is not expanded;
  it carries `(+K inside)`, K being its descendant element count.
- Lines beyond `--max-lines` (default 80) are replaced by one final line:
  `… K more lines; narrow --select, lower --depth or raise --max-lines`.

## Document preparation

**Input.** A path or standard input, read completely before any work. Any readable file
kind is accepted, including pipes and process substitution. More than `max_input_bytes`
refuses with `limit_exceeded`.

**Decoding.** The encoding is the first that applies: a UTF-8, UTF-16LE or UTF-16BE byte
order mark; the caller's `--encoding` label; a declaration found by the HTML standard's
prescan of the first 1,024 bytes; otherwise UTF-8. Labels and decoding follow the WHATWG
Encoding standard through `encoding_rs`. An unknown caller label is `invalid_encoding`; the standard prescan ignores unknown in-document declarations. Malformed
input is never replaced: it refuses with `undecodable_input`, naming the encoding and the
byte offset of the malformed sequence.

**Parsing.** A private HTMLCut builder admits retained DOM allocation and every
append, insertion, detach and reparent operation before freezing an opaque document.
Maintained html5ever 0.40.1 parses bounded feeds of at most 1,024 UTF-8 bytes. A refusal
latch stops owned DOM allocation/mutation; unique handles retain immutable parser name,
namespace, template and MathML integration facts until the current feed stops. Refusal
uses ordinary errors, including under `panic=abort`, and does not finalize a partial tree.
Depth is parent-edge distance from every retained arena root at depth zero, counting all
node kinds, including detached/template forests. Transient construction depth over 1,024
refuses, even if the final repaired tree would be shallower. Selector identity uses stable
frozen node storage; matching caches belong to each extraction call.

## Limits and time

Limits are operator policy, outside the contract.

| Limit | Default | Bounds |
|---|---|---|
| `max_input_bytes` | 64 MiB | supplied bytes before decoding |
| `max_nodes` | 2,000,000 | retained owned DOM nodes before allocation |
| `max_reading_bytes` | 64 MiB | each complete transformed UTF-8 source reading |
| `max_output_bytes` | 64 MiB | complete canonical JSON, charged during production |
| `time_limit` | 10 s | cooperative preparation or execution call |
| depth | 1,024, fixed | transient retained tree depth |

Patterns/captures and predicates may require complete source scalars; their per-reading
admission is separate from final output. A long source with a tiny capture remains valid
within `reading_bytes`. Discarded normalization whitespace and excluded subtrees do not
consume reading/output admission. Keys, delimiters, Unicode and JSON escaping count
exactly. Caller-constructed `Value.to_json` enforces its supplied output bound independently.

Library time checks run between bounded parser feeds and at least every 1,024 owned
iterations. Upstream tokenizer, attribute, parser stack, URL and regex operations may
continue within a feed/call; tokens spanning feeds can retain buffers under input admission.
These logical limits do not promise a physical heap cap or interruption of every upstream
instruction. Callers supply physical process/memory containment when required.

The CLI starts one monotonic deadline after valid argument parsing, before contract and
snapshot acquisition. It stays active through stream write/flush or regular-file staging
and metadata. Zero timeout wins before input consumption. An atomic winner selects expiry,
commit-in-progress, success or error. A file commit claim succeeds only before expiry; after
that indivisible rename starts, its actual outcome governs, with no contradictory time
refusal. Atomic filesystem commit cannot promise an exact wall-clock bound or rollback.

One reporting owner delivers best-effort diagnostics on a separate thread, with a central
50 ms grace. Blocked/closed/nonblocking stderr cannot change the exit class or defeat
termination; its diagnostic can be absent/partial. Ordinary errors claim terminal error
before reporting. Help/version are untimed metadata output and may block. Schema output
uses its own default 10 s invocation deadline and ordinary publication.

Staging ownership is reserved before exclusive creation, confirmed before result bytes,
and checked on creation return. Timeout cleanup runs separately from hard termination;
only confirmed owned stages are removed.

Containing-parent handles require search/path capability rather than directory listing:
Darwin uses `O_SEARCH`, and Linux uses `O_PATH` with `fstat` support (kernel 3.6+).
Write/search permission remains necessary for actual creation and `renameat`; parent
aliases follow kernel resolution and their identities are revalidated before commit.
Darwin staging-directory handles request search access,
so inherited listing denials do not prevent ACL preparation or cleanup. A creation that
succeeds but cannot return a confirmable handle/identity (including inherited denial of
search access) may leave an empty unconfirmed directory; diagnostics name it. Extraordinary
stuck filesystem syscalls may exceed the cleanup grace and leave a candidate/stage, named
in available diagnostics. Unconfirmed creation collisions are never deleted.

## Errors

A contract, preparation or limit refusal is one JSON object on standard error, on one line, and nothing on standard output. A transport failure after stdout publication begins can expose a prefix. A closed downstream pipe ends silently with exit 0; other output failures report `output_unwritable` and exit 5. Regular-file output is atomic; stream and device output is direct. It always has `error` and `message`. Other members appear when known:

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
| `expectation_failed` | `path`, `select`, `row`, and either `found` with `expected`, `value`, or none of those when an attribute is missing |
| `missing_attribute` | `path`, `row` |
| `unresolvable_url`, `unrepresentable_output` | `path`, `row`, `value` |
| `limit_exceeded` | `limit` (`input_bytes`, `nodes`, `depth`, `output_bytes`, `time`), `bound` |
| `undecodable_input` | `encoding` (the WHATWG name in lower case), `offset` |
| `invalid_contract`, `invalid_pattern` | `path` when the failure has a location |
| `invalid_selector` | `select`, and `path` only for a contract location |
| `input_unreadable`, `output_unwritable` | `file`, `detail` |

`row` is present only inside an `all` match, and `select` only where the contract gives
one. Selector messages quote selectors as JSON and give a column and a reason on syntax
errors. Without a selector, messages name "the match itself". Messages contain no Rust
debug renderings.

| Exit | Codes |
|---|---|
| 2 | `invalid_arguments`, `invalid_contract`, `invalid_selector`, `invalid_pattern`, `invalid_encoding`, `invalid_base_url` |
| 3 | `count_mismatch`, `value_mismatch`, `expectation_failed`, `missing_attribute`, `unresolvable_url`, `unrepresentable_output` |
| 4 | `limit_exceeded` |
| 5 | `input_unreadable`, `undecodable_input`, `output_unwritable` |
| 6 | `internal` |

Exit 3 means a contract or output-representation refusal: re-author or use JSON. Exit 2 means the
request itself is wrong: fix it without looking at the document.

## Command line

The four commands are:

- `outline [FILE]` prints a bounded structure, optionally narrowed by `--select`.
- `get CSS [FILE]` runs a value contract, with `--all` and `--read` when requested.
- `run CONTRACT-JSON [FILE]` runs an inline contract; `run --from-file PATH [FILE]`
  reads the contract from a UTF-8 file. `--from-file -` consumes contract stdin only with a distinct explicit HTML source; implicit/dash and recognizable FD0 aliases (including symlinks) refuse before either read. Harder aliases are not universally detectable; `./-` is a literal filename.
- `schema [-o DEST] [--timeout SECONDS]` prints the contract schema without document input.

`FILE` defaults to standard input; `-` explicitly selects standard input. Readable paths
include pipes and process substitution. Implicit input from a terminal is refused before
reading; pass a path, pipe a document, or explicitly use `-`. Input waiting is covered by the invocation deadline.

Results are complete compact UTF-8 JSON followed by one LF. `--raw` / `-r` prints a
string without JSON quoting followed by LF, an array of LF-free strings one per line, or no bytes
for `null`. A record root is refused before document input is read. A multiline raw scalar remains valid; an LF-containing raw list refuses before publication with `unrepresentable_output`, a root path and offending row.

`--output DEST` / `-o DEST` follows symbolic links to the destination a write would
reach. `-` is standard output. A regular file must be writable by the caller without
truncation; only then does a complete staged result atomically replace it. An absent
path is created atomically. A directory is refused. Pipes, devices and descriptor aliases
(such as `/dev/stdout` and `/dev/fd/N`) are written directly, without replacement or
an atomicity guarantee. Local descriptor aliases duplicate the inherited descriptor,
preserving its cursor, append flag and access mode. Foreign process descriptor paths
are opened as path streams; they do not duplicate a local descriptor.

A refusal or limit before publication leaves destinations untouched, including a FIFO
without a reader. Confirmed owned staging is cleaned subject to the creation/cleanup boundaries above. Existing files preserve handle-derived owner/group, rwx, discretionary ACLs and applicable access labels; privileged executable bits/capabilities and unrelated content xattrs are not copied. Unix result files start at mode zero in a verified directory with access/default ACLs removed. Windows result files are created atomically with a protected DACL granting the owner only writer/metadata/delete rights, without data-read access; the retained writer handle installs an empty protected DACL before bytes. Thus a pathname reader cannot acquire a data handle before restriction. An empty non-secret policy probe captures inherited/umask authorization for absent destinations. Ownership and applicable mandatory labels are established and verified under restrictive staging before any result byte. A private deny-read ACL protects Darwin metadata inspection; final ACLs precede mode grants. Only complete content receives final discretionary policy; inspection/reproduction failure refuses before rename. Target identity/policy, original path resolution and stage identity are rechecked immediately before commit. Cleanup also checks created-object identity and leaves detected foreign replacements untouched. The guarantee assumes stage and target identities/policies stay unchanged through commit: portable replacement and unlink cannot eliminate every hostile mutation after revalidation. On Unix, a new file uses mode `0666`
reduced by umask; replacement preserves read, write and execute bits, discarding
set-user-ID, set-group-ID and sticky bits. Replacement retains the pinned owner/group;
an absent destination follows the caller/inherited ownership policy. Other hard links retain the old content. Namespace changes by other processes
between the permission check and rename are not isolated.

Standard output and direct stream output can expose a prefix on transport failure. A
reader closing the pipe is silent success, exit 0; other failures are
`output_unwritable`, exit 5. File publication does not promise fsync durability.

`--help`, `-h` and `help` print the [guide](../crates/htmlcut-cli/src/guide.txt).
`--version` and `-V` print the engine version. A bare invocation remains an argument
error. Argument errors carry the parser's message and end with
`Run htmlcut --help for the guide.` Invalid inline JSON that names an existing file
points to `--from-file`.

## Library

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
pub struct Error { /* code, message and the documented error facts */ }
```

A `Document` keeps its limits; each `extract` and `outline` call gets a fresh time limit.
The library does not start threads, read files or write output.

## Stability

The following are frozen within a major version: contract members and their meaning; the
readings; the commands and options under [Command line](#command-line) and exit classes under [Errors](#errors); error codes and fact
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

Sibling indexes are cached for one extraction or outline call over the immutable tree.
Relative-selector caches are reset for each scope. Selectors with an `of S` argument are refused at admission even if an upstream parser
starts accepting that syntax.


Windows mandatory labels are queried/set separately from audit-only SACL backup. SACL
protection reproduction can require additional native rights; inability to reproduce it
refuses before replacement. See Microsoft's [security information rights](https://learn.microsoft.com/en-us/windows/win32/secauthz/security-information).
