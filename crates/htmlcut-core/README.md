# HTMLCut core

HTMLCut accepts immutable UTF-8 HTML, prepares one bounded original DOM and
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
Parser types remain private. Preparation and execution limits are logical
accounting rather than OS CPU/RSS isolation.

See the [full core contract](https://github.com/resoltico/HTMLCut/blob/main/docs/core.md)
for readings, records, guards, supported bounds and refusal behavior.
