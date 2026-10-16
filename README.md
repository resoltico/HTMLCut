# HTMLCut

HTMLCut returns exactly the data an extraction contract declares from one caller-owned HTML document, or refuses with facts that help repair the contract.

The loop is **outline → author → run → refuse → repair**. Outline shows a bounded skeleton; a contract declares selectors, counts, lexical value shapes and page expectations; subsequent runs need no model.

## Install from a clean checkout

Install rustup and a native linker as described in [platform support](https://github.com/resoltico/HTMLCut/blob/main/docs/platform-support.md). From the checkout root, run the following command; rustup selects the pinned development toolchain automatically.

```sh
cargo install --path crates/htmlcut-cli --locked
```

Cargo builds the executable and installs it in its standard bin directory; ensure that directory is on your PATH.

## Workflow

From the checkout root, extract the three products in the included [catalogue snapshot](https://github.com/resoltico/HTMLCut/blob/main/conformance/contract/04-records/page.html):

```sh
htmlcut outline conformance/contract/04-records/page.html --select 'article.product'
htmlcut get 'h1' conformance/contract/04-records/page.html
htmlcut run '{"expect":[{"select":"h1","equals":"All products"}],"select":"article.product","match":"all","min":3,"max":3,"fields":{"title":{"select":"h3 a","read":"attr:title"},"price":{"select":".price","pattern":"^£(?<value>[0-9]+[.][0-9]{2})$"},"badge":{"select":".badge","match":"optional"}}}' conformance/contract/04-records/page.html
```

Save the same JSON as `contract.json` to reuse it:

```sh
htmlcut run --from-file contract.json conformance/contract/04-records/page.html
```

Use your own local snapshot as `FILE`. It defaults to standard input; readable paths include pipes and process substitution. Run `htmlcut --help` for the [workflow guide](https://github.com/resoltico/HTMLCut/blob/main/crates/htmlcut-cli/src/guide.txt) and `htmlcut COMMAND --help` for that command's options.

Extraction results are complete compact UTF-8 JSON followed by LF; outline prints a text skeleton. The output ceiling includes this LF. Raw scalars print strings with LF; raw lists require LF-free items and otherwise refuse with a repair hint to use JSON. Diagnostics carry the available contract location, selector, row indexes, observed count or value, and resource or I/O facts.

`-o FILE` atomically replaces a regular file only after a complete result. Symlinks are followed; stdout, pipes and devices are written directly. Existing-file authorization must be inspectable and reproducible. See [output publication](https://github.com/resoltico/HTMLCut/blob/main/docs/publication.md) for access-policy requirements, deadlines, cleanup and concurrency limits.

Counts defend against missing, duplicated or cleanly truncated rows. Callers still validate that a snapshot represents the intended business entity; a structurally identical wrong page can satisfy a contract.

Fetching, JavaScript execution, article inference, document conversion, type conversion, pagination and joins belong to other tools that compose through files and pipes. Operators supply physical isolation when they require it.

Read the [contract language](https://github.com/resoltico/HTMLCut/blob/main/docs/contract.md), [execution guarantees](https://github.com/resoltico/HTMLCut/blob/main/docs/design.md), [output publication](https://github.com/resoltico/HTMLCut/blob/main/docs/publication.md), [executable cases](https://github.com/resoltico/HTMLCut/blob/main/conformance/README.md) and [contributor setup](https://github.com/resoltico/HTMLCut/blob/main/CONTRIBUTING.md).

For embedding, [htmlcut-runtime](https://github.com/resoltico/HTMLCut/tree/main/crates/htmlcut-runtime) supplies shared preflight, reusable documents, cancellation, structured outlines, exports and local publication. [htmlcut-core](https://github.com/resoltico/HTMLCut/tree/main/crates/htmlcut-core) is the pure low-level engine.
