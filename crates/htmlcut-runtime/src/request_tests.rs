// SPDX-License-Identifier: MPL-2.0
use crate::*;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
fn finished<T>(completion: Completion<T>) -> Result<T, Error> {
    match completion {
        Completion::Finished(result) => result,
        Completion::Supervised(_) => panic!("library request unexpectedly supervised"),
    }
}
#[test]
fn malformed_or_schema_requests_never_inspect_irrelevant_html_metadata() {
    let operations = [
        (
            Operation::Extract {
                contract: ContractInput::Json("[".into()),
                format: ExportFormat::Json,
            },
            Some(ErrorCode::InvalidContract),
        ),
        (
            Operation::Extract {
                contract: ContractInput::Quick(QueryRequest {
                    select: "[".into(),
                    read: Reading::default().to_string(),
                    cardinality: Match::One,
                }),
                format: ExportFormat::Json,
            },
            Some(ErrorCode::InvalidSelector),
        ),
        (
            Operation::Extract {
                contract: ContractInput::Json(r#"{"fields":{"value":{}}}"#.into()),
                format: ExportFormat::Raw,
            },
            Some(ErrorCode::UnrepresentableOutput),
        ),
        (Operation::Schema, None),
    ];
    for (operation, expected) in operations {
        let lookups = Arc::new(AtomicUsize::new(0));
        let observed = lookups.clone();
        let result = crate::local_io::with_link_lookup_probe(
            move |_| {
                observed.fetch_add(1, Ordering::Relaxed);
                Err(std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    "controlled HTML metadata lookup failure",
                ))
            },
            || {
                finished(Job::new(None).unwrap().execute(Request {
                    operation,
                    input: Input::File("irrelevant-html-source".into()),
                    encoding: None,
                    base_url: None,
                    limits: Limits::default(),
                    destination: Destination::Return,
                }))
            },
        );
        if let Some(expected) = expected {
            assert_eq!(result.unwrap_err().code(), expected);
        } else {
            assert!(
                matches!(result.unwrap(),Outcome::Returned(result) if result.schema().is_some())
            );
        }
        assert_eq!(
            lookups.load(Ordering::Relaxed),
            0,
            "preflight inspected HTML before refusing or computing schema"
        );
    }
}
