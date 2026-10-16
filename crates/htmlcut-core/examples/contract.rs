// SPDX-License-Identifier: MPL-2.0
//! A caller prepares its bytes once and reuses a compiled contract.
use htmlcut_core::{Contract, Document, Limits, Source};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let limits = Limits::default();
    let context = htmlcut_core::ExecutionContext::new(limits.time_limit)?;
    let contract = Contract::from_json(
        r#"{"select":"article","fields":{"id":{"read":"attr:id"},"title":"h2","price":{"select":"p","pattern":"^[0-9]+\\.[0-9]{2}$"}}}"#,
        &context,
    )?;
    let document = Document::parse(
        Source::new(b"<article id=a><h2>Snapshot</h2><p>12.00</p></article>"),
        &limits.preparation(),
        &context,
    )?;
    let value = document.extract(&contract, &limits.execution(), &context)?;
    use std::io::Write;
    std::io::stdout().write_all(&value.export(
        htmlcut_core::ExportFormat::Json,
        limits.max_output_bytes,
        &context,
    )?)?;
    Ok(())
}
