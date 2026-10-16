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
                contract: ContractInput::Quick(QuickQuery {
                    select: "p".into(),
                    read: DEFAULT_READING.into(),
                    cardinality: QueryCardinality::All,
                }),
                format: ExportFormat::Json,
            },
            vec!["get", "p", "--all"],
        ),
        (
            Operation::Extract {
                contract: ContractInput::Quick(QuickQuery {
                    select: "p".into(),
                    read: DEFAULT_READING.into(),
                    cardinality: QueryCardinality::All,
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
