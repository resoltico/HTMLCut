# HTMLCut core

HTMLCut accepts immutable UTF-8 HTML, parses one original DOM and checks declared acceptance limits and
executes reusable compiled queries into complete owned typed results. The core
performs extraction; callers own acquisition and delivery.

```rust
use htmlcut_core::{CompiledPlan, ExtractionPlan, Reading, PreparationLimits,
    PreparedDocument, SnapshotMetadata, SourceSnapshot};
let document = PreparedDocument::new(
    SourceSnapshot::new("<p id='amount'>EUR 180</p><a href='next'>Link</a>", SnapshotMetadata::default())?,
    PreparationLimits::default())?;
let amount = CompiledPlan::compile(&ExtractionPlan::css("#amount")?)?;
let mut link = ExtractionPlan::css("a")?;
link.read = Some(Reading::Attribute("href".into()));
let link = CompiledPlan::compile(&link)?;
assert_eq!(document.execute(&amount)?.data().as_values().unwrap(), ["EUR 180"]);
assert_eq!(document.execute(&link)?.data().as_values().unwrap(), ["next"]);
# Ok::<(), htmlcut_core::ExtractionError>(())
```

Results expose complete counts and owned values. A successful query establishes
query completion; callers must separately verify JSON encoding and delivery.
Parser types remain private. DOM element/node/depth limits are checked after upstream parsing.
Execution budgets count HTMLCut traversal and projected bytes, not upstream parser or selector
internals. The core has no physical CPU, memory or stack containment; use the supervised CLI
worker for untrusted HTML and selectors.

See the [full core contract](https://github.com/resoltico/HTMLCut/blob/main/docs/core.md)
for readings, records, guards, supported bounds and refusal behavior.
