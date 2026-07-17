use super::*;

#[test]
fn inspect_source_keeps_heading_samples_inside_primary_scope() {
    let source = memory_source(
        "fixture.html",
        "<html><body><aside><h2>Recommended</h2><h3>More like this</h3></aside><main id=\"content\"><article><h1>Main Title</h1><h2>Section Title</h2><p><a href=\"/guide\">Guide</a></p></article></main></body></html>",
    );

    let inspection = inspect_source(
        &source,
        &RuntimeOptions::default(),
        &InspectionOptions {
            include_source_text: false,
            sample_limit: 6,
        },
    );

    assert!(inspection.ok);
    let document = inspection.document.expect("document inspection");
    assert_eq!(document.reading_candidates[0].selector, "#content");
    assert_eq!(
        document
            .headings
            .iter()
            .map(|heading| heading.text.as_str())
            .collect::<Vec<_>>(),
        vec!["Main Title", "Section Title"]
    );
    assert_eq!(document.links.len(), 1);
    assert_eq!(document.links[0].text, "Guide");
}

#[test]
fn inspect_source_prefers_inner_candidate_over_outer_chrome_wrapper() {
    let source = memory_source_with_base(
        "fixture.html",
        "<html><body><main id=\"content\"><nav class=\"page-tools\"><a href=\"/edit\">Edit</a></nav><div id=\"mw-content-text\"><article class=\"article-body\"><h1>Article Title</h1><p>Alpha beta gamma delta epsilon zeta eta theta iota kappa lambda mu.</p><a href=\"/guide\">Guide</a></article></div><aside class=\"related-topics\"><h2>Related Topics</h2><a href=\"/other\">Other</a></aside></main></body></html>",
        "https://example.test/start.html",
    );

    let inspection = inspect_source(
        &source,
        &RuntimeOptions::default(),
        &InspectionOptions {
            include_source_text: false,
            sample_limit: 4,
        },
    );

    assert!(inspection.ok);
    let document = inspection.document.expect("document inspection");
    assert_eq!(
        document.extraction_candidates[0].selector,
        "article.article-body"
    );
    assert_eq!(document.headings[0].text, "Article Title");
    assert!(
        document
            .headings
            .iter()
            .all(|heading| heading.text != "Related Topics")
    );
    assert_eq!(document.links[0].href.as_deref(), Some("/guide"));
    assert!(
        document
            .links
            .iter()
            .all(|link| link.href.as_deref() != Some("/edit"))
    );
    assert!(
        document
            .links
            .iter()
            .all(|link| link.href.as_deref() != Some("/other"))
    );
}

#[test]
fn inspect_source_keeps_markdown_body_candidates_inside_mixed_layout_shells() {
    let repo_heading_chrome = (1..=48)
        .map(|index| format!("<h3>Repository Section {index}</h3>"))
        .collect::<String>();
    let repo_link_chrome = (1..=240)
        .map(|index| format!("<a href=\"/repo-link-{index}\">Repository Link {index}</a>"))
        .collect::<String>();
    let html = format!(
        "<html><body><main id=\"js-repo-pjax-container\"><div id=\"wiki-wrapper\" class=\"page\"><nav class=\"gh-header repo-nav\"><h1>Jackson Release 3.1</h1><a href=\"#wiki-pages-box\">Jump to bottom</a><span>Tatu Saloranta edited this page</span><a href=\"/_history\">152 revisions</a>{repo_heading_chrome}{repo_link_chrome}</nav><div id=\"wiki-content\"><div class=\"Layout Layout--sidebarPosition-end\"><div class=\"Layout-main\"><div id=\"wiki-body\" class=\"gollum-markdown-content\"><div class=\"markdown-body\"><p><a href=\"Jackson-Releases\">Jackson Version</a> 3.1 is a Major New version.</p><p>This wiki page gives a list of links to all changes.</p><div class=\"markdown-heading\"><h2>Status</h2><a class=\"anchor\" href=\"#status\">#</a></div><p>Branch is open for patch releases.</p><div class=\"markdown-heading\"><h3>Patches</h3><a class=\"anchor\" href=\"#patches\">#</a></div><ul><li><a href=\"Jackson-Release-3.1.1\">3.1.1</a></li><li><a href=\"Jackson-Release-3.1.2\">3.1.2</a></li></ul></div></div><aside class=\"Layout-sidebar related-topics\"><h2>Related Topics</h2><a href=\"/other\">Other</a></aside></div></div></div></main></body></html>"
    );
    let source = memory_source_with_base(
        "fixture.html",
        &html,
        "https://github.com/FasterXML/jackson/wiki/Jackson-Release-3.1",
    );

    let inspection = inspect_source(
        &source,
        &RuntimeOptions::default(),
        &InspectionOptions {
            include_source_text: false,
            sample_limit: 5,
        },
    );

    assert!(inspection.ok);
    let document = inspection.document.expect("document inspection");
    assert!(matches!(
        document.extraction_candidates[0].selector.as_str(),
        "#wiki-content" | "div.Layout-main" | "#wiki-body" | "div.markdown-body"
    ));
    assert!(matches!(
        document.reading_candidates[0].selector.as_str(),
        "#wiki-content" | "div.Layout-main" | "#wiki-body" | "div.markdown-body"
    ));
    assert_eq!(document.headings[0].text, "Status");
    assert!(
        document
            .links
            .iter()
            .any(|link| link.href.as_deref() == Some("Jackson-Releases"))
    );
    assert!(
        document
            .links
            .iter()
            .all(|link| link.href.as_deref() != Some("/other"))
    );
}

