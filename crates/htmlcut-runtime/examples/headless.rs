// SPDX-License-Identifier: MPL-2.0
//! A host without terminal parsing, global hooks or process termination.
use htmlcut_runtime::{
    Completion, ContractInput, Destination, Error, ExportFormat, Input, Job, Limits, Operation,
    Outcome, OutlineAttribute, OutlineOptions, OutlineRow,
};
fn attribute_name(attribute: &OutlineAttribute) -> &str {
    &attribute.name
}
fn attribute_names(row: &OutlineRow) -> Vec<&str> {
    row.attributes.iter().map(attribute_name).collect()
}
fn completed<T>(completion: Completion<T>) -> Result<T, Error> {
    match completion {
        Completion::Finished(result) => result,
        Completion::Supervised(error) => Err(error),
    }
}
fn main() -> Result<(), Error> {
    let limits = Limits::default();
    let document = completed(Job::new(limits.time_limit)?.prepare(
        Input::Bytes(b"<main><h1 data-label=title>Snapshot</h1></main>".to_vec()),
        None,
        None,
        limits.clone(),
    ))?;
    let operation = Operation::Extract {
        contract: ContractInput::Json("\"h1\"".into()),
        format: ExportFormat::Json,
    };
    let outcome = completed(Job::new(limits.time_limit)?.execute_prepared(
        &document,
        operation.clone(),
        limits.clone(),
        Destination::Return,
    ))?;
    let Outcome::Returned(result) = outcome else {
        panic!("expected returned extraction")
    };
    assert!(result.value().is_some());
    let bytes = completed(Job::new(limits.time_limit)?.export(&result, limits.clone()))?;
    assert_eq!(bytes, b"\"Snapshot\"\n");
    let outline = completed(Job::new(limits.time_limit)?.execute_prepared(
        &document,
        Operation::Outline(OutlineOptions::default()),
        limits.clone(),
        Destination::Return,
    ))?;
    let Outcome::Returned(result) = outline else {
        panic!("expected returned outline")
    };
    let report = result.outline().unwrap();
    let heading = report.rows.iter().find(|row| row.tag == "h1").unwrap();
    assert_eq!(attribute_names(heading), ["data-label"]);
    let mut smaller = limits.clone();
    smaller.max_output_bytes = 1;
    assert!(
        completed(Job::new(limits.time_limit)?.execute_prepared(
            &document,
            operation.clone(),
            smaller,
            Destination::Return
        ))
        .is_err()
    );
    let job = Job::new(limits.time_limit)?;
    job.control().cancel();
    assert!(
        completed(job.execute_prepared(&document, operation, limits, Destination::Return)).is_err()
    );
    Ok(())
}
