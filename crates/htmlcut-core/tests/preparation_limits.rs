// SPDX-License-Identifier: MPL-2.0
//! Caller-owned byte and cooperative deadline boundaries through the public API.
use htmlcut_core::{Contract, Document, ErrorCode, Limits, OutlineOptions, Source};
use std::time::Duration;

#[test]
fn input_bytes_admit_equality_and_refuse_every_excess() {
    let bytes = b"<p>snow: \xe9\x9b\xaa</p>";
    for excess in [0, 1, 2] {
        let mut limits = Limits::default();
        limits.max_input_bytes = bytes.len() - excess;
        let result = Document::parse(Source::new(bytes), &limits);
        if excess == 0 {
            let value = result
                .unwrap()
                .extract(&Contract::from_json(r#""p""#).unwrap())
                .unwrap();
            assert_eq!(serde_json::to_value(value).unwrap(), "snow: 雪");
        } else {
            let error = result.unwrap_err();
            assert_eq!(error.error, ErrorCode::LimitExceeded);
            assert_eq!(error.limit.as_deref(), Some("input_bytes"));
            assert_eq!(error.bound, Some((bytes.len() - excess).into()));
        }
    }
}

#[test]
fn library_deadlines_are_enforced_without_a_cli_watchdog() {
    let mut limits = Limits::default();
    limits.time_limit = Some(Duration::ZERO);
    let error = Document::parse(Source::new(b"<p>x</p>"), &limits).unwrap_err();
    assert_eq!(error.error, ErrorCode::LimitExceeded);
    assert_eq!(error.limit.as_deref(), Some("time"));
    let mut unlimited = Limits::default();
    unlimited.time_limit = None;
    let document = Document::parse(Source::new(b"<p>x</p>"), &unlimited).unwrap();
    assert_eq!(
        serde_json::to_value(
            document
                .extract(&Contract::from_json(r#""p""#).unwrap())
                .unwrap()
        )
        .unwrap(),
        "x"
    );
    assert!(
        !document
            .outline(&OutlineOptions::default())
            .unwrap()
            .is_empty()
    );
}
