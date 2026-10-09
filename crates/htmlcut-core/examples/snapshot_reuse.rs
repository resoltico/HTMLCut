// SPDX-License-Identifier: MPL-2.0
//! Compile multiple plans once and reuse one immutable prepared snapshot.
use htmlcut_core::{
    CompiledPlan, ExtractionPlan, PreparationLimits, PreparedDocument, Reading, SnapshotMetadata,
    SourceSnapshot,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let source = SourceSnapshot::new(
        "<p id='amount'>EUR 180</p><a href='relative.html'>Link</a>",
        SnapshotMetadata::default(),
    )?;
    let document = PreparedDocument::new(source, PreparationLimits::default())?;
    let amount = CompiledPlan::compile(&ExtractionPlan::css("#amount")?)?;
    let mut link = ExtractionPlan::css("a")?;
    link.read = Some(Reading::Attribute("href".into()));
    let link = CompiledPlan::compile(&link)?;
    for plan in [&amount, &link] {
        println!("{}", serde_json::to_string(document.execute(plan)?.data())?);
    }
    Ok(())
}
