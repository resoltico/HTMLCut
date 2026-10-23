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
            contract: ContractInput::Quick(QueryRequest {
                select: "p".into(),
                read: Reading::default().to_string(),
                cardinality: Match::All,
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
                .code(),
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
            assert_eq!(initial.unwrap_err().code(), ErrorCode::LimitExceeded);
            assert_eq!(saved.unwrap_err().code(), ErrorCode::LimitExceeded);
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

#[test]
fn representations_re_admit_exports_and_new_or_existing_files_before_publication() {
    let nested = r#"{"fields":{"items":{"select":"p","match":"all"},"missing":{"select":"aside","match":"optional"}}}"#;
    let mut outline = OutlineOptions::default();
    outline.select = Some("main".into());
    outline.max_lines = 1;
    let extract = |text: &str, format| Operation::Extract {
        contract: ContractInput::Json(text.into()),
        format,
    };
    let schema = result(Operation::Schema);
    let schema_bytes =
        finished(Job::new(None).unwrap().export(&schema, Limits::default())).unwrap();
    // Schema's existing structural/parity tests remain its independent semantic oracle.
    for (operation, expected, allowance) in [
        (extract("\"p\"", ExportFormat::Json), "\"A \\\"雪\\\"\"\n".as_bytes().to_vec(), 12),
        (extract(nested, ExportFormat::Json), "{\"items\":[\"A \\\"雪\\\"\"],\"missing\":null}\n".as_bytes().to_vec(), 39),
        (extract("\"p\"", ExportFormat::Raw), "A \"雪\"\n".as_bytes().to_vec(), 8),
        (extract(r#"{"select":"p","match":"all"}"#, ExportFormat::Raw), "A \"雪\"\n".as_bytes().to_vec(), 8),
        (extract(r#"{"select":"aside","match":"optional"}"#, ExportFormat::Raw), Vec::new(), 0),
        (Operation::Outline(outline), "1 match for main\n… 2 more lines; narrow the selection, reduce depth or increase the line allowance\n".as_bytes().to_vec(), 101),
        (Operation::Schema, schema_bytes.clone(), schema_bytes.len()),
    ] {
        let request = Request { operation, input:Input::Bytes("<main><p>A &quot;雪&quot;</p></main>".as_bytes().to_vec()), encoding:None, base_url:None, limits:Limits::default(), destination:Destination::Return };
        let Outcome::Returned(computed) = finished(Job::new(None).unwrap().execute(request)).unwrap() else { panic!() };
        let mut limits = Limits::default(); limits.max_output_bytes = allowance;
        assert_eq!(finished(Job::new(None).unwrap().export(&computed, limits.clone())).unwrap(), expected);
        let mut below = limits.clone(); below.max_output_bytes = allowance.saturating_sub(1);
        if allowance > 0 {
        assert_eq!(serde_json::to_value(finished(Job::new(None).unwrap().export(&computed, below.clone())).unwrap_err()).unwrap()["limit"].as_str(), Some("output_bytes"));
        }
        for existing in [false, true] {
            let directory = tempfile::tempdir().unwrap();
            let path = directory.path().join("result");
            if existing { std::fs::write(&path, b"old").unwrap(); }
            let before:Vec<_> = std::fs::read_dir(directory.path()).unwrap().map(|e| e.unwrap().file_name()).collect();
            if allowance > 0 {
            assert_eq!(serde_json::to_value(finished(Job::new(None).unwrap().save(&computed, Destination::File(path.clone()), below.clone())).unwrap_err()).unwrap()["limit"].as_str(), Some("output_bytes"));
            let after:Vec<_> = std::fs::read_dir(directory.path()).unwrap().map(|e| e.unwrap().file_name()).collect();
            assert_eq!(after, before);
            if existing { assert_eq!(std::fs::read(&path).unwrap(), b"old"); }
            }
            finished(Job::new(None).unwrap().save(&computed, Destination::File(path.clone()), limits.clone())).unwrap();
            assert_eq!(std::fs::read(path).unwrap(), expected);
        }
    }
}
