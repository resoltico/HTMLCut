use super::*;

#[test]
fn parse_document_and_preview_cover_public_entrypoints() {
    let request = selector_request("<article>Hello</article>");
    let parsed = parse_document(&request.source, &RuntimeOptions::default());
    assert!(parsed.ok);
    assert_eq!(parsed.operation_id, OperationId::DocumentParse);
    assert!(parsed.document.is_some());

    let inspection = inspect_source(
        &request.source,
        &RuntimeOptions::default(),
        &InspectionOptions::default(),
    );
    assert!(inspection.ok);
    assert_eq!(inspection.operation_id, OperationId::SourceInspect);
    assert!(inspection.document.is_some());

    let preview = preview_extraction(&request, &RuntimeOptions::default());
    assert!(preview.ok);
    assert_eq!(preview.operation_id, OperationId::SelectPreview);

    let missing = file_source("/definitely/missing.html");
    let parsed_error = parse_document(&missing, &RuntimeOptions::default());
    assert!(!parsed_error.ok);
    assert_eq!(parsed_error.operation_id, OperationId::DocumentParse);
    assert_eq!(parsed_error.diagnostics[0].code, "SOURCE_LOAD_FAILED");

    let inspection_error = inspect_source(
        &missing,
        &RuntimeOptions::default(),
        &InspectionOptions::default(),
    );
    assert!(!inspection_error.ok);
    assert_eq!(inspection_error.operation_id, OperationId::SourceInspect);
    assert_eq!(inspection_error.diagnostics[0].code, "SOURCE_LOAD_FAILED");

    let mut invalid = selector_request("<article>Hello</article>");
    invalid.spec_version = 0;
    let invalid_result = extract(&invalid, &RuntimeOptions::default());
    assert!(!invalid_result.ok);
    assert_eq!(invalid_result.operation_id, OperationId::SelectExtract);
    assert_eq!(invalid_result.stats.match_count, 0);
    assert_eq!(invalid_result.source.bytes_read, 0);
    assert_eq!(
        invalid_result.diagnostics[0].code,
        "UNSUPPORTED_SPEC_VERSION"
    );
}

#[test]
fn unresolved_effective_base_is_reported_for_inspection_and_rewrite_requests() {
    let source = memory_source(
        "inline",
        "<html><head><base href=\"../content/\"></head><body><a href=\"guide.html\">Guide</a></body></html>",
    );

    let inspection = inspect_source(
        &source,
        &RuntimeOptions::default(),
        &InspectionOptions::default(),
    );
    assert!(inspection.ok);
    assert_eq!(
        inspection.diagnostics[0].code,
        "EFFECTIVE_BASE_URL_UNRESOLVED"
    );
    assert!(inspection.source.effective_base_url.is_none());

    let mut selector_request = ExtractionRequest::new(
        source.clone(),
        ExtractionSpec::selector(selector_query("a")).with_value(attribute_value("href")),
    );
    selector_request.output.rendering.rewrite_urls = true;
    let selector_result = extract(&selector_request, &RuntimeOptions::default());
    assert!(selector_result.ok);
    assert_eq!(
        selector_result.matches[0].value,
        Value::String("guide.html".to_owned())
    );
    assert_eq!(
        selector_result.diagnostics[0].code,
        "EFFECTIVE_BASE_URL_UNRESOLVED"
    );

    let mut slice_request = ExtractionRequest::new(
        source,
        ExtractionSpec::slice(
            slice_spec("<a ", "</a>").with_boundary_retention(BoundaryRetention::IncludeBoth),
        )
        .with_value(attribute_value("href")),
    );
    slice_request.output.rendering.rewrite_urls = true;
    let slice_result = extract(&slice_request, &RuntimeOptions::default());
    assert!(slice_result.ok);
    assert_eq!(
        slice_result.matches[0].value,
        Value::String("guide.html".to_owned())
    );
    assert_eq!(
        slice_result.diagnostics[0].code,
        "EFFECTIVE_BASE_URL_UNRESOLVED"
    );
}

