// SPDX-License-Identifier: MPL-2.0
//! Compile multiple plans once and reuse one immutable prepared snapshot.
use htmlcut_core::{
    Boundary, CompiledPlan, ExtractionPlan, PreparationLimits, PreparedDocument, Projection,
    SnapshotMetadata, SourceSnapshot, ValueProjection, canonical_json,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let source = SourceSnapshot::new(
        "<p id='amount'>EUR 180</p><a href='relative.html'>Link</a>START\r\n✓\r\nEND",
        SnapshotMetadata::default(),
    )?;
    let document = PreparedDocument::new(source, PreparationLimits::default())?;
    let amount = CompiledPlan::compile(&ExtractionPlan::css("#amount")?)?;
    let mut link = ExtractionPlan::css("a")?;
    link.projection = Projection::Value(ValueProjection::Attribute {
        name: "href".into(),
    });
    let link = CompiledPlan::compile(&link)?;
    let slice = CompiledPlan::compile(&ExtractionPlan::slice(
        Boundary::Literal {
            value: "START".into(),
        },
        Boundary::Literal {
            value: "END".into(),
        },
    )?)?;
    for plan in [&amount, &link, &slice] {
        println!("{}", canonical_json(&document.execute(plan)?.data)?);
    }
    Ok(())
}
