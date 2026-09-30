use super::*;

fn value(html: &str, selector: &str, projection: Projection) -> String {
    let mut plan = ExtractionPlan::css(selector).unwrap();
    plan.projection = projection;
    prepared(html)
        .execute(&CompiledPlan::compile(&plan).unwrap())
        .unwrap()
        .values
        .remove(0)
}

#[test]
fn t08_t09_six_full_value_fidelity_oracles() {
    let fixtures = [
        (
            "<p>Before <a href=\"relative.html\">IMPORTANT</a> after.</p>",
            "p",
            "Before IMPORTANT after.",
            "Before [IMPORTANT](relative.html) after.",
        ),
        (
            "<p>Before <a class=\"reference internal\" href=\"relative.html\">IMPORTANT</a> after.</p>",
            "p",
            "Before IMPORTANT after.",
            "Before [IMPORTANT](relative.html) after.",
        ),
        (
            "<article><p>BEGINNING</p><section id=\"section-one\"><h2>Policy terms</h2><p>IMPORTANT POLICY CONTENT</p></section><p>ENDING</p></article>",
            "article",
            "BEGINNINGPolicy termsIMPORTANT POLICY CONTENTENDING",
            "BEGINNING\n## Policy terms\nIMPORTANT POLICY CONTENT\nENDING",
        ),
        (
            "<article><p>BEGINNING</p><section id=\"policy\"><h2>Policy terms</h2><p>IMPORTANT POLICY CONTENT</p></section><p>ENDING</p></article>",
            "article",
            "BEGINNINGPolicy termsIMPORTANT POLICY CONTENTENDING",
            "BEGINNING\n## Policy terms\nIMPORTANT POLICY CONTENT\nENDING",
        ),
        (
            "<p>Damage: <img src=\"mirror.jpg\" alt=\"Broken mirror\"> end.</p>",
            "p",
            "Damage:  end.",
            "Damage: Broken mirror end.",
        ),
        (
            "<article><table><caption>Charges</caption><tr><td>EUR 180</td></tr></table><img src=\"mirror.jpg\" alt=\"Broken mirror\"></article>",
            "article",
            "ChargesEUR 180",
            "[table]\nCharges\nEUR 180\n[/table]\nBroken mirror",
        ),
    ];
    for (html, selector, literal, document) in fixtures {
        assert_eq!(
            value(html, selector, Projection::DomText),
            literal,
            "{html}"
        );
        assert_eq!(
            value(html, selector, Projection::DocumentText),
            document,
            "{html}"
        );
    }
}

#[test]
fn t10_t12_payload_templates_hidden_entities_and_exclusions() {
    assert_eq!(
        value(
            "<div hidden aria-hidden='true' style='display:none'>A&amp;B&nbsp;C<template>T<span>M</span></template><script>S</script><style>X</style></div>",
            "div",
            Projection::DomText
        ),
        "A&B\u{a0}CTMSX"
    );
    assert_eq!(
        value(
            "<div hidden aria-hidden='true'>A<template>T</template><script>S</script><style>X</style></div>",
            "div",
            Projection::DocumentText
        ),
        "A"
    );
    for element in ["script", "style", "template"] {
        let html = format!("<{element}>payload</{element}>");
        assert_eq!(value(&html, element, Projection::DomText), "payload");
        assert_eq!(value(&html, element, Projection::DocumentText), "");
    }
    let source = prepared(
        "<aside class='omit'>outside</aside><p id='root'><span class='omit'>A</span>B</p>",
    );
    let mut plan = ExtractionPlan::css("#root").unwrap();
    plan.exclude = vec![".omit".into()];
    assert_eq!(
        source
            .execute(&CompiledPlan::compile(&plan).unwrap())
            .unwrap()
            .values,
        ["B"]
    );
    plan.exclude = vec![":scope".into()];
    assert_eq!(
        source
            .execute(&CompiledPlan::compile(&plan).unwrap())
            .unwrap()
            .values,
        [""]
    );
    assert_eq!(
        source
            .execute(&CompiledPlan::compile(&ExtractionPlan::css("#root").unwrap()).unwrap())
            .unwrap()
            .values,
        ["AB"]
    );
}

#[test]
fn t12_preformatted_lists_spans_and_nested_tables() {
    assert_eq!(
        value("<pre>a\n```\n b</pre>", "pre", Projection::DocumentText),
        "````\na\n```\n b\n````"
    );
    assert_eq!(
        value(
            "<ol start='3'><li>A</li><li value='7'>B<ul><li>C</li></ul></li></ol>",
            "ol",
            Projection::DocumentText
        ),
        "3. A\n7. B\n  - C"
    );
    assert_eq!(
        value(
            "<ol reversed><li>A</li><li>B</li></ol>",
            "ol",
            Projection::DocumentText
        ),
        "2. A\n1. B"
    );
    assert_eq!(
        value(
            "<table><tr><th colspan='2'>Name</th><td rowspan='3'>V<table><tr><td>N</td></tr></table></td></tr></table>",
            "body > table",
            Projection::DocumentText
        ),
        "[table]\n[header] [colspan=2] Name | [rowspan=3] V\n[table]\nN\n[/table]\n[/table]"
    );
}

