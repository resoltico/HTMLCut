// SPDX-License-Identifier: MPL-2.0
use htmlcut_core::{
    Contract, Document, Error, ExportFormat, Limit, ParseOptions, ResultLimits, Stop,
};
#[test]
fn emitted_byte_boundaries_agree_during_extraction_and_export() {
    let stop = Stop::none();
    for (page, contract, json, raw) in [
        ("<p>雪</p>", r#""p""#, "\"雪\"\n", "雪\n"),
        (
            "<p>a</p><p>雪</p>",
            r#"{"select":"p","match":"all"}"#,
            "[\"a\",\"雪\"]\n",
            "a\n雪\n",
        ),
        ("", r#"{"select":"p","match":"optional"}"#, "null\n", ""),
    ] {
        let document = Document::parse(page.as_bytes(), &ParseOptions::default(), &stop).unwrap();
        let contract = Contract::from_json(contract, &stop).unwrap();
        for (format, expected) in [(ExportFormat::Json, json), (ExportFormat::Raw, raw)] {
            let limits = ResultLimits {
                max_output_bytes: expected.len(),
                ..Default::default()
            };
            let value = document.extract(&contract, format, &limits, &stop).unwrap();
            assert_eq!(
                value.export(format, expected.len(), &stop).unwrap(),
                expected.as_bytes()
            );
            if !expected.is_empty() {
                let limits = ResultLimits {
                    max_output_bytes: expected.len() - 1,
                    ..Default::default()
                };
                for error in [
                    document
                        .extract(&contract, format, &limits, &stop)
                        .unwrap_err(),
                    value.export(format, expected.len() - 1, &stop).unwrap_err(),
                ] {
                    assert!(matches!(
                        error,
                        Error::LimitExceeded {
                            limit: Limit::OutputBytes,
                            ..
                        }
                    ));
                }
            }
        }
    }
}
#[test]
fn stop_is_shareable_and_refuses_before_document_admission() {
    fn traits<T: Clone + Send + Sync>() {}
    traits::<Stop>();
    assert!(matches!(
        Document::parse(
            b"invalid",
            &ParseOptions::default(),
            &Stop::after(std::time::Duration::ZERO).unwrap()
        ),
        Err(Error::LimitExceeded {
            limit: Limit::Time,
            ..
        })
    ));
    let flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
    assert!(matches!(
        Document::parse(
            b"",
            &ParseOptions::default(),
            &Stop::none().with_cancel(flag)
        ),
        Err(Error::Cancelled)
    ));
}