#[test]
fn inspect_source_prefers_inner_markdown_body_when_outer_wrapper_only_adds_heading_shell() {
    let repo_heading_chrome = (1..=60)
        .map(|index| format!("<h3>Repository Section {index}</h3>"))
        .collect::<String>();
    let body_sections = (1..=18)
        .map(|index| {
            format!(
                "<h2>Body Section {index}</h2><p>Alpha beta gamma delta epsilon zeta eta theta iota kappa lambda mu nu xi omicron pi rho sigma tau upsilon phi chi psi omega section {index}.</p>"
            )
        })
        .collect::<String>();
    let html = format!(
        "<html><body><main id=\"js-repo-pjax-container\"><div id=\"repo-content-pjax-container\"><div id=\"wiki-wrapper\" class=\"page\"><nav class=\"gh-header repo-nav\"><h1>Jackson Release 3.1</h1><a href=\"#wiki-body\">Jump to wiki</a><a href=\"/_history\">152 revisions</a>{repo_heading_chrome}</nav><div id=\"wiki-content\"><div class=\"Layout Layout--sidebarPosition-end\"><div class=\"Layout-main\"><div id=\"wiki-body\" class=\"gollum-markdown-content\"><div class=\"markdown-body\">{body_sections}</div></div><aside class=\"Layout-sidebar related-topics\"><h2>Related Topics</h2><a href=\"/other\">Other</a></aside></div></div></div></div></main></body></html>"
    );
    let source = memory_source_with_base(
        "fixture.html",
        &html,
        "https://github.com/FasterXML/jackson/wiki/Jackson-Release-3.1",
    );

    let inspection = inspect_source(
        &source,
        &RuntimeOptions::default(),
        &InspectionOptions {
            include_source_text: false,
            sample_limit: 5,
        },
    );

    assert!(inspection.ok);
    let document = inspection.document.expect("document inspection");
    assert!(matches!(
        document.extraction_candidates[0].selector.as_str(),
        "#wiki-content" | "div.Layout-main" | "#wiki-body" | "div.markdown-body"
    ));
    assert!(matches!(
        document.reading_candidates[0].selector.as_str(),
        "#wiki-content" | "div.Layout-main" | "#wiki-body" | "div.markdown-body"
    ));
    assert_ne!(
        document.extraction_candidates[0].selector,
        "#js-repo-pjax-container"
    );
    assert_ne!(
        document.extraction_candidates[0].selector,
        "#repo-content-pjax-container"
    );
    assert_ne!(document.extraction_candidates[0].selector, "#wiki-wrapper");
}

