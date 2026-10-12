// SPDX-License-Identifier: MPL-2.0
use super::*;

fn value(html: &str, selector: &str, reading: Reading) -> String {
    let mut plan = ExtractionPlan::css(selector).unwrap();
    plan.read = Some(reading);
    prepared(html)
        .execute(&CompiledPlan::compile(&plan).unwrap())
        .unwrap()
        .data()
        .as_values()
        .unwrap()[0]
        .clone()
}

#[test]
fn t28_url_resolution_preserves_labels_queries_and_declared_base_only() {
    let html = "<base href='https://ignored.test/'><p><a href='relative?q=1'>Label</a></p>";
    let source = PreparedDocument::new(
        SourceSnapshot::new(
            html,
            SnapshotMetadata {
                base_url: Some("https://example.test/path/".into()),
            },
        )
        .unwrap(),
        Default::default(),
    )
    .unwrap();
    let mut plan = ExtractionPlan::css("a").unwrap();
    plan.read = Some(Reading::Url("href".into()));
    assert_eq!(
        source
            .execute(&CompiledPlan::compile(&plan).unwrap())
            .unwrap()
            .data()
            .as_values()
            .unwrap(),
        ["https://example.test/path/relative?q=1"]
    );
    plan.limits.max_value_bytes = 16;
    assert_eq!(
        source
            .execute(&CompiledPlan::compile(&plan).unwrap())
            .unwrap_err()
            .code,
        ErrorCode::ResourceLimit
    );
    plan.limits.max_value_bytes = 8;
    assert_eq!(
        source
            .execute(&CompiledPlan::compile(&plan).unwrap())
            .unwrap_err()
            .code,
        ErrorCode::ResourceLimit
    );
    plan.limits.max_value_bytes = 1024;
    assert_eq!(
        prepared(html)
            .execute(&CompiledPlan::compile(&plan).unwrap())
            .unwrap_err()
            .code,
        ErrorCode::InvalidBaseUrl
    );
    assert_eq!(
        prepared("<a href='https://example.test/a?q=1'>Label</a>")
            .execute(&CompiledPlan::compile(&plan).unwrap())
            .unwrap()
            .data()
            .as_values()
            .unwrap(),
        ["https://example.test/a?q=1"]
    );
    assert_eq!(
        prepared("<a href='http://['>Label</a>")
            .execute(&CompiledPlan::compile(&plan).unwrap())
            .unwrap_err()
            .code,
        ErrorCode::InvalidBaseUrl
    );
    plan.read = Some(Reading::Attribute("href".into()));
    plan.limits.max_value_bytes = 1024;
    assert_eq!(
        source
            .execute(&CompiledPlan::compile(&plan).unwrap())
            .unwrap()
            .data()
            .as_values()
            .unwrap(),
        ["relative?q=1"]
    );
}

#[test]
fn t28_resolved_unicode_urls_may_expand_beyond_input_length_without_early_truncation() {
    let raw = format!("https://example.test/?q={}", "é".repeat(2000));
    let source = prepared(&format!("<a href='{raw}'>label</a>"));
    let mut plan = ExtractionPlan::css("a").unwrap();
    plan.read = Some(Reading::Url("href".into()));
    assert_eq!(
        source
            .execute(&CompiledPlan::compile(&plan).unwrap())
            .unwrap()
            .data()
            .as_values()
            .unwrap(),
        [format!("https://example.test/?q={}", "%C3%A9".repeat(2000))]
    );
}

#[test]
fn structural_literal_and_outer_html_preserve_their_distinct_payload_contracts() {
    let html = "<article><p hidden>A &amp; B<span> C</span></p><pre> x\n  y\n</pre><script>inert</script><template>template</template><p class=omit>excluded</p></article>";
    let document = prepared(html);
    let mut plan = ExtractionPlan::css("article").unwrap();
    plan.exclude = Some(vec![".omit".into()]);
    for (reading, expected) in [
        (Reading::Text, "A & B C x\n  y\n"),
        (Reading::Literal, "A & B C x\n  y\ninerttemplate"),
        (
            Reading::OuterHtml,
            "<article><p hidden=\"\">A &amp; B<span> C</span></p><pre> x\n  y\n</pre><script>inert</script><template>template</template></article>",
        ),
    ] {
        plan.read = Some(reading);
        let compiled = CompiledPlan::compile(&plan).unwrap();
        for _ in 0..2 {
            assert_eq!(
                document
                    .execute(&compiled)
                    .unwrap()
                    .data()
                    .as_values()
                    .unwrap(),
                [expected]
            );
        }
    }
    assert_eq!(document.snapshot().html(), html);
    assert_eq!(document.parse_count(), 1);
}

#[test]
fn complete_values_refuse_each_output_bound_without_returning_a_prefix() {
    for reading in [Reading::Text, Reading::Literal, Reading::OuterHtml] {
        let mut plan = ExtractionPlan::css("p").unwrap();
        plan.read = Some(reading);
        plan.limits.max_value_bytes = 3;
        let error = prepared("<p>éé</p>")
            .execute(&CompiledPlan::compile(&plan).unwrap())
            .unwrap_err();
        assert_eq!(error.code, ErrorCode::ResourceLimit);
        assert_eq!(error.resource_counter.as_deref(), Some("max_value_bytes"));
    }
}

#[test]
fn selected_pre_descendants_inherit_preformatted_whitespace() {
    assert_eq!(
        value(
            "<pre>outside<code id=x> A\n  B </code>outside</pre>",
            "#x",
            Reading::Text
        ),
        " A\n  B "
    );
    assert_eq!(
        value("<div><code id=x> A\n  B </code></div>", "#x", Reading::Text),
        "A B"
    );
}
