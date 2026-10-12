// SPDX-License-Identifier: MPL-2.0
//! A caller prepares its bytes once and reuses a compiled contract.
use htmlcut_core::{Contract, Document, Limits, Source};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let document = Document::parse(
        Source::new(b"<article id=a><h2>Snapshot</h2><p>12.00</p></article>"),
        &Limits::default(),
    )?;
    let contract = Contract::from_json(
        r#"{"select":"article","fields":{"id":{"read":"attr:id"},"title":"h2","price":{"select":"p","pattern":"^[0-9]+\\.[0-9]{2}$"}}}"#,
    )?;
    let value = document.extract(&contract)?;
    println!("{}", std::str::from_utf8(&value.to_json(65536)?)?);
    Ok(())
}
