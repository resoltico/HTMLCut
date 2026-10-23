// SPDX-License-Identifier: MPL-2.0
use htmlcut_runtime::*;
use std::{sync::Arc, time::Duration};
fn finished<T>(result: Completion<T>) -> Result<T, Error> {
    match result {
        Completion::Finished(result) => result,
        Completion::Supervised(_) => panic!("library use entered supervisor ownership"),
    }
}
fn query(format: ExportFormat) -> Operation {
    Operation::Extract {
        contract: ContractInput::Quick(QuickQuery {
            select: "p".into(),
            read: "text".into(),
            cardinality: QueryCardinality::One,
        }),
        format,
    }
}
fn document() -> PreparedDocument {
    finished(Job::new(None).unwrap().prepare(
        Input::Bytes(b"<p>snow: \xe9\x9b\xaa</p>".to_vec()),
        None,
        None,
        Limits::default(),
    ))
    .unwrap()
}
#[test]
fn exact_immutable_types_have_safe_auto_traits() {
    fn shared<T: Send + Sync>() {}
    shared::<PreparedDocument>();
    shared::<Computed>();
    shared::<Outcome>();
    shared::<Value>();
}
#[test]
fn workers_share_only_semantic_state_and_keep_policy_and_cancellation_independent() {
    let document = Arc::new(document());
    let cancelled = Job::new(None).unwrap();
    cancelled.control().cancel();
    let other = document.clone();
    let worker = std::thread::spawn(move || {
        let mut limits = Limits::default();
        limits.max_output_bytes = 1;
        finished(Job::new(None).unwrap().execute_prepared(
            &other,
            query(ExportFormat::Json),
            limits,
            Destination::Return,
        ))
        .unwrap_err()
        .code()
    });
    let error = finished(cancelled.execute_prepared(
        &document,
        query(ExportFormat::Json),
        Limits::default(),
        Destination::Return,
    ))
    .unwrap_err();
    assert_eq!(error.code(), ErrorCode::Cancelled);
    assert!(serde_json::to_value(&error).unwrap().get("limit").is_none());
    assert!(
        serde_json::to_value(&error)
            .unwrap()
            .get("bound")
            .cloned()
            .is_none()
    );
    assert_eq!(worker.join().unwrap(), ErrorCode::LimitExceeded);
    let result = finished(Job::new(None).unwrap().execute_prepared(
        &document,
        query(ExportFormat::Json),
        Limits::default(),
        Destination::Return,
    ))
    .unwrap();
    assert!(
        matches!(result, Outcome::Returned(result) if result.value() == Some(&Value::String("snow: 雪".into())))
    );
}
#[test]
fn later_save_rechecks_canonical_admission_including_null_and_framing() {
    let mut limits = Limits::default();
    limits.max_output_bytes = 5;
    let operation = Operation::Extract {
        contract: ContractInput::Json(r#"{"select":"aside","match":"optional"}"#.into()),
        format: ExportFormat::Raw,
    };
    let Outcome::Returned(result) = finished(Job::new(None).unwrap().execute_prepared(
        &document(),
        operation,
        limits.clone(),
        Destination::Return,
    ))
    .unwrap() else {
        panic!()
    };
    limits.max_output_bytes = 4;
    let root = tempfile::tempdir().unwrap();
    let output = root.path().join("result");
    std::fs::write(&output, b"old").unwrap();
    let error = finished(Job::new(None).unwrap().save(
        &result,
        Destination::File(output.clone()),
        limits,
    ))
    .unwrap_err();
    assert_eq!(error.code(), ErrorCode::LimitExceeded);
    assert_eq!(std::fs::read(output).unwrap(), b"old");
}
#[test]
fn invalid_export_and_contract_refuse_before_missing_html_and_zero_jobs_do_no_lookup() {
    for (contract, format, expected) in [
        ("[", ExportFormat::Json, ErrorCode::InvalidContract),
        (
            r#"{"fields":{"x":{}}}"#,
            ExportFormat::Raw,
            ErrorCode::UnrepresentableOutput,
        ),
    ] {
        let result = finished(Job::new(None).unwrap().execute(Request {
            operation: Operation::Extract {
                contract: ContractInput::Json(contract.into()),
                format,
            },
            input: Input::File("missing-html".into()),
            encoding: None,
            base_url: None,
            limits: Limits::default(),
            destination: Destination::Return,
        }));
        assert_eq!(result.unwrap_err().code(), expected);
    }
    assert_eq!(
        Job::new(Some(Duration::MAX)).err().unwrap().code(),
        ErrorCode::InvalidRequest
    );
    let error = finished(Job::new(Some(Duration::ZERO)).unwrap().prepare(
        Input::File("missing-html".into()),
        None,
        None,
        Limits::default(),
    ))
    .unwrap_err();
    assert_eq!(
        serde_json::to_value(&error).unwrap()["limit"].as_str(),
        Some("time")
    );
}
#[test]
fn structured_outline_counts_hidden_header_and_rows() {
    for lines in [0, 1, 2, 80] {
        let mut options = OutlineOptions::default();
        options.select = Some("p".into());
        options.max_lines = lines;
        let Outcome::Returned(result) = finished(Job::new(None).unwrap().execute_prepared(
            &document(),
            Operation::Outline(options),
            Limits::default(),
            Destination::Return,
        ))
        .unwrap() else {
            panic!()
        };
        let report = result.outline().unwrap();
        assert_eq!(report.selection_count, Some(1));
        assert_eq!(report.header_emitted, lines > 0);
        assert_eq!(report.omitted_lines, 2usize.saturating_sub(lines));
        if lines > 1 {
            assert_eq!(report.rows[0].tag, "p");
            assert_eq!(report.rows[0].preview, "snow: 雪");
        }
    }
}

#[test]
fn explicit_file_sources_and_destinations_do_not_interpret_shell_markers() {
    const CHILD: &str = "HTMLCUT_LITERAL_DASH_CHILD";
    if std::env::var_os(CHILD).is_some() {
        std::fs::write("-", b"<p>snow: \xe9\x9b\xaa</p>").unwrap();
        let request = Request {
            operation: query(ExportFormat::Json),
            input: Input::File("-".into()),
            encoding: None,
            base_url: None,
            limits: Limits::default(),
            destination: Destination::File("-".into()),
        };
        let result = finished(request.job().unwrap().execute(request)).unwrap();
        assert!(matches!(result, Outcome::Delivered(Delivery::Delivered)));
        assert_eq!(std::fs::read("-").unwrap(), b"\"snow: \xe9\x9b\xaa\"\n");
        return;
    }
    let root = tempfile::tempdir().unwrap();
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "explicit_file_sources_and_destinations_do_not_interpret_shell_markers",
            "--nocapture",
        ])
        .env(CHILD, "1")
        .current_dir(root.path())
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 1);
}
#[test]
fn syntax_conflicts_refuse_before_a_request_job_deadline_starts() {
    let mut limits = Limits::default();
    limits.time_limit = Some(Duration::ZERO);
    let request = Request {
        operation: Operation::Extract {
            contract: ContractInput::Stdin,
            format: ExportFormat::Json,
        },
        input: Input::Stdin {
            allow_terminal: true,
        },
        encoding: None,
        base_url: None,
        limits,
        destination: Destination::Return,
    };
    assert_eq!(
        request.job().err().unwrap().code(),
        ErrorCode::InvalidRequest
    );
}
#[test]
fn schema_admits_exact_framing_bytes_and_refuses_one_below() {
    let request = |maximum| {
        let mut limits = Limits::default();
        limits.max_output_bytes = maximum;
        Request {
            operation: Operation::Schema,
            input: Input::Bytes(Vec::new()),
            encoding: None,
            base_url: None,
            limits,
            destination: Destination::Return,
        }
    };
    let result = finished(Job::new(None).unwrap().execute(request(usize::MAX))).unwrap();
    let Outcome::Returned(result) = result else {
        panic!()
    };
    assert_eq!(result.schema().unwrap().last(), Some(&b'\n'));
    let root = tempfile::tempdir().unwrap();
    let output = root.path().join("schema.json");
    finished(Job::new(None).unwrap().save(
        &result,
        Destination::File(output.clone()),
        Limits::default(),
    ))
    .unwrap();
    let bytes = std::fs::read(output).unwrap();
    assert_eq!(bytes.last(), Some(&b'\n'));
    let mut compact =
        serde_json::to_vec(&serde_json::from_slice::<serde_json::Value>(&bytes).unwrap()).unwrap();
    compact.push(b'\n');
    assert_eq!(compact, bytes);
    assert!(finished(Job::new(None).unwrap().execute(request(bytes.len()))).is_ok());
    assert_eq!(
        finished(Job::new(None).unwrap().execute(request(bytes.len() - 1)))
            .unwrap_err()
            .code(),
        ErrorCode::LimitExceeded
    );
}

