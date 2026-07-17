use super::*;

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
