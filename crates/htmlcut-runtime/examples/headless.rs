// SPDX-License-Identifier: MPL-2.0
//! A host without terminal parsing, global hooks or process termination.
use htmlcut_runtime::{
    Completion, ContractInput, Destination, Error, ExportFormat, Input, Job, Limits, Operation,
    Outcome, OutlineOptions,
};
fn completed<T>(completion: Completion<T>) -> Result<T, Error> {
    match completion {
        Completion::Finished(result) => result,
        Completion::Supervised(error) => Err(error),
    }
}
fn main() -> Result<(), Error> {
    let limits = Limits::default();
    let document = completed(Job::new(limits.time_limit)?.prepare(
        Input::Bytes(b"<main><h1>Snapshot</h1></main>".to_vec()),
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
    if let Outcome::Returned(result) = outcome {
        assert!(result.value().is_some());
    }
    let outline = completed(Job::new(limits.time_limit)?.execute_prepared(
        &document,
        Operation::Outline(OutlineOptions::default()),
        limits.clone(),
        Destination::Return,
    ))?;
    if let Outcome::Returned(result) = outline {
        assert!(!result.outline().unwrap().rows.is_empty());
    }
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
