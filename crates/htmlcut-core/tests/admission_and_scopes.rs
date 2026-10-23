use support::{DocumentFixture, TestOptions, parse_fixture};
mod support;
// SPDX-License-Identifier: MPL-2.0
// Independent literal oracles for the reviewed range and source admission contracts.
use htmlcut_core::{Contract, Value};
fn extract(html: &str, contract: &str, limits: &TestOptions) -> Result<Value, htmlcut_core::Error> {
    parse_fixture(
        DocumentFixture::new(html.as_bytes()),
        &(limits).preparation(),
        &support::test_stop((limits).time_limit).unwrap(),
    )?
    .extract(
        &Contract::from_json(contract, &support::test_stop(None).unwrap())?,
        htmlcut_core::ExportFormat::Json,
        &limits.execution(),
        &support::test_stop((limits).time_limit).unwrap(),
    )
}
#[test]
fn inherited_range_contains_children_but_preserves_real_css_context() {
    let html = "<main><h2 id=a>A</h2><!--c--><p>one</p><h2 id=b>B</h2><p>two</p><footer>end</footer></main>";
    let limits = TestOptions::default();
    let contract = r#"{"select":"h2","match":"all","until":"h2","fields":{"body":{"select":":scope","until":"footer"},"whole":{},"heading":{"select":":scope"},"neighbor":{"select":":scope:has(+ p)","read":"attr:id","match":"optional"}}}"#;
    assert_eq!(extract(html,contract,&limits).unwrap().export(htmlcut_core::ExportFormat::Json, 1024, &support::test_stop(None).unwrap()).unwrap(),br#"[{"body":"A\none","heading":"A","neighbor":"a","whole":"A\none"},{"body":"B\ntwo","heading":"B","neighbor":"b","whole":"B\ntwo\nend"}]
"#);
    let contract = r#"{"select":"h2","nth":1,"until":"h2","fields":{"outside":{"select":"h2:nth-child(3)","match":"optional"}}}"#;
    assert_eq!(
        extract(html, contract, &limits)
            .unwrap()
            .export(
                htmlcut_core::ExportFormat::Json,
                1024,
                &support::test_stop(None).unwrap()
            )
            .unwrap(),
        b"{\"outside\":null}\n"
    );
}
#[test]
fn final_section_without_stop_ends_naturally_at_the_parent_boundary() {
    assert_eq!(
        extract(
            "<h2>A</h2><p>body</p>",
            r#"{"select":"h2","until":"footer"}"#,
            &TestOptions::default()
        )
        .unwrap(),
        Value::String("A\nbody".into())
    );
}
#[test]
fn source_reading_limit_is_independent_of_final_capture_and_predicates() {
    let html = "<p>aaaaax</p>";
    let contract = r#"{"select":"p","pattern":"(?<value>x)$"}"#;
    let mut limits = TestOptions {
        max_reading_bytes: 6,
        max_output_bytes: 4,
        ..Default::default()
    };
    assert_eq!(
        extract(html, contract, &limits).unwrap(),
        Value::String("x".into())
    );
    limits.max_reading_bytes = 5;
    assert_eq!(
        serde_json::to_value(extract(html, contract, &limits).unwrap_err()).unwrap()["limit"]
            .as_str(),
        Some("reading_bytes")
    );
    limits.max_reading_bytes = 6;
    assert_eq!(extract(html,r#"{"select":"p","expect":[{"select":"p","equals":"aaaaax"}],"pattern":"(?<value>x)$"}"#,&limits).unwrap(),Value::String("x".into()));
}
#[test]
fn discarded_whitespace_never_consumes_transformed_reading_admission() {
    let html = format!("<p>{}x{}</p>", " ".repeat(1_000_000), " ".repeat(1_000_000));
    let limits = TestOptions {
        max_reading_bytes: 1,
        max_output_bytes: 4,
        ..Default::default()
    };
    assert_eq!(
        extract(&html, r#""p""#, &limits).unwrap(),
        Value::String("x".into())
    );
    assert_eq!(
        serde_json::to_value(
            extract(&html, r#"{"select":"p","read":"text-content"}"#, &limits).unwrap_err()
        )
        .unwrap()["limit"]
            .as_str(),
        Some("reading_bytes")
    );
}
#[test]
fn malformed_url_with_base_reports_its_actual_parse_cause() {
    let mut source = DocumentFixture::new(b"<a href='https://['>bad</a>");
    source.base_url = Some("https://good.test/");
    let document = parse_fixture(
        source,
        &TestOptions::default().preparation(),
        &support::test_stop(TestOptions::default().time_limit).unwrap(),
    )
    .unwrap();
    let error = document
        .extract(
            &Contract::from_json(
                r#"{"select":"a","read":"url:href"}"#,
                &support::test_stop(None).unwrap(),
            )
            .unwrap(),
            htmlcut_core::ExportFormat::Json,
            &support::TestOptions::default().execution(),
            &support::test_stop((support::TestOptions::default()).time_limit).unwrap(),
        )
        .unwrap_err();
    assert!(error.to_string().contains("malformed URL"));
    assert!(!error.to_string().contains("without a usable"));
    assert_eq!(
        serde_json::to_value(&error).unwrap()["path"].as_str(),
        Some("$")
    );
    assert_eq!(
        serde_json::to_value(&error).unwrap()["value"].as_str(),
        Some("https://[")
    );
}

#[test]
fn scope_fast_path_uses_parsed_css_rather_than_unicode_trimming() {
    for selector in ["\u{a0}:scope", "\u{2009} :scope"] {
        let contract = serde_json::json!({"select":selector,"match":"optional"}).to_string();
        assert_eq!(
            extract("<p>x</p>", &contract, &TestOptions::default()).unwrap(),
            Value::Null
        );
    }
    for selector in [":scope", " /*comment*/ :scope ", ":scope /*comment*/"] {
        let contract = serde_json::json!({"select":selector,"read":"outer-html"}).to_string();
        assert_eq!(
            extract("<p>x</p>", &contract, &TestOptions::default()).unwrap(),
            Value::String("<html><head></head><body><p>x</p></body></html>".into())
        );
    }
}
