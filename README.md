# HTMLCut

HTMLCut extracts exactly what a contract declares from a caller-owned HTML snapshot, or refuses with facts that help repair the contract.

Use **outline → author → run → refuse → repair**: inspect a bounded skeleton, declare selectors, counts, value checks and page expectations, then reuse the contract without a model.

## Install

Install the CLI with `cargo install htmlcut --locked`, or unpack an archive from [GitHub releases](https://github.com/resoltico/HTMLCut/releases). Put the executable on your PATH. Archives contain the binary, documentation and license notices; there are no installer scripts.

For a clean source checkout, install rustup and a native linker, then build:

```sh
cargo build --release -p htmlcut --locked
```

The executable is `target/release/htmlcut` (`htmlcut.exe` on Windows). Rustup uses [rust-toolchain.toml](rust-toolchain.toml); [Cargo.toml](Cargo.toml) declares the minimum supported Rust version.

Release targets are macOS arm64 and x86_64, Linux x86_64 musl and Windows x86_64 MSVC. Apple release builds target macOS 12; native CI does not establish execution on every older OS version. Source builds work wherever Rust and the dependencies build. Windows source builds need the MSVC C++ build tools; Unix builds need the native linker; fuzz tooling also needs a C++ compiler.

## Workflow

These commands run from the checkout root against the included three-product snapshot:

```sh
cargo run -q -p htmlcut -- outline conformance/contract/records/product-fields/page.html --select 'article.product'
cargo run -q -p htmlcut -- get h1 conformance/contract/records/product-fields/page.html
cargo run -q -p htmlcut -- run conformance/contract/records/product-fields/contract.json conformance/contract/records/product-fields/page.html --base-url https://books.example/shop/index.html
```

The contract asserts the page heading and extracts records with lexical values, optional badges and resolved URLs. Author the contract once; subsequent runs use the JSON file. A count that the snapshot cannot satisfy refuses with exit 3:

```sh
cargo run -q -p htmlcut -- run --json '{"select":"article.product","match":"all","min":4}' conformance/contract/records/product-fields/page.html
```

After checking that three products are the intended complete set, repair the declaration:

```sh
cargo run -q -p htmlcut -- run --json '{"select":"article.product","match":"all","min":3,"max":3}' conformance/contract/records/product-fields/page.html
```

Counts protect against missing, duplicated and cleanly truncated rows. A structurally identical wrong business entity can still satisfy a contract; declare the page expectations you need.

## Commands and input

- `htmlcut outline [FILE]` inspects structure; `--select`, `--depth`, `--max-lines` and `--attrs` control the view.
- `htmlcut get CSS [FILE]` reads one value; use `--all`, `--read READING` and `-r/--raw` as needed.
- `htmlcut run CONTRACT [FILE]` reads a contract path; `-` in the CONTRACT position selects contract stdin. `htmlcut run --json TEXT [FILE]` uses inline JSON.
- `htmlcut schema [-o DEST]` prints the generated authoring schema.

FILE is a local snapshot path and defaults to stdin. Implicit stdin is refused when it is a terminal; explicit `-` is allowed. Contract `-` cannot be combined with absent HTML FILE or HTML `-`. Contract files must be UTF-8 and fit the contract byte bound. `--base-url` and `--encoding` supply snapshot metadata; a BOM takes precedence over an encoding label.

Use `htmlcut --help` for examples, contract members and exit statuses, and `htmlcut COMMAND --help` for exact options. Help and version have no deadline.

## Output, limits and diagnostics

JSON is compact UTF-8 plus LF. Raw scalars emit their bytes plus LF; raw lists require LF-free strings and emit one LF per item; an optional null emits nothing. The output limit counts the bytes emitted in the requested format. Each transformed source reading has its own reading-byte ceiling.

The defaults are 64 MiB input, 2,000,000 retained nodes, 64 MiB per reading and 64 MiB output. Override them with `--max-input-bytes`, `--max-nodes`, `--max-reading-bytes` and `--max-output-bytes`. The contract and parser also have fixed admission bounds described in the [language specification](docs/contract.md).

`--timeout SECONDS` defaults to 10 and is **cooperative** through compilation, chunked input, parsing, evaluation and export. Blocking input can exceed it; output writing is not interrupted. For a hard wall-clock bound, use a process supervisor, for example `timeout 10 htmlcut …` where `timeout(1)` is installed. Logical limits do not impose an operating-system memory quota.

No destination is opened until complete result bytes exist. `-o PATH` stages beside an ordinary file and renames it into place. Symlinks to existing files keep the link and replace its canonical target; dangling symlinks and directories refuse. Existing file permission bits are copied; ownership, ACLs and extended access policy are not preserved. New Unix files follow the umask. Hard links and open handles retain the old file. There is no fsync, and a kill signal can leave a `.htmlcut-*` stage.

Stdout (`-o -` or no output option), lexical device paths and existing nonregular targets are direct writes. Device paths are Unix `/dev/…`, Windows `\\.\…` and reserved DOS device names, case-insensitive and ignoring extensions. Broken pipes on direct writes/flushes are silent success; other I/O failures can expose a prefix. File replacement does not provide a concurrency or durability guarantee beyond the rename.

Help and version use stdout with exit 0. Request errors, including unknown flags, use the same JSON diagnostic format as extraction errors. Errors are one flat JSON object plus LF on stderr when reporting succeeds. Facts include paths, selectors, row indexes, observed counts/value previews and resource or I/O details. Count expectations are objects such as `{"min":1,"max":1}`. See the [diagnostic facts table](docs/contract.md#diagnostics).

| Exit | Meaning |
|---|---|
| 0 | Success, including a closed direct output pipe |
| 2 | Invalid request, contract, selector, pattern, encoding or base URL |
| 3 | Contract or output representation refusal |
| 4 | Resource limit exceeded |
| 5 | Input, decoding or output failure |
| 6 | Internal error, including a caught panic with message/location |
| 130 | Cancellation |

## Acquire a snapshot first

Fetch successfully before launching extraction, and pass the source URL for relative links:

```sh
curl -fsSL --max-filesize 67108864 -o page.html https://example.test/catalogue \
  && htmlcut run contract.json page.html --base-url https://example.test/catalogue
```

`example.test` is a placeholder for your source URL. Supply `--encoding` when a transport charset is needed. An unchecked partial fetch can still look like valid HTML; counts and entity expectations remain necessary.

Fetching, JavaScript execution, article inference, conversion, pagination, joins and session/network policy belong to other tools. HTMLCut does not execute page scripts or infer business meaning.

For embedding, use [htmlcut-core](crates/htmlcut-core/README.md). Read the [contract language](docs/contract.md), [executable corpus](conformance/README.md), [contributor instructions](CONTRIBUTING.md) and [changelog](CHANGELOG.md).