#[test]
fn inspect_source_summarizes_document_structure() {
    let source = memory_source_with_base(
        "fixture.html",
        "<!DOCTYPE html><html><head><title>Fixture</title><base href=\"../content/\"></head><body><main><article class=\"story card\"><h1>Hello</h1><p>World</p><a href=\"../guide.html\">Guide</a><img src=\"hero.png\" alt=\"Hero\"><table><tr><td>A</td></tr></table></article><section class=\"card\"><h2>More</h2><a href=\"/docs\">Docs</a></section></main></body></html>",
        "https://example.test/docs/start.html",
    );
    let inspection = inspect_source(
        &source,
        &RuntimeOptions::default(),
        &InspectionOptions {
            include_source_text: true,
            sample_limit: 4,
        },
    );

    assert!(inspection.ok);
    assert_eq!(
        inspection.source.text.as_deref(),
        Some(
            "<!DOCTYPE html><html><head><title>Fixture</title><base href=\"../content/\"></head><body><main><article class=\"story card\"><h1>Hello</h1><p>World</p><a href=\"../guide.html\">Guide</a><img src=\"hero.png\" alt=\"Hero\"><table><tr><td>A</td></tr></table></article><section class=\"card\"><h2>More</h2><a href=\"/docs\">Docs</a></section></main></body></html>"
        )
    );
    assert_eq!(
        inspection.source.input_base_url.as_deref(),
        Some("https://example.test/docs/start.html")
    );
    assert_eq!(
        inspection.source.effective_base_url.as_deref(),
        Some("https://example.test/content/")
    );
    let document = inspection.document.expect("document inspection");
    assert_eq!(document.title.as_deref(), Some("Fixture"));
    assert_eq!(document.document_base_href.as_deref(), Some("../content/"));
    assert_eq!(document.root_tag, "html");
    assert!(document.element_count >= 10);
    assert_eq!(document.link_count, 2);
    assert_eq!(document.image_count, 1);
    assert_eq!(document.table_count, 1);
    assert_eq!(document.top_tags[0].name, "a");
    assert_eq!(document.top_tags[0].count, 2);
    assert_eq!(document.top_classes[0].name, "card");
    assert_eq!(document.top_classes[0].count, 2);
    assert!(!document.reading_candidates.is_empty());
    assert_eq!(document.reading_candidates[0].tag_name, "main");
    assert!(document.reading_candidates[0].selector.contains("main"));
    assert!(!document.extraction_candidates.is_empty());
    assert_eq!(document.headings[0].level, 1);
    assert_eq!(document.headings[0].text, "Hello");
    assert_eq!(document.links[0].href.as_deref(), Some("../guide.html"));
    assert_eq!(
        document.links[0].resolved_href.as_deref(),
        Some("https://example.test/guide.html")
    );
    assert!(document.text_char_count > 0);
}

#[test]
fn inspect_source_reports_body_text_count_without_content_heuristics() {
    let source = memory_source(
        "fixture.html",
        "<html><body><nav><a href=\"/edit\">Edit</a></nav><main id=\"content\"><h1>Main Title</h1><p>Body paragraph.</p></main></body></html>",
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
        document.text_char_count,
        "Edit Main Title Body paragraph.".chars().count()
    );
}

#[test]
fn inspect_source_honors_zero_sample_limit_without_collecting_previews() {
    let source = memory_source_with_base(
        "fixture.html",
        "<html><body><h1>Hello</h1><a href=\"/guide\">Guide</a><a>No href</a></body></html>",
        "https://example.test/start.html",
    );

    let inspection = inspect_source(
        &source,
        &RuntimeOptions::default(),
        &InspectionOptions {
            include_source_text: false,
            sample_limit: 0,
        },
    );

    assert!(inspection.ok);
    let document = inspection.document.expect("document inspection");
    assert_eq!(document.link_count, 2);
    assert!(document.extraction_candidates.is_empty());
    assert!(document.reading_candidates.is_empty());
    assert!(document.headings.is_empty());
    assert!(document.links.is_empty());
    assert!(document.top_tags.is_empty());
    assert!(document.top_classes.is_empty());
}

#[test]
fn inspect_source_prioritizes_content_scope_and_skips_placeholder_link_samples() {
    let source = memory_source_with_base(
        "fixture.html",
        "<html><body><nav><h2>Navigation</h2><a href=\"#\">Comments</a></nav><main id=\"content\"><article><h1>Article Title</h1><p>Alpha beta gamma delta epsilon zeta eta theta iota kappa lambda mu.</p><a href=\"/guide\">Guide</a><a href=\"javascript:void(0)\">Share</a></article></main></body></html>",
        "https://example.test/start.html",
    );

    let inspection = inspect_source(
        &source,
        &RuntimeOptions::default(),
        &InspectionOptions {
            include_source_text: false,
            sample_limit: 3,
        },
    );

    assert!(inspection.ok);
    let document = inspection.document.expect("document inspection");
    assert_eq!(document.reading_candidates[0].selector, "#content");
    assert_eq!(document.headings[0].text, "Article Title");
    assert_eq!(document.links[0].href.as_deref(), Some("/guide"));
    assert!(
        document
            .links
            .iter()
            .all(|link| link.href.as_deref() != Some("#"))
    );
    assert!(
        document
            .links
            .iter()
            .all(|link| link.href.as_deref() != Some("javascript:void(0)"))
    );
}

