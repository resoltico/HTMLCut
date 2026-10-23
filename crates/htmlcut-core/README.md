# htmlcut-core

A pure engine for caller-owned HTML bytes. Compile JSON with `Contract::from_json` or build `NodeSpec` and call `Contract::compile`. Parse with `ParseOptions`, then reuse the immutable document with independent `ResultLimits` and `Stop` values. `Document`, `Contract`, `Value` and `OutlineReport` are `Send + Sync`.

```rust
use htmlcut_core::{Contract, Document, ExportFormat, ParseOptions, ResultLimits, Stop, Value};

let stop = Stop::none();
let document = Document::parse(b"<h1>Snapshot</h1>", &ParseOptions::default(), &stop)?;
let contract = Contract::from_json(r#""h1""#, &stop)?;
let limits = ResultLimits::default();
let value = document.extract(&contract, ExportFormat::Json, &limits, &stop)?;
assert_eq!(value, Value::String("Snapshot".into()));
assert_eq!(value.export(ExportFormat::Json, limits.max_output_bytes, &stop)?, b"\"Snapshot\"\n");
# Ok::<(), htmlcut_core::Error>(())
```

`Contract::supports(format)` admits the root representation before acquiring HTML. `Document::outline` returns grouped facts; `OutlineReport::render` emits bounded text. Result ceilings count emitted bytes in the requested format, including LF; raw null emits zero bytes. Transformed source readings have a separate reading-byte limit. Parsing defaults to 64 MiB input and 2,000,000 retained nodes; result limits default to 64 MiB per reading/output.

`Stop::after` creates one cooperative deadline, and `with_cancel` attaches an atomic cancellation flag. Core imposes no default deadline, starts no threads and performs no local stream/file acquisition or publication. Checkpoints do not impose a physical memory quota or interrupt blocking host operations. Error variants carry typed facts; `ErrorCode::class()` lets hosts choose their terminal policy, and `to_json_line()` renders the diagnostic wire format.

Read the [contract specification](https://github.com/resoltico/HTMLCut/blob/main/docs/contract.md) and the [packaged example](https://github.com/resoltico/HTMLCut/blob/main/crates/htmlcut-core/examples/contract.rs). The workspace lint forbids unsafe code. Scopes, sibling extents, readings, decoding, URL resolution and the selector dialect follow the language specification.