#[test]
fn t10_t28_transforms_preserve_preformatted_origin_and_url_labels() {
    let source = PreparedDocument::new(
        SourceSnapshot::new(
            "<div>A \n<span>\t B</span><pre> C  D </pre><a href='next?q=1'> L  M </a></div>",
            SnapshotMetadata {
                base_url: Some("https://example.test/path/".into()),
            },
        )
        .unwrap(),
        PreparationLimits::default(),
    )
    .unwrap();
    let mut plan = ExtractionPlan::css("div").unwrap();
    plan.transforms = vec![Transform::NormalizeWhitespace];
    assert_eq!(
        source
            .execute(&CompiledPlan::compile(&plan).unwrap())
            .unwrap()
            .values,
        ["A B C  D  L M "]
    );
    plan.projection = Projection::DocumentText;
    plan.transforms = vec![Transform::NormalizeWhitespace, Transform::ResolveUrls];
    let result = source
        .execute(&CompiledPlan::compile(&plan).unwrap())
        .unwrap();
    assert_eq!(
        result.values,
        ["A B\n```\n C  D \n```\n[ L M ](https://example.test/path/next?q=1)"]
    );
    plan.transforms.reverse();
    assert_eq!(
        source
            .execute(&CompiledPlan::compile(&plan).unwrap())
            .unwrap()
            .values,
        result.values
    );
    assert_eq!(
        value("<p>A\u{a0}B\u{200b}C</p>", "p", Projection::DomText),
        "A\u{a0}B\u{200b}C"
    );
}

#[test]
fn t15_dom_serialization_is_filtered_without_source_or_dom_mutation() {
    let source = prepared("<DIV id='x'>A<span class='omit'>B</span>C</DIV>");
    let mut plan = ExtractionPlan::css("#x").unwrap();
    plan.projection = Projection::OuterHtml;
    plan.exclude = vec![".omit".into()];
    assert_eq!(
        source
            .execute(&CompiledPlan::compile(&plan).unwrap())
            .unwrap()
            .values,
        ["<div id=\"x\">AC</div>"]
    );
    plan.exclude.clear();
    plan.projection = Projection::InnerHtml;
    assert_eq!(
        source
            .execute(&CompiledPlan::compile(&plan).unwrap())
            .unwrap()
            .values,
        ["A<span class=\"omit\">B</span>C"]
    );
    assert_eq!(
        source.snapshot().html(),
        "<DIV id='x'>A<span class='omit'>B</span>C</DIV>"
    );
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
    plan.projection = Projection::Attribute {
        name: "href".into(),
    };
    plan.transforms = vec![Transform::ResolveUrls];
    assert_eq!(
        source
            .execute(&CompiledPlan::compile(&plan).unwrap())
            .unwrap()
            .values,
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
            .values,
        ["https://example.test/a?q=1"]
    );
    assert_eq!(
        prepared("<a href='http://['>Label</a>")
            .execute(&CompiledPlan::compile(&plan).unwrap())
            .unwrap_err()
            .code,
        ErrorCode::InvalidBaseUrl
    );
    plan.transforms.clear();
    plan.limits.max_value_bytes = 1024;
    assert_eq!(
        source
            .execute(&CompiledPlan::compile(&plan).unwrap())
            .unwrap()
            .values,
        ["relative?q=1"]
    );
}

#[test]
fn t12_structural_payload_escaping_blocks_and_preformatted_edges() {
    let html = "<article><h1>Title</h1><div>A<br>B</div><ul><li>one</li><li>two</li></ul><p><a href='x?a=(b)'>[label]</a><img alt='[alt]'></p><script>omit</script><pre>`````\n  code</pre></article>";
    assert_eq!(
        value(html, "article", Projection::DocumentText),
        "# Title\nA\nB\n- one\n- two\n[\\[label\\]](x?a=\\(b\\))\\[alt\\]\n``````\n`````\n  code\n``````"
    );
    assert_eq!(
        value("<p><a>unlinked</a><img></p>", "p", Projection::DocumentText),
        "unlinked"
    );
    let mut plan = ExtractionPlan::css("article").unwrap();
    plan.projection = Projection::DocumentText;
    plan.exclude = vec!["div, script".into()];
    let source =
        prepared("<article><div>removed<span>also removed</span></div><p>kept</p></article>");
    assert_eq!(
        source
            .execute(&CompiledPlan::compile(&plan).unwrap())
            .unwrap()
            .values,
        ["kept"]
    );
    let mut plan = ExtractionPlan::css("pre").unwrap();
    plan.projection = Projection::DocumentText;
    plan.limits.max_value_bytes = 3;
    assert_eq!(
        prepared("<pre>````</pre>")
            .execute(&CompiledPlan::compile(&plan).unwrap())
            .unwrap_err()
            .code,
        ErrorCode::ResourceLimit
    );
    let mut plan = ExtractionPlan::css("p").unwrap();
    plan.projection = Projection::OuterHtml;
    plan.limits.max_value_bytes = 3;
    assert_eq!(
        prepared("<p>content</p>")
            .execute(&CompiledPlan::compile(&plan).unwrap())
            .unwrap_err()
            .code,
        ErrorCode::ResourceLimit
    );
    let mut plan = ExtractionPlan::css("p").unwrap();
    plan.projection = Projection::DomText;
    plan.limits.max_value_bytes = 1;
    assert_eq!(
        prepared("<p>long</p>")
            .execute(&CompiledPlan::compile(&plan).unwrap())
            .unwrap_err()
            .code,
        ErrorCode::ResourceLimit
    );
}