#[test]
fn inspect_source_ignores_document_root_feature_shells() {
    let source = memory_source(
        "fixture.html",
        "<html class=\"vector-feature-language-in-main-menu-disabled vector-feature-toc-pinned-clientpref-1\"><body><div class=\"mw-page-container\"><main id=\"content\" class=\"mw-body\"><header class=\"mw-body-header\"><nav class=\"vector-toc-landmark\"><a href=\"#bodyContent\">Jump to content</a></nav><h1 id=\"firstHeading\">Mathematics</h1></header><div id=\"bodyContent\" class=\"vector-body\"><div id=\"mw-content-text\" class=\"mw-body-content\"><div class=\"mw-content-ltr mw-parser-output\"><div role=\"note\" class=\"hatnote navigation-not-searchable\">For other uses, see <a href=\"/wiki/Mathematics_(disambiguation)\">Mathematics (disambiguation)</a>.</div><p>Mathematics includes the study of quantity, structure, space, and change.</p><h2>Areas of mathematics</h2><p>Number theory and geometry are classical branches.</p></div></div></div></main></div></body></html>",
    );

    let inspection = inspect_source(
        &source,
        &RuntimeOptions::default(),
        &InspectionOptions {
            include_source_text: false,
            sample_limit: 4,
        },
    );

    assert!(inspection.ok);
    let document = inspection.document.expect("document inspection");
    assert!(!document.extraction_candidates.is_empty());
    assert!(!document.reading_candidates.is_empty());
    assert!(matches!(
        document.extraction_candidates[0].selector.as_str(),
        "#content" | "#mw-content-text" | "div.mw-content-ltr.mw-parser-output"
    ));
    assert!(matches!(
        document.reading_candidates[0].selector.as_str(),
        "#content" | "#mw-content-text" | "div.mw-content-ltr.mw-parser-output"
    ));
    assert_eq!(document.headings[0].text, "Mathematics");
    assert_eq!(document.headings[1].text, "Areas of mathematics");
}

#[test]
fn inspect_source_rejects_outer_wrapper_when_extra_headings_are_recommendation_chrome() {
    let source = memory_source_with_base(
        "fixture.html",
        "<html><body><div class=\"page\"><div class=\"pre-content recommended-navbar\"><h3>Recommended Videos</h3><h3>Recommended Articles</h3><section><a href=\"/promo-1\"><h4>Promo one</h4></a><a href=\"/promo-2\"><h4>Promo two</h4></a><a href=\"/promo-3\"><h4>Promo three</h4></a></section></div><div class=\"page-content\"><main class=\"main-content\"><article class=\"article-wrap\"><header><h1>Article Title</h1><h2>Article Subtitle</h2></header><div class=\"article-body\"><p>Alpha beta gamma delta epsilon zeta eta theta iota kappa lambda mu nu xi omicron pi rho sigma tau upsilon phi chi psi omega.</p><p><a href=\"/guide\">Guide</a></p></div></article></main></div></div></body></html>",
        "https://example.test/start.html",
    );

    let inspection = inspect_source(
        &source,
        &RuntimeOptions::default(),
        &InspectionOptions {
            include_source_text: false,
            sample_limit: 4,
        },
    );

    assert!(inspection.ok);
    let document = inspection.document.expect("document inspection");
    assert_ne!(document.extraction_candidates[0].selector, "div.page");
    assert!(matches!(
        document.extraction_candidates[0].selector.as_str(),
        "main.main-content" | "article.article-wrap"
    ));
    assert_eq!(
        document
            .headings
            .iter()
            .map(|heading| heading.text.as_str())
            .collect::<Vec<_>>(),
        vec!["Article Title", "Article Subtitle"]
    );
}

#[test]
fn inspect_source_prefers_precise_descendant_when_outer_wrapper_only_adds_chrome_links() {
    let source = memory_source_with_base(
        "fixture.html",
        "<html><body><main id=\"content\"><div id=\"bodyContent\"><div class=\"header-tools\"><a href=\"#jump\">Jump to content</a><a href=\"/edit\">Edit</a><a href=\"/history\">History</a><a href=\"/talk\">Talk</a></div><div id=\"mw-content-text\" class=\"mw-body-content\"><div class=\"mw-content-ltr mw-parser-output\"><p>Mathematics includes the study of quantity, structure, space, and change.</p><h2>Areas of mathematics</h2><p>Number theory and geometry are classical branches.</p></div></div></div></main></body></html>",
        "https://example.test/wiki/Mathematics",
    );

    let inspection = inspect_source(
        &source,
        &RuntimeOptions::default(),
        &InspectionOptions {
            include_source_text: false,
            sample_limit: 4,
        },
    );

    assert!(inspection.ok);
    let document = inspection.document.expect("document inspection");
    assert!(matches!(
        document.extraction_candidates[0].selector.as_str(),
        "#mw-content-text" | "div.mw-content-ltr.mw-parser-output"
    ));
    assert!(matches!(
        document.reading_candidates[0].selector.as_str(),
        "#mw-content-text" | "div.mw-content-ltr.mw-parser-output"
    ));
}
