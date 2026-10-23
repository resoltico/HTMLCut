// SPDX-License-Identifier: MPL-2.0
use htmlcut_runtime::*;
use std::{
    io::Write,
    process::{Command, Stdio},
};
fn finished<T>(completion: Completion<T>) -> T {
    match completion {
        Completion::Finished(Ok(value)) => value,
        Completion::Finished(Err(error)) | Completion::Supervised(error) => panic!("{error}"),
    }
}
#[test]
fn returned_runtime_bytes_match_the_delivered_cli_for_every_representation() {
    let binary = std::env::var_os("HTMLCUT_TEST_BINARY")
        .unwrap_or_else(|| env!("CARGO_BIN_EXE_htmlcut").into());
    let html = "<main><p>A &amp; 雪</p><p>B</p></main>";
    let mut outline = OutlineOptions::default();
    outline.select = Some("main".into());
    let cases = [
        (
            Operation::Extract {
                contract: ContractInput::Quick(QueryRequest {
                    select: "p".into(),
                    read: Reading::default().to_string(),
                    cardinality: Match::All,
                }),
                format: ExportFormat::Json,
            },
            vec!["get", "p", "--all"],
        ),
        (
            Operation::Extract {
                contract: ContractInput::Quick(QueryRequest {
                    select: "p".into(),
                    read: Reading::default().to_string(),
                    cardinality: Match::All,
                }),
                format: ExportFormat::Raw,
            },
            vec!["get", "p", "--all", "--raw"],
        ),
        (
            Operation::Outline(outline),
            vec!["outline", "--select", "main"],
        ),
    ];
    for (operation, args) in cases {
        let request = Request {
            operation,
            input: Input::Bytes(html.as_bytes().to_vec()),
            encoding: None,
            base_url: None,
            limits: Limits::default(),
            destination: Destination::Return,
        };
        let Outcome::Returned(value) = finished(request.job().unwrap().execute(request)) else {
            panic!("expected typed value")
        };
        let expected = finished(Job::new(None).unwrap().export(&value, Limits::default()));
        let mut child = Command::new(&binary)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(html.as_bytes())
            .unwrap();
        let output = child.wait_with_output().unwrap();
        assert!(output.status.success(), "{output:?}");
        assert!(output.stderr.is_empty());
        assert_eq!(output.stdout, expected);
    }
}

#[test]
fn schema_bytes_and_output_admission_match_the_delivered_cli() {
    let binary = std::env::var_os("HTMLCUT_TEST_BINARY")
        .unwrap_or_else(|| env!("CARGO_BIN_EXE_htmlcut").into());
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
    let Outcome::Returned(value) = finished(Job::new(None).unwrap().execute(request(usize::MAX)))
    else {
        panic!("expected schema value")
    };
    let expected = finished(Job::new(None).unwrap().export(&value, Limits::default()));
    assert_eq!(expected.last(), Some(&b'\n'));
    for maximum in [expected.len(), expected.len() - 1] {
        let runtime = match Job::new(None).unwrap().execute(request(maximum)) {
            Completion::Finished(result) => result,
            Completion::Supervised(error) => Err(error),
        };
        let cli = Command::new(&binary)
            .args(["schema", "--max-output-bytes"])
            .arg(maximum.to_string())
            .stdin(Stdio::null())
            .output()
            .unwrap();
        if maximum == expected.len() {
            let Outcome::Returned(value) = runtime.unwrap() else {
                panic!("expected admitted schema")
            };
            assert_eq!(
                finished(
                    Job::new(None)
                        .unwrap()
                        .export(&value, request(maximum).limits)
                ),
                expected
            );
            assert!(cli.status.success(), "{cli:?}");
            assert_eq!(cli.stdout, expected);
            assert!(cli.stderr.is_empty());
        } else {
            let error = match runtime {
                Err(error) => error,
                Ok(_) => panic!("schema exceeded its complete output allowance"),
            };
            assert_eq!(error.code(), ErrorCode::LimitExceeded);
            assert_eq!(
                serde_json::to_value(&error).unwrap()["limit"].as_str(),
                Some("output_bytes")
            );
            assert_eq!(cli.status.code(), Some(4), "{cli:?}");
            assert!(cli.stdout.is_empty());
            assert_eq!(cli.stderr, error.to_json_line());
        }
    }
}
