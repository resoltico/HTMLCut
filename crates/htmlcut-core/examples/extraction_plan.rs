//! Round-trip a closed plan and print its fully materialized defaults.
use htmlcut_core::{CompiledPlan, ExtractionPlan, canonical_json};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let plan = ExtractionPlan::css("#amount")?;
    let json = canonical_json(&plan)?;
    let round_trip = ExtractionPlan::from_json(json.as_bytes())?;
    assert_eq!(plan, round_trip);
    println!(
        "{}",
        canonical_json(CompiledPlan::compile(&round_trip)?.plan())?
    );
    Ok(())
}