#[test]
fn oversized_outline_selection_refuses_before_source_lookup_or_diagnostic_copy() {
    let mut options = OutlineOptions::default();
    options.select = Some("x".repeat(1024 * 1024));
    let request = Request {
        operation: Operation::Outline(options),
        input: Input::File("missing-html".into()),
        encoding: None,
        base_url: None,
        limits: Limits::default(),
        destination: Destination::Return,
    };
    let error = finished(Job::new(None).unwrap().execute(request)).unwrap_err();
    assert_eq!(error.code(), ErrorCode::InvalidSelector);
    assert!(
        serde_json::to_value(&error)
            .unwrap()
            .get("select")
            .is_none()
    );
    assert!(serde_json::to_value(&error).unwrap().get("file").is_none());
}

#[test]
fn prepared_handle_debug_is_bounded_and_keeps_engine_state_opaque() {
    let document = finished(Job::new(None).unwrap().prepare(
        Input::Bytes(b"<p id=private-prepared-marker>unexported document text</p>".to_vec()),
        None,
        None,
        Limits::default(),
    ))
    .unwrap();
    let debug = format!("{document:?}");
    assert!(debug.starts_with("PreparedDocument"));
    assert!(debug.len() < 128);
    assert!(!debug.contains("private-prepared-marker"));
    assert!(!debug.contains("unexported document text"));
}
