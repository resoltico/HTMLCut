use super::*;

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
