---
afad: "4.0"
version: "15.0.0"
domain: CORE
updated: "2026-09-30"
route:
  keywords: [core, extract, inspect_source, preview_extraction, operation_catalog, schema_catalog, typed requests, diagnostics]
  questions: ["what is the maintained htmlcut-core surface?", "what does the core schema registry cover?", "how should a Rust caller embed htmlcut-core?"]
---

# Core API

Supply accepted UTF-8 bytes and explicit metadata. The snapshot preserves source spelling and CRLF; the parsed DOM follows HTML parsing rules. Prepared documents parse lazily at most once and cache failures. Compile plans once and reuse them.

```rust
use htmlcut_core::{CompiledPlan, ExtractionPlan, PreparationLimits, PreparedDocument,
    Projection, SnapshotMetadata, SourceSnapshot};
let snapshot = SourceSnapshot::new("<p id='amount'>EUR 180</p><a href='next'>Link</a>", SnapshotMetadata::default())?;
let document = PreparedDocument::new(snapshot, PreparationLimits::default())?;
let amount = CompiledPlan::compile(&ExtractionPlan::css("#amount")?)?;
let mut link = ExtractionPlan::css("a")?;
link.projection = Projection::Attribute { name: "href".into() };
let link = CompiledPlan::compile(&link)?;
assert_eq!(document.execute(&amount)?.values, ["EUR 180"]);
assert_eq!(document.execute(&link)?.values, ["next"]);
# Ok::<(), htmlcut_core::ExtractionError>(())
```

Slicing returns exact accepted bytes and requires no DOM parsing:

```rust
use htmlcut_core::{Boundary, CompiledPlan, ExtractionPlan, PreparationLimits,
    PreparedDocument, SnapshotMetadata, SourceSnapshot};
let document = PreparedDocument::new(SourceSnapshot::new("STARTéEND", SnapshotMetadata::default())?, PreparationLimits::default())?;
let plan = ExtractionPlan::slice(Boundary::Literal { value: "START".into() }, Boundary::Literal { value: "END".into() })?;
let result = document.execute(&CompiledPlan::compile(&plan)?)?;
assert_eq!(result.values, ["é"]);
assert_eq!(result.ranges.unwrap()[0].start, 5);
# Ok::<(), htmlcut_core::ExtractionError>(())
```

One plan selects once and projects once. Single is the default; all and one-based nth are explicit. Present empty attributes/text are valid; absence fails. Context guards are conjunctive and every matched value must satisfy the exact or regex-search predicate. Their original-DOM reads cannot be weakened by output exclusions or transforms.

Core preparation/execution limits bind identities. Adapter acquisition/framing limits do not. Source and normalized-plan digests prove identity, not authenticity or business correctness. Callers own browsers, domain mapping, history and comparison policy.
