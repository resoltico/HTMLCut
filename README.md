

# HTMLCut

HTMLCut executes declared extraction contracts over immutable UTF-8 HTML snapshots. Its Rust core and native CLI return requested values or records, reject unmet assumptions, and bound preparation, matching and output work.

This source uses query/receipt wire version 6 and semantics 6. It is a breaking 20.0.0 candidate; publication is a separate release operation. Retired requests, APIs, commands and flags have no compatibility adapters.

Build with the pinned Rust toolchain from a clean checkout:

```sh
cargo install --path crates/htmlcut-cli --locked
htmlcut extract --help
htmlcut inspect --file page.html
htmlcut extract --file page.html --select article --nth 1 --read outer-html --raw
htmlcut extract --file page.html --select 'article a' --all --read attr:href
```

Default output is a canonical JSON array of strings or records plus one LF, identical for terminals and pipes. Default selection requires exactly one node. `--all` defaults to minimum one; `--nth N` is positive and one-based, with complete candidate counting. Missing attributes fail; present empty strings remain values. `--raw` emits exactly one scalar without LF.

For related records, declare `--field NAME SELECT READ`, `--optional-field NAME SELECT READ`, or `--many-field NAME SELECT READ`. Optional returns null only when no node exists; many requires at least one. Use current JSON for zero-or-more arrays, bounded/nth fields and richer expectations:

```sh
htmlcut extract --file quotes.html --plan-json '{"version":6,"select":".quote","match":"all","fields":{"author":{"select":".author","match":"optional"},"tags":{"select":".tag","match":"all","min":0},"text":{"select":".text"}}}'
```

Fields run in lexical name order within each original row. `--following-siblings N` adds exactly N element-sibling subtrees (0–63), rejecting missing siblings or overlap with another root candidate. Selectors can inspect original ancestor/sibling context; returned payload stays within the row forest. `--exclude`, `--field-exclude` and `--expect-text` provide small conveniences over the same compiler. Numeric conversion, filtering, ranking, joins and business meaning remain caller code.

The default `text` reading collapses Unicode whitespace outside original HTML pre ancestry, separates structural blocks/breaks and omits inert script/style/template payloads. It retains hidden and noscript content and does not infer browser visibility or main content. `literal` preserves parsed descendant-text concatenation. `markdown` retains selected prose, roles, links, images, code, source list ordinals and table cells as nested lists. `inner-html`/`outer-html` serialize parsed DOM. Relative URL readings require explicit base metadata.

Selected byte-boundary slicing and source-range receipts are removed. Literal text and parsed HTML can change accepted spelling, CRLF, entities or NUL; they do not replace byte cutting. Use guarded caller byte/string code for those jobs. The complete accepted UTF-8 source remains exact in snapshots and replay bundles.

`inspect --select CSS` counts completely and samples bounded identifiers, attribute names and structural text with separate completeness labels. Without select, it surveys repeated siblings and table shapes. Hints are verified against exact original-node membership and order. Discover unknown fields by reading one representative row's outer HTML, then validate the declared fields across every requested row. No recursive inspection or arbitrary attribute-value sampler is needed.

Callers supply saved/rendered UTF-8 HTML and own acquisition, charset conversion, authentication and browser state. HTMLCut does not fetch, crawl or execute JavaScript. Prefer a native API or direct browser/JSON extraction when it already solves the task.

The Rust core retains reusable compiled queries, one lazy prepared DOM (including cached failures), and fresh shared execution budgets. Results have private immutable typed data and one bounded canonical encoding. Ordinary execution performs no identity hashing or receipt serialization. `--receipt FILE` requests execution identities/counts separately; `--bundle FILE` captures source/query/configuration/evidence in closed uncompressed USTAR. They are mutually exclusive. `htmlcut replay FILE` recomputes from bundled bytes after original files move or disappear. Evidence uses the execution's remaining work; cache hits do not grant free logical work. Holding typed data and encoded bytes is bounded duplication.

Artifacts stage before publication. Each file is atomically created/replaced; overwrite requires `--overwrite`. Multiple files/stdout are not a transaction, and later delivery failure can leave a sidecar or pipe bytes. Receipts identify execution/integrity, not authenticity or delivery. Logical bounds do not promise OS CPU/RAM isolation or sanitization.

Distribution uses GitHub native/source archives. Rust consumers use git/path dependencies; owned package registry publication is disabled. See [Getting Started](docs/getting-started.md), [CLI](docs/cli.md), [Core](docs/core.md), [Schemas](docs/schema.md), [Documentation Index](docs/README.md), [Quality Gates](docs/quality-gates.md), [Changelog](changelog.md), [License](LICENSE), [Notice](NOTICE), and [Patents](PATENTS.md).

HTMLCut’s original source is licensed under [MPL-2.0](LICENSE). Third-party components retain their own licenses; see [NOTICE](NOTICE) and [Patent Notes](PATENTS.md).
