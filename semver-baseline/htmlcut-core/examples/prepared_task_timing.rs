// SPDX-License-Identifier: MPL-2.0
//! Caller-owned in-process timing over the same immutable title-extraction task as the Python comparison.
use htmlcut_core::{
    CompiledPlan, ExtractionPlan, Match, PreparationLimits, PreparedDocument, Reading,
    SnapshotMetadata, SourceSnapshot,
};
use std::time::Instant;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let file = std::env::args().nth(1).ok_or("fixture path required")?;
    let selector = std::env::args().nth(2).ok_or("title selector required")?;
    let html = std::fs::read_to_string(file)?;
    let source = PreparedDocument::new(
        SourceSnapshot::new(html, SnapshotMetadata::default())?,
        PreparationLimits::default(),
    )?;
    let mut plan = ExtractionPlan::css(selector)?;
    plan.match_mode = Match::All;
    plan.min = Some(20);
    plan.max = Some(20);
    plan.read = Some(Reading::Attribute("title".into()));
    let compiled = CompiledPlan::compile(&plan)?;
    let expected = source.execute(&compiled)?;
    for _ in 0..3 {
        assert_eq!(
            source.execute(&compiled)?.data().as_values().unwrap(),
            expected.data().as_values().unwrap()
        );
    }
    let mut samples = Vec::new();
    for _ in 0..100 {
        let start = Instant::now();
        let values = source.execute(&compiled)?;
        let elapsed = start.elapsed().as_nanos();
        assert_eq!(values.data(), expected.data());
        samples.push(elapsed);
    }
    println!(
        "{}",
        serde_json::json!({"task":"titles","prepared_document_count":1,"compiled_plan_count":1,
        "warmups":3,"repeats":100,"samples_ns":samples,"values":expected.data(),
        "scope":"execution reuse; excludes source reading, compilation and first DOM preparation"})
    );
    Ok(())
}
