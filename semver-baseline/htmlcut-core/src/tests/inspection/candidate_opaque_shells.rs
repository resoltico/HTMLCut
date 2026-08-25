use super::*;

use crate::inspect::opaque_div_has_long_form_shape_for_tests;

#[test]
fn opaque_div_shape_requires_every_threshold_at_its_exact_boundary() {
    assert!(opaque_div_has_long_form_shape_for_tests(800, 3, 1));
    assert!(!opaque_div_has_long_form_shape_for_tests(799, 3, 1));
    assert!(!opaque_div_has_long_form_shape_for_tests(800, 2, 1));
    assert!(!opaque_div_has_long_form_shape_for_tests(800, 3, 0));
    assert!(!opaque_div_has_long_form_shape_for_tests(800, 3, 2));
}

#[test]
fn inspect_source_promotes_a_long_opaque_div_anchored_by_a_nearby_h1() {
    let article_paragraphs = (1..=8)
        .map(|index| {
            format!(
                "<p>Long-form legal paragraph {index}: alpha beta gamma delta epsilon zeta eta theta iota kappa lambda mu nu xi omicron pi rho sigma tau upsilon phi chi psi omega.</p>"
            )
        })
        .collect::<String>();
    let repeated_prose = "alpha beta gamma delta epsilon zeta eta theta iota kappa lambda mu nu xi omicron pi rho sigma tau upsilon phi chi psi omega. ".repeat(8);
    let two_long_paragraphs = (1..=2)
        .map(|index| format!("<p>Two-paragraph guard {index}: {repeated_prose}</p>"))
        .collect::<String>();
    let html = format!(
        "<html><body><h1>   </h1><div id=\"opaque-root\"><div><span><div><div><h1>Host Damage Protection Terms</h1></div></div></span></div>{article_paragraphs}<p><a href=\"/body-guide\">Body guide</a></p></div><div class=\"article-card\"><h2>Related article</h2><p>Short related summary one.</p><p>Short related summary two.</p><p><a href=\"/related\">Related link</a></p></div><div id=\"opaque-short\"><div><h1>Short promo</h1></div><p>One paragraph only.</p><p>Two paragraphs only.</p></div><div id=\"opaque-two-paragraph\"><div><h1>Two paragraph guard</h1></div>{two_long_paragraphs}</div><div id=\"opaque-two-title\"><div><h1>First title</h1><h1>Second title</h1></div>{article_paragraphs}</div><div id=\"too-distant-root\"><div><div><div><div><div><div><div><h1>Distant page shell</h1></div></div></div></div></div></div></div>{article_paragraphs}</div></body></html>"
    );
    let inspection = inspect_source(
        &memory_source_with_base("opaque-shell.html", html, "https://example.test/help/terms"),
        &RuntimeOptions::default(),
        &InspectionOptions {
            include_source_text: false,
            sample_limit: 4,
        },
    );

    assert!(inspection.ok);
    let document = inspection.document.expect("document inspection");
    assert_eq!(document.extraction_candidates[0].selector, "#opaque-root");
    assert_eq!(document.reading_candidates[0].selector, "#opaque-root");
    assert!(document.extraction_candidates[0].text_char_count > 800);
    assert!(document.reading_candidates[0].text_char_count > 800);
    for candidates in [
        &document.extraction_candidates,
        &document.reading_candidates,
    ] {
        for excluded in [
            "#opaque-short",
            "#opaque-two-paragraph",
            "#opaque-two-title",
            "#too-distant-root",
        ] {
            assert!(
                candidates
                    .iter()
                    .all(|candidate| candidate.selector != excluded)
            );
        }
    }
    assert_eq!(document.headings[0].text, "Host Damage Protection Terms");
    assert!(
        document
            .headings
            .iter()
            .all(|heading| heading.text != "Related article")
    );
    assert_eq!(
        document.links.first().and_then(|link| link.href.as_deref()),
        Some("/body-guide")
    );
}
