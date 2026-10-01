//! Caller-owned in-process timing over the same immutable title-extraction task as the Python comparison.
use htmlcut_core::{
    CompiledPlan, ExtractionPlan, PreparationLimits, PreparedDocument, Projection, Selection,
    SnapshotMetadata, SourceSnapshot,
};
use std::time::Instant;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let file = std::env::args().nth(1).ok_or("fixture path required")?;
    let html = std::fs::read_to_string(file)?;
    let source = PreparedDocument::new(
        SourceSnapshot::new(html, SnapshotMetadata::default())?,
        PreparationLimits::default(),
    )?;
    let mut plan = ExtractionPlan::css("a.item")?;
    plan.selection = Selection::All {
        min: 20,
        max: Some(20),
    };
    plan.projection = Projection::Attribute {
        name: "title".into(),
    };
    let compiled = CompiledPlan::compile(&plan)?;
    let expected = source.execute(&compiled)?.values;
    for _ in 0..3 {
        assert_eq!(source.execute(&compiled)?.values, expected);
    }
    let mut samples = Vec::new();
    for _ in 0..100 {
        let start = Instant::now();
        let values = source.execute(&compiled)?.values;
        let elapsed = start.elapsed().as_nanos();
        assert_eq!(values, expected);
        samples.push(elapsed);
    }
    println!(
        "{}",
        serde_json::json!({"task":"titles","prepared_document_count":1,"compiled_plan_count":1,
        "warmups":3,"repeats":100,"samples_ns":samples,"values":expected,
        "scope":"execution reuse; excludes source reading, compilation and first DOM preparation"})
    );
    Ok(())
}