#[test]
fn t12_list_ordinals_honor_html_integer_prefixes_and_defer_unused_overflow() {
    assert_eq!(
        value(
            "<ol start=' +3 trailing'><li>A</li><li value=' 7x'>B</li></ol>",
            "ol",
            Projection::DocumentText
        ),
        "3. A\n7. B"
    );
    assert_eq!(
        value(
            "<ol start='invalid'><li value='invalid'>A</li></ol>",
            "ol",
            Projection::DocumentText
        ),
        "1. A"
    );
    assert_eq!(
        value(
            "<ol start='9223372036854775807'><li>A</li></ol>",
            "ol",
            Projection::DocumentText
        ),
        "9223372036854775807. A"
    );
    let mut plan = ExtractionPlan::css("ol").unwrap();
    plan.projection = Projection::DocumentText;
    assert_eq!(
        prepared("<ol start='9223372036854775807'><li>A</li><li>B</li></ol>")
            .execute(&CompiledPlan::compile(&plan).unwrap())
            .unwrap_err()
            .code,
        ErrorCode::ResourceLimit
    );
    assert_eq!(
        prepared("<ol start='9999999999999999999999999'><li>A</li></ol>")
            .execute(&CompiledPlan::compile(&plan).unwrap())
            .unwrap_err()
            .code,
        ErrorCode::ResourceLimit
    );
    plan.exclude = vec![".omit".into()];
    assert_eq!(
        prepared("<ol reversed><li class='omit'>A</li><li>B</li><li>C</li></ol>")
            .execute(&CompiledPlan::compile(&plan).unwrap())
            .unwrap()
            .values,
        ["2. B\n1. C"]
    );
}

#[test]
fn t12_preformatted_fences_cover_alternative_text_and_nested_formatting() {
    assert_eq!(
        value(
            "<pre><img alt='````'></pre>",
            "pre",
            Projection::DocumentText
        ),
        "`````\n````\n`````"
    );
    assert_eq!(
        value(
            "<pre>A<pre>B</pre>C</pre>",
            "body > pre",
            Projection::DocumentText
        ),
        "````\nA\n```\nB\n```\nC\n````"
    );
    assert_eq!(
        value(
            "<pre>A<span class='omit'>``````</span>B</pre>",
            "pre",
            Projection::DocumentText
        ),
        "```````\nA``````B\n```````"
    );
}

#[test]
fn t12_selected_structural_fragments_do_not_require_ancestors_in_the_projection() {
    let html = "<table><tr><th>H</th><td>V</td></tr></table><ul><li>item</li></ul><div><p>A</p><img><img alt=''><img alt='B'><p>C</p><a href=''>D</a></div>";
    for (selector, expected) in [
        ("tr", "[header] HV"),
        ("th", "[header] H"),
        ("td", "V"),
        ("li", "- item"),
        ("div", "A\nB\nC\n[D]()"),
    ] {
        assert_eq!(value(html, selector, Projection::DocumentText), expected);
    }
}

#[test]
fn t28_invalid_destination_with_a_valid_base_reports_resolution_failure() {
    let source = PreparedDocument::new(
        SourceSnapshot::new(
            "<a href='http://['>label</a>",
            SnapshotMetadata {
                base_url: Some("https://example.test/path/".into()),
            },
        )
        .unwrap(),
        Default::default(),
    )
    .unwrap();
    let mut plan = ExtractionPlan::css("a").unwrap();
    plan.projection = Projection::Attribute {
        name: "href".into(),
    };
    plan.transforms = vec![Transform::ResolveUrls];
    assert_eq!(
        source
            .execute(&CompiledPlan::compile(&plan).unwrap())
            .unwrap_err()
            .code,
        ErrorCode::InvalidBaseUrl
    );
    assert_eq!(
        value(
            "<pre><img><a>label</a></pre>",
            "pre",
            Projection::DocumentText
        ),
        "```\nlabel\n```"
    );
}

#[test]
fn t28_resolved_unicode_urls_may_expand_beyond_input_length_without_early_truncation() {
    let raw = format!("https://example.test/?q={}", "é".repeat(2000));
    let source = prepared(&format!("<a href='{raw}'>label</a>"));
    let mut plan = ExtractionPlan::css("a").unwrap();
    plan.projection = Projection::Attribute {
        name: "href".into(),
    };
    plan.transforms = vec![Transform::ResolveUrls];
    assert_eq!(
        source
            .execute(&CompiledPlan::compile(&plan).unwrap())
            .unwrap()
            .values,
        [format!("https://example.test/?q={}", "%C3%A9".repeat(2000))]
    );
}
