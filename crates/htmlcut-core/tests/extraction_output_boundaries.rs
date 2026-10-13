// SPDX-License-Identifier: MPL-2.0
//! Public result and inspection boundaries, with independent literal output oracles.
use htmlcut_core::{Contract, Document, ErrorCode, Limits, OutlineOptions, Source};

fn extraction(html: &str, contract: &str, expected: &str) {
    let contract = Contract::from_json(contract).unwrap();
    for maximum in [expected.len(), expected.len() + 1] {
        let mut limits = Limits::default();
        limits.max_output_bytes = maximum;
        let document = Document::parse(Source::new(html.as_bytes()), &limits).unwrap();
        let result = document.extract(&contract).unwrap();
        assert_eq!(result.to_json(expected.len()).unwrap(), expected.as_bytes());
        for shorter in [expected.len() - 1, expected.len() - 2] {
            assert_eq!(
                result.to_json(shorter).unwrap_err().limit.as_deref(),
                Some("output_bytes")
            );
        }
    }
    for maximum in [expected.len() - 1, expected.len() - 2] {
        let mut limits = Limits::default();
        limits.max_output_bytes = maximum;
        let document = Document::parse(Source::new(html.as_bytes()), &limits).unwrap();
        let error = document.extract(&contract).unwrap_err();
        assert_eq!(error.error, ErrorCode::LimitExceeded);
        assert_eq!(error.limit.as_deref(), Some("output_bytes"));
        assert_eq!(error.bound, Some(maximum.into()));
    }
}

#[test]
fn complete_json_counts_one_and_three_list_members_and_record_fields() {
    extraction("<p>é</p>", r#"{"select":"p","match":"all"}"#, r#"["é"]"#);
    extraction(
        "<p>a</p><p>b</p><p>c</p>",
        r#"{"select":"p","match":"all"}"#,
        r#"["a","b","c"]"#,
    );
    extraction(
        "<p>é</p>",
        r#"{"select":"p","fields":{"a":{}}}"#,
        r#"{"a":"é"}"#,
    );
    extraction(
        "<p title='x'>é</p>",
        r#"{"select":"p","fields":{"z":{},"a":{"read":"attr:title"},"n":{"select":"b","match":"optional"}}}"#,
        r#"{"a":"x","n":null,"z":"é"}"#,
    );
    extraction(
        "<p>\"\n</p>",
        r#"{"select":"p","read":"text-content"}"#,
        r#""\"\n""#,
    );
    extraction("", r#"{"select":"p","match":"optional"}"#, "null");
}

fn outline(html: &str, options: &OutlineOptions, expected: &str) {
    for maximum in [expected.len(), expected.len() + 1] {
        let mut limits = Limits::default();
        limits.max_output_bytes = maximum;
        let document = Document::parse(Source::new(html.as_bytes()), &limits).unwrap();
        assert_eq!(document.outline(options).unwrap(), expected);
    }
    for maximum in [expected.len() - 1, expected.len() - 2] {
        let mut limits = Limits::default();
        limits.max_output_bytes = maximum;
        let document = Document::parse(Source::new(html.as_bytes()), &limits).unwrap();
        let error = document.outline(options).unwrap_err();
        assert_eq!(error.limit.as_deref(), Some("output_bytes"));
        assert_eq!(error.bound, Some(maximum.into()));
    }
}

#[test]
fn outline_line_and_utf8_byte_limits_include_the_omission_summary() {
    let html = "<main><p>é</p><aside>x</aside></main>";
    let mut options = OutlineOptions::default();
    options.select = Some("main".into());
    for (maximum, expected) in [
        (
            0,
            "… 4 more lines; narrow --select, lower --depth or raise --max-lines\n",
        ),
        (
            1,
            "1 match for main\n… 3 more lines; narrow --select, lower --depth or raise --max-lines\n",
        ),
        (
            2,
            "1 match for main\nmain\n… 2 more lines; narrow --select, lower --depth or raise --max-lines\n",
        ),
        (
            3,
            "1 match for main\nmain\n  p \"é\"\n… 1 more lines; narrow --select, lower --depth or raise --max-lines\n",
        ),
        (4, "1 match for main\nmain\n  p \"é\"\n  aside \"x\"\n"),
    ] {
        options.max_lines = maximum;
        outline(html, &options, expected);
    }
}

#[test]
fn outline_depth_classes_and_preview_cutoffs_are_exact() {
    let mut options = OutlineOptions::default();
    options.select = Some("main".into());
    options.depth = 0;
    outline(
        "<main><p><b>x</b></p></main>",
        &options,
        "1 match for main\nmain (+2 inside)\n",
    );
    options.depth = 1;
    outline(
        "<main><p><b>x</b></p></main>",
        &options,
        "1 match for main\nmain\n  p (+1 inside)\n",
    );
    options.depth = 6;
    outline(
        "<main class='a b c d'>x</main>",
        &options,
        "1 match for main\nmain.a.b.c.d \"x\"\n",
    );
    outline(
        "<main class='a b c d e'>x</main>",
        &options,
        "1 match for main\nmain.a.b.c.d.+1 \"x\"\n",
    );
    options.attrs = true;
    let html = format!("<main title='{}'>{}</main>", "é".repeat(40), "x".repeat(60));
    let expected = format!(
        "1 match for main\nmain [title=\"{}\"] \"{}\"\n",
        "é".repeat(40),
        "x".repeat(60)
    );
    outline(&html, &options, &expected);
    let html = format!("<main title='{}'>{}</main>", "é".repeat(41), "x".repeat(61));
    let expected = format!(
        "1 match for main\nmain [title=\"{}…\"] \"{}…\"\n",
        "é".repeat(40),
        "x".repeat(60)
    );
    outline(&html, &options, &expected);
}

#[test]
fn cardinality_refusals_identify_the_selector_and_complete_range() {
    let document = Document::parse(Source::new(b"<p>x</p>"), &Limits::default()).unwrap();
    for (contract, path, selector, expected_range, identity) in [
        (
            r#"{"select":"p","match":"all","min":2,"max":3}"#,
            "$",
            Some("p"),
            "2..3",
            "selector \"p\"",
        ),
        (
            r#"{"select":"p","nth":2}"#,
            "$",
            Some("p"),
            "2..",
            "selector \"p\"",
        ),
        (
            r#"{"expect":[{"min":2,"max":3}]}"#,
            "$.expect[0]",
            None,
            "2..3",
            "the match itself",
        ),
        (
            r#"{"expect":[{"select":"b"}]}"#,
            "$.expect[0]",
            Some("b"),
            "1",
            "selector \"b\"",
        ),
    ] {
        let error = document
            .extract(&Contract::from_json(contract).unwrap())
            .unwrap_err();
        assert!(error.message.contains(identity), "{}", error.message);
        assert_eq!(error.path.as_deref(), Some(path));
        assert_eq!(error.select.as_deref(), selector);
        assert_eq!(error.expected.as_deref(), Some(expected_range));
        assert_eq!(error.found, Some(usize::from(selector != Some("b"))));
    }
}
