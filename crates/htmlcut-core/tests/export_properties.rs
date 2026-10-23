use support::{DocumentFixture, TestOptions, parse_fixture};
mod support;
// SPDX-License-Identifier: MPL-2.0
// Bounded composed scalar cases with an oracle independent of materialization accounting.
use htmlcut_core::{Contract, ErrorCode, ExportFormat, Value};

#[test]
fn composed_scalar_lists_preserve_values_and_exact_complete_export_bounds() {
    let pieces = [
        "", "a", "雪", "🦀", "&", "<", ">", "\"", "'", "\\", "\t", "\n", "\u{2028}",
    ];
    let context = support::test_stop(None).unwrap();
    let contract = Contract::from_json(
        r#"{"select":"p","match":"all","min":0,"read":"text-content"}"#,
        &context,
    )
    .unwrap();
    let limits = TestOptions::default();
    for case in 0..128 {
        let expected: Vec<String> = (0..case % 5)
            .map(|row| {
                (0..(case / 5 + row) % 7)
                    .map(|part| pieces[(case * 7 + row * 3 + part) % pieces.len()])
                    .collect()
            })
            .collect();
        let html = expected
            .iter()
            .map(|value| {
                let escaped = value
                    .replace('&', "&amp;")
                    .replace('<', "&lt;")
                    .replace('>', "&gt;");
                format!("<p>{escaped}</p>")
            })
            .collect::<String>();
        let document = parse_fixture(
            DocumentFixture::new(html.as_bytes()),
            &limits.preparation(),
            &context,
        )
        .unwrap();
        let mut json = serde_json::to_vec(&expected).unwrap();
        json.push(b'\n');
        let mut policy = limits.execution();
        policy.max_output_bytes = json.len();
        let value = document
            .extract(
                &contract,
                htmlcut_core::ExportFormat::Json,
                &policy,
                &context,
            )
            .unwrap_or_else(|error| panic!("case {case}, input {html:?}: {error}"));
        assert_eq!(
            value,
            Value::List(expected.iter().cloned().map(Value::String).collect()),
            "scalar oracle case {case}, input {html:?}"
        );
        assert_eq!(
            value
                .export(ExportFormat::Json, json.len(), &context)
                .unwrap(),
            json,
            "JSON case {case}"
        );
        policy.max_output_bytes -= 1;
        let error = document
            .extract(
                &contract,
                htmlcut_core::ExportFormat::Json,
                &policy,
                &context,
            )
            .unwrap_err();
        assert_eq!(
            error.code(),
            ErrorCode::LimitExceeded,
            "one-below case {case}"
        );
        assert_eq!(
            serde_json::to_value(&error).unwrap()["limit"].as_str(),
            Some("output_bytes")
        );
        assert_eq!(
            serde_json::to_value(&error).unwrap().get("bound").cloned(),
            Some(policy.max_output_bytes.into())
        );
        assert_eq!(
            serde_json::to_value(
                value
                    .export(ExportFormat::Json, json.len() - 1, &context)
                    .unwrap_err()
            )
            .unwrap()["limit"]
                .as_str(),
            Some("output_bytes")
        );
        match expected.iter().position(|value| value.contains('\n')) {
            Some(row) => {
                let error = value
                    .export(ExportFormat::Raw, json.len(), &context)
                    .unwrap_err();
                assert_eq!(
                    error.code(),
                    ErrorCode::UnrepresentableOutput,
                    "raw case {case}"
                );
                assert_eq!(
                    serde_json::from_value::<Option<Vec<usize>>>(
                        serde_json::to_value(&error).unwrap()["row"].clone()
                    )
                    .unwrap(),
                    Some(vec![row + 1])
                );
            }
            None => {
                let raw = expected
                    .iter()
                    .map(|value| format!("{value}\n"))
                    .collect::<String>();
                assert_eq!(
                    value
                        .export(ExportFormat::Raw, json.len(), &context)
                        .unwrap(),
                    raw.as_bytes(),
                    "raw case {case}"
                );
            }
        }
    }
}
