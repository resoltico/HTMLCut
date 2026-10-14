# HTMLCut

HTMLCut returns exactly the data an extraction contract declares from one caller-owned HTML document, or refuses with facts that help repair the contract.

The loop is **outline → author → run → refuse → repair**. Outline shows a bounded skeleton; a contract declares selectors, counts, lexical value shapes and page expectations; subsequent runs need no model.

## Build from a clean checkout

Install rustup and a native linker as described in [platform support](docs/platform-support.md). Rustup uses the pinned development toolchain automatically from this directory.

```sh
cargo build --release --locked -p htmlcut-cli
cargo install --path crates/htmlcut-cli --locked
```

Cargo installs the executable in its standard bin directory; ensure that directory is on your PATH. The current source version is 22.0.0. Build this source for this contract language.

## Workflow

These commands use page.html with the structure of [the catalogue example](conformance/contract/04-records/page.html):

```sh
htmlcut outline page.html --select 'article.product'
htmlcut get 'h1' page.html
htmlcut run '{"expect":[{"select":"h1","equals":"All products"}],"select":"article.product","match":"all","fields":{"title":{"select":"h3 a","read":"attr:title"},"price":{"select":".price","pattern":"^£(?<value>[0-9.]+)$"},"badge":{"select":".badge","match":"optional"}}}' page.html
```

Save the same JSON as a contract and use run --from-file. FILE defaults to standard input, and readable paths include pipes and process substitution. Run htmlcut --help for the complete [guide](crates/htmlcut-cli/src/guide.txt).

Results are compact UTF-8 JSON. Raw scalars print strings with LF; raw lists require LF-free items and otherwise refuse with a JSON repair hint. A writable regular output file is replaced atomically only after a complete result; symlinks are followed, `-` is stdout, and pipes/devices are written directly. A refusal before publication preserves the destination. Existing-file replacement preserves inspected authorization, with identity/policy revalidation and the concurrent-mutation limits in the guarantees. The invocation deadline covers acquisition and stream delivery, with an atomic regular-file commit boundary. Diagnostics name the contract location, selector, enclosing row indexes, observed count or value, and relevant resource or I/O facts.

Counts defend against missing, duplicated or cleanly truncated rows. Callers still validate that a snapshot represents the intended business entity; a structurally identical wrong page can satisfy a contract.

Fetching, JavaScript execution, article inference, document conversion, type conversion, pagination and joins belong to other tools that compose through files and pipes. Operators supply physical isolation when they require it.

Read the [contract language](docs/contract.md), [guarantees](docs/design.md), [executable cases](conformance/README.md) and [contributor setup](CONTRIBUTING.md).