#[test]
fn inspect_source_skips_empty_headings_and_preserves_button_backed_titles() {
    let source = memory_source(
        "fixture.html",
        "<html><body><main id=\"content\"><article><h1>Main Title</h1><h2>   </h2><section><h3><button type=\"button\"><div aria-hidden=\"true\">Chevron</div><div>Expandable Section</div></button></h3><p>Body</p></section></article></main></body></html>",
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
    assert_eq!(document.reading_candidates[0].selector, "article");
    assert_eq!(
        document
            .headings
            .iter()
            .map(|heading| heading.text.as_str())
            .collect::<Vec<_>>(),
        vec!["Main Title", "Expandable Section"]
    );
    assert_eq!(document.reading_candidates[0].heading_count, 2);
}

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

#[test]
fn inspect_source_rejects_title_only_header_fragment_when_article_body_exists() {
    let source = memory_source_with_base(
        "fixture.html",
        "<html><body><main class=\"main-content\"><article class=\"article-wrap\"><header><div class=\"article-meta article-meta-upper\"><h1>Article Title</h1><h2>Article Subtitle</h2></div></header><div class=\"article-body\"><p>Alpha beta gamma delta epsilon zeta eta theta iota kappa lambda mu nu xi omicron pi rho sigma tau upsilon phi chi psi omega.</p><p>Body continues with context.</p></div></article></main></body></html>",
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
    assert_ne!(
        document.extraction_candidates[0].selector,
        "div.article-meta.article-meta-upper"
    );
    assert!(matches!(
        document.extraction_candidates[0].selector.as_str(),
        "main.main-content" | "article.article-wrap"
    ));
}

#[test]
fn inspect_source_keeps_body_links_when_the_top_candidate_has_none() {
    let source = memory_source_with_base(
        "fixture.html",
        "<html><body><main class=\"story-shell\"><section class=\"content-header\"><h2>Here's the latest</h2></section><article class=\"story-body\"><p>Alpha beta gamma delta epsilon zeta eta theta iota kappa lambda mu nu xi omicron pi rho sigma tau.</p><p><a href=\"/guide\">Guide</a></p></article></main></body></html>",
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
    assert!(
        document
            .links
            .iter()
            .any(|link| link.href.as_deref() == Some("/guide"))
    );
}

#[test]
fn inspect_source_prefers_heading_bearing_wrapper_over_body_only_fragment() {
    let source = memory_source_with_base(
        "fixture.html",
        "<html><body><main class=\"main-content\"><article class=\"article-wrap\"><h1>Article Title</h1><h2>Article Subtitle</h2><div class=\"article-content-wrap\"><div class=\"article-content\"><h3>Highlights</h3><h4>Key point</h4><div class=\"article-body\"><p>Alpha beta gamma delta epsilon zeta eta theta iota kappa lambda mu.</p><p>Nu xi omicron pi rho sigma tau upsilon phi chi psi omega.</p></div></div></div></article></main></body></html>",
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
    assert_ne!(document.reading_candidates[0].selector, "div.article-body");
    assert_eq!(
        document.reading_candidates[0].selector,
        "article.article-wrap"
    );
    assert_eq!(document.headings[0].text, "Article Title");
}

#[test]
fn inspect_source_prefers_title_bearing_wrapper_over_body_only_fragment() {
    let source = memory_source_with_base(
        "fixture.html",
        "<html><body><main id=\"story\"><header class=\"story-header\"><div class=\"eyebrow\"><a href=\"/category\">Updates</a></div><h1>Primary Title</h1><h2>Secondary Deck</h2><div class=\"author-byline\">By Reporter</div></header><div class=\"story-body\"><div class=\"notice\"><span class=\"label\">NEW</span> playback available</div><p>Alpha beta gamma delta epsilon zeta eta theta iota kappa lambda mu nu xi omicron pi rho sigma tau upsilon phi chi psi omega.</p><p>Body paragraphs continue with context and evidence for the story.</p><p><a href=\"/background\"><strong>BACKGROUND READING FOR THIS TOPIC</strong></a></p></div><footer class=\"related-topics\"><h3>Related Topics</h3><a href=\"/other\">Other</a></footer></main></body></html>",
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
    assert_eq!(document.reading_candidates[0].selector, "#story");
    assert_eq!(document.headings[0].text, "Primary Title");
    assert!(
        document
            .headings
            .iter()
            .all(|heading| heading.text != "Related Topics")
    );
    assert!(
        document
            .links
            .iter()
            .any(|link| link.href.as_deref() == Some("/background"))
    );
    assert!(
        document
            .links
            .iter()
            .all(|link| link.href.as_deref() != Some("/other"))
    );
}

#[test]
fn inspect_source_prefers_stable_structural_selector_over_exact_path_fallback() {
    let source = memory_source_with_base(
        "fixture.html",
        "<html><body><div class=\"layout-live-story\"><section class=\"live-story-wrapper\"><h1>Main Story Title</h1><p>Lead summary for the full live report.</p><div class=\"posts\"><article class=\"story-post\"><div><h2>First update</h2><p>Alpha beta gamma delta epsilon zeta eta theta iota kappa lambda mu nu xi omicron.</p></div></article><article class=\"story-post\"><div><h2>Second update</h2><p>Pi rho sigma tau upsilon phi chi psi omega alpha beta gamma delta epsilon.</p></div></article></div></section></div></body></html>",
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
        document.reading_candidates[0].selector,
        "section.live-story-wrapper"
    );
    assert_ne!(
        document.reading_candidates[0].selector,
        document.reading_candidates[0].path
    );
    assert_eq!(document.headings[0].text, "Main Story Title");
}

#[test]
fn validate_request_reports_unsupported_versions_and_invalid_selectors() {
    let mut request = selector_request("");
    request.spec_version = 99;
    request.extraction = ExtractionSpec::selector(selector_query("["));

    let diagnostics = validate_request(&request).expect_err("invalid request");
    assert!(has_errors(&diagnostics));
    assert!(
        diagnostics
            .iter()
            .any(|item| item.code == "UNSUPPORTED_SPEC_VERSION")
    );
    assert!(
        diagnostics
            .iter()
            .any(|item| item.code == "INVALID_SELECTOR")
    );

    let mut selected_html_request = selector_request("<article>Hello</article>");
    selected_html_request.extraction = selected_html_request
        .extraction
        .clone()
        .with_value(ValueSpec::SelectedHtml);
    let diagnostics = validate_request(&selected_html_request)
        .expect_err("selected html on selector should fail");
    assert!(
        diagnostics
            .iter()
            .any(|item| item.code == "UNSUPPORTED_VALUE_TYPE")
    );
}

#[test]
fn validate_request_accepts_current_requests() {
    let selector = selector_request("<article>Hello</article>");
    assert!(validate_request(&selector).is_ok());

    let mut slice = slice_request(
        "<section data-id=\"7\">Hello</section>",
        "<section",
        "</section>",
    );
    slice.extraction = ExtractionSpec::slice(SliceSpec {
        pattern: SlicePatternSpec::literal(
            slice_boundary("<section"),
            slice_boundary("</section>"),
        ),
        boundary_retention: BoundaryRetention::IncludeBoth,
    })
    .with_selection(nth_selection(1))
    .with_value(attribute_value("data-id"));
    slice.output.preview_chars = NonZeroUsize::new(32).expect("preview chars");

    assert!(validate_request(&slice).is_ok());
}

#[test]
fn extract_rejects_invalid_requests_before_loading_the_source() {
    let missing_file_selector = ExtractionRequest::new(
        file_source("/definitely/missing.html"),
        ExtractionSpec::selector(selector_query("[")),
    );
    let selector_result = extract(&missing_file_selector, &RuntimeOptions::default());
    assert!(!selector_result.ok);
    assert_eq!(selector_result.source.bytes_read, 0);
    assert_eq!(selector_result.diagnostics[0].code, "INVALID_SELECTOR");

    let missing_file_slice = ExtractionRequest::new(
        file_source("/definitely/missing.html"),
        ExtractionSpec::slice(regex_slice_spec("[", "</article>")),
    );
    let slice_result = extract(&missing_file_slice, &RuntimeOptions::default());
    assert!(!slice_result.ok);
    assert_eq!(slice_result.source.bytes_read, 0);
    assert_eq!(slice_result.diagnostics[0].code, "INVALID_SLICE_PATTERN");
}
