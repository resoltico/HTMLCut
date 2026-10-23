use support::{DocumentFixture, TestOptions, parse_fixture};
mod support;
// SPDX-License-Identifier: MPL-2.0
// Caller-owned byte and cooperative deadline boundaries through the public API.
use htmlcut_core::{Contract, ErrorCode, OutlineOptions};
use std::time::Duration;

#[test]
fn input_bytes_admit_equality_and_refuse_every_excess() {
    let bytes = b"<p>snow: \xe9\x9b\xaa</p>";
    for excess in [0, 1, 2] {
        let limits = TestOptions {
            max_input_bytes: bytes.len() - excess,
            ..Default::default()
        };
        let result = parse_fixture(
            DocumentFixture::new(bytes),
            &limits.preparation(),
            &support::test_stop(limits.time_limit).unwrap(),
        );
        if excess == 0 {
            let value = result
                .unwrap()
                .extract(
                    &Contract::from_json(r#""p""#, &support::test_stop(None).unwrap()).unwrap(),
                    htmlcut_core::ExportFormat::Json,
                    &limits.execution(),
                    &support::test_stop((limits).time_limit).unwrap(),
                )
                .unwrap();
            assert_eq!(serde_json::to_value(value).unwrap(), "snow: 雪");
        } else {
            let error = result.unwrap_err();
            assert_eq!(error.code(), ErrorCode::LimitExceeded);
            assert_eq!(
                serde_json::to_value(&error).unwrap()["limit"].as_str(),
                Some("input_bytes")
            );
            assert_eq!(
                serde_json::to_value(&error).unwrap().get("bound").cloned(),
                Some((bytes.len() - excess).into())
            );
        }
    }
}

#[test]
fn library_deadlines_are_enforced_without_a_cli_watchdog() {
    let limits = TestOptions {
        time_limit: Some(Duration::ZERO),
        ..Default::default()
    };
    let error = parse_fixture(
        DocumentFixture::new(b"<p>x</p>"),
        &limits.preparation(),
        &support::test_stop(limits.time_limit).unwrap(),
    )
    .unwrap_err();
    assert_eq!(error.code(), ErrorCode::LimitExceeded);
    assert_eq!(
        serde_json::to_value(&error).unwrap()["limit"].as_str(),
        Some("time")
    );
    let unlimited = TestOptions {
        time_limit: None,
        ..Default::default()
    };
    let document = parse_fixture(
        DocumentFixture::new(b"<p>x</p>"),
        &unlimited.preparation(),
        &support::test_stop(unlimited.time_limit).unwrap(),
    )
    .unwrap();
    assert_eq!(
        serde_json::to_value(
            document
                .extract(
                    &Contract::from_json(r#""p""#, &support::test_stop(None).unwrap()).unwrap(),
                    htmlcut_core::ExportFormat::Json,
                    &unlimited.execution(),
                    &support::test_stop((unlimited).time_limit).unwrap()
                )
                .unwrap()
        )
        .unwrap(),
        "x"
    );
    assert!(
        !document
            .outline(
                &OutlineOptions::default(),
                &unlimited.execution(),
                &support::test_stop((unlimited).time_limit).unwrap()
            )
            .and_then(|report| report.render(
                unlimited.max_output_bytes,
                &support::test_stop((unlimited).time_limit).unwrap()
            ))
            .unwrap()
            .is_empty()
    );
}
