// SPDX-License-Identifier: MPL-2.0
//! A frontend that depends only on the runtime gets bytes without a stream/file bridge.
use htmlcut_runtime::*;
fn finished<T>(result: Completion<T>) -> Result<T, Error> {
    match result {
        Completion::Finished(result) => result,
        Completion::Supervised(_) => panic!("library export unexpectedly supervised"),
    }
}
fn result(operation: Operation) -> Computed {
    let request = Request {
        operation,
        input: Input::Bytes("<main><p>A &amp; 雪</p><p>B</p></main>".as_bytes().to_vec()),
        encoding: None,
        base_url: None,
        limits: Limits::default(),
        destination: Destination::Return,
    };
    let Outcome::Returned(result) = finished(request.job().unwrap().execute(request)).unwrap()
    else {
        panic!("expected typed return")
    };
    result
}
#[test]
fn runtime_only_consumer_gets_complete_json_raw_and_outline_exports() {
    for (format, expected) in [
        (ExportFormat::Json, "[\"A & 雪\",\"B\"]\n"),
        (ExportFormat::Raw, "A & 雪\nB\n"),
    ] {
        let value = result(Operation::Extract {
            contract: ContractInput::Quick(QuickQuery {
                select: "p".into(),
                read: DEFAULT_READING.into(),
                cardinality: QueryCardinality::All,
            }),
            format,
        });
        let bytes = finished(Job::new(None).unwrap().export(&value, Limits::default())).unwrap();
        assert_eq!(bytes, expected.as_bytes());
        let mut smaller = Limits::default();
        smaller.max_output_bytes = 1;
        assert_eq!(
            finished(Job::new(None).unwrap().export(&value, smaller))
                .unwrap_err()
                .error,
            ErrorCode::LimitExceeded
        );
    }
    let mut options = OutlineOptions::default();
    options.select = Some("main".into());
    let outline = result(Operation::Outline(options));
    let bytes = finished(Job::new(None).unwrap().export(&outline, Limits::default())).unwrap();
    assert_eq!(
        bytes,
        "1 match for main\nmain\n  p ×2 \"A & 雪\"\n".as_bytes()
    );
}
#[test]
fn hidden_header_computation_and_later_export_share_the_canonical_allowance() {
    let mut options = OutlineOptions::default();
    options.select = Some(format!("p{}", " ".repeat(200)));
    options.max_lines = 0;
    let original = result(Operation::Outline(options.clone()));
    let expected =
        "… 2 more lines; narrow the selection, reduce depth or increase the line allowance\n"
            .as_bytes();
    for maximum in [expected.len() - 1, expected.len(), 100] {
        let mut limits = Limits::default();
        limits.max_output_bytes = maximum;
        let request = Request {
            operation: Operation::Outline(options.clone()),
            input: Input::Bytes(b"<p>value</p>".to_vec()),
            encoding: None,
            base_url: None,
            limits: limits.clone(),
            destination: Destination::Return,
        };
        let initial = finished(request.job().unwrap().execute(request));
        let saved = finished(Job::new(None).unwrap().export(&original, limits));
        if maximum < expected.len() {
            assert_eq!(initial.unwrap_err().error, ErrorCode::LimitExceeded);
            assert_eq!(saved.unwrap_err().error, ErrorCode::LimitExceeded);
        } else {
            let Outcome::Returned(result) = initial.unwrap() else {
                panic!()
            };
            let bytes =
                finished(Job::new(None).unwrap().export(&result, Limits::default())).unwrap();
            assert_eq!(bytes, expected);
            assert_eq!(saved.unwrap(), expected);
        }
    }
}
