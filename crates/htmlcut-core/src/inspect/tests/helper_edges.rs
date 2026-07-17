use super::support::*;
use super::*;

#[test]
fn promotion_helpers_cover_path_depth_empty_inputs_and_cleaner_descendants() {
    let mut empty = Vec::new();
    promote_cleaner_reading_descendant_candidate(&mut empty, &[]);
    promote_title_bearing_reading_ancestor_candidate(&mut empty, &[]);

    let mut cleaner_descendants = vec![ranked_content_candidate(PromotionFixture {
        selector: "#content",
        path: "html > body > main#content",
        tag_name: "main",
        text_char_count: 1_000,
        heading_count: 3,
        link_count: 80,
        primary_heading_level: Some(1),
        primary_heading_depth: Some(1),
    })];
    let reading_candidates = vec![
        cleaner_descendants[0].clone(),
        ranked_content_candidate(PromotionFixture {
            selector: ".story",
            path: "html > body > main#content > section > article.story",
            tag_name: "article",
            text_char_count: 930,
            heading_count: 3,
            link_count: 40,
            primary_heading_level: Some(1),
            primary_heading_depth: Some(2),
        }),
        ranked_content_candidate(PromotionFixture {
            selector: ".story-body",
            path: "html > body > main#content > article.story-body",
            tag_name: "article",
            text_char_count: 930,
            heading_count: 3,
            link_count: 40,
            primary_heading_level: Some(1),
            primary_heading_depth: Some(2),
        }),
    ];
    promote_cleaner_reading_descendant_candidate(&mut cleaner_descendants, &reading_candidates);
    assert_eq!(cleaner_descendants[0].inspection.selector, ".story-body");

    let mut precise_descendants = vec![ranked_content_candidate(PromotionFixture {
        selector: "main.layout",
        path: "html > body > main.layout",
        tag_name: "main",
        text_char_count: 1_000,
        heading_count: 3,
        link_count: 100,
        primary_heading_level: None,
        primary_heading_depth: None,
    })];
    let precise_reading = vec![
        ranked_content_candidate(PromotionFixture {
            selector: "article.story:nth-of-type(1)",
            path: "html > body > main.layout > section > article.story:nth-of-type(1)",
            tag_name: "article",
            text_char_count: 920,
            heading_count: 3,
            link_count: 60,
            primary_heading_level: None,
            primary_heading_depth: None,
        }),
        ranked_content_candidate(PromotionFixture {
            selector: "article.story",
            path: "html > body > main.layout > article.story",
            tag_name: "article",
            text_char_count: 920,
            heading_count: 3,
            link_count: 60,
            primary_heading_level: None,
            primary_heading_depth: None,
        }),
    ];
    promote_precise_reading_descendant_candidate(&mut precise_descendants, &precise_reading);
    assert_eq!(precise_descendants[0].inspection.selector, "article.story");

    let mut precise_path_depth_tie = vec![ranked_content_candidate(PromotionFixture {
        selector: "main.layout",
        path: "html > body > main.layout",
        tag_name: "main",
        text_char_count: 1_000,
        heading_count: 3,
        link_count: 100,
        primary_heading_level: None,
        primary_heading_depth: None,
    })];
    let precise_path_depth_reading = vec![
        ranked_content_candidate(PromotionFixture {
            selector: "article.story-a",
            path: "html > body > main.layout > section > article.story-a",
            tag_name: "article",
            text_char_count: 920,
            heading_count: 3,
            link_count: 60,
            primary_heading_level: None,
            primary_heading_depth: None,
        }),
        ranked_content_candidate(PromotionFixture {
            selector: "article.story-b",
            path: "html > body > main.layout > article.story-b",
            tag_name: "article",
            text_char_count: 920,
            heading_count: 3,
            link_count: 60,
            primary_heading_level: None,
            primary_heading_depth: None,
        }),
    ];
    promote_precise_reading_descendant_candidate(
        &mut precise_path_depth_tie,
        &precise_path_depth_reading,
    );
    assert_eq!(
        precise_path_depth_tie[0].inspection.selector,
        "article.story-b"
    );
}

#[test]
fn selector_rank_and_link_preview_helpers_cover_attribute_and_reference_edges() {
    assert_eq!(selector_stability_rank("[itemprop=\"articleBody\"]"), 4);
    assert_eq!(selector_stability_rank("[data-surface=\"story\"]"), 4);

    assert!(!link_preview_is_low_signal(
        "#cite-note",
        "   ",
        "article > p"
    ));
    assert!(link_preview_is_low_signal(
        "#cite-note",
        "[12]",
        "article > p"
    ));
    assert!(link_preview_is_low_signal(
        "#cite-note",
        "*",
        "article > sup:nth-of-type(2)"
    ));
    assert!(link_preview_is_low_signal(
        "/privacy/terms",
        "Guide",
        "article > p > a"
    ));
    assert!(link_preview_is_low_signal(
        "/guide",
        "Terms apply",
        "article > p > a"
    ));
    assert!(link_preview_is_low_signal(
        "/guide",
        "Guide",
        "article > footer.related > a"
    ));
    assert!(same_page_url(
        "https://example.test/guide#fragment",
        "https://example.test/guide"
    ));
    assert!(!same_page_url(
        "https://example.test/guide",
        "not a valid url"
    ));
}

#[test]
fn density_and_heading_helpers_cover_link_penalties_and_title_insertion() {
    assert!(!candidate_has_readable_density(
        "section", 1_500, 1, 25, 3, 3
    ));
    assert!(!candidate_has_readable_density("section", 100, 1, 0, 0, 1));
    assert!(candidate_has_readable_density("section", 180, 1, 1, 1, 0));
    assert!(!candidate_has_readable_density("section", 180, 8, 5, 1, 0));
    assert!(!candidate_has_readable_density("section", 180, 1, 20, 1, 0));
    assert!(candidate_has_readable_density("section", 300, 1, 1, 3, 0));
    assert!(candidate_has_readable_density("section", 220, 1, 1, 1, 0));
    assert!(!candidate_has_readable_density("section", 500, 20, 0, 2, 3));
    assert!(!candidate_has_readable_density("section", 500, 1, 30, 2, 3));
    assert!(candidate_has_readable_density(
        "section", 4_000, 20, 0, 2, 3
    ));
    assert!(candidate_has_readable_density(
        "section", 4_000, 1, 30, 2, 3
    ));
    let section_with_itemprop =
        parse_document_node("<section itemprop=\"articleBody\"><p>Alpha</p></section>");
    assert!(is_content_candidate_container(
        &select_first(&section_with_itemprop, "section").expect("section"),
        0,
    ));
    let section_with_role_main =
        parse_document_node("<section role=\"main\"><p>Alpha</p></section>");
    assert!(is_content_candidate_container(
        &select_first(&section_with_role_main, "section").expect("section"),
        0,
    ));
    let div_with_role = parse_document_node("<div role=\"main\"><p>Alpha</p></div>");
    assert!(is_content_candidate_container(
        &select_first(&div_with_role, "div").expect("div"),
        0,
    ));
    let div_with_itemprop = parse_document_node("<div itemprop=\"articleBody\"><p>Alpha</p></div>");
    assert!(is_content_candidate_container(
        &select_first(&div_with_itemprop, "div").expect("div"),
        0,
    ));
    let section_with_three_paragraphs =
        parse_document_node("<section><p>Alpha</p><p>Beta</p><p>Gamma</p></section>");
    assert!(element_has_narrative_section_shape(
        &select_first(&section_with_three_paragraphs, "section").expect("section"),
    ));
    let section_with_heading_and_list = parse_document_node(
        "<section><h2>Body</h2><p>Alpha</p><p>Beta</p><ul><li>One</li><li>Two</li></ul></section>",
    );
    assert!(element_has_narrative_section_shape(
        &select_first(&section_with_heading_and_list, "section").expect("section"),
    ));
    let section_with_list_shape = parse_document_node(
        "<section><p>Alpha</p><p>Beta</p><ul><li>One</li><li>Two</li></ul></section>",
    );
    assert!(element_has_narrative_section_shape(
        &select_first(&section_with_list_shape, "section").expect("section"),
    ));

    let document = parse_document_node(
        "<html><body><h1>Document Title</h1><section><h2>Body</h2><p>Alpha beta gamma delta epsilon zeta eta theta iota.</p></section></body></html>",
    );
    let mut zero_limit_headings = vec![HeadingInspection {
        level: 2,
        text: "Body".to_owned(),
        path: "html > body > section > h2".to_owned(),
    }];
    prepend_document_title_heading_if_missing(&document, 0, &mut zero_limit_headings);
    assert_eq!(zero_limit_headings[0].level, 2);
    let mut existing_h1 = vec![HeadingInspection {
        level: 1,
        text: "Document Title".to_owned(),
        path: "html > body > h1".to_owned(),
    }];
    prepend_document_title_heading_if_missing(&document, 3, &mut existing_h1);
    assert_eq!(existing_h1.len(), 1);
    let mut headings = vec![HeadingInspection {
        level: 2,
        text: "Body".to_owned(),
        path: "html > body > section > h2".to_owned(),
    }];
    let mut unconstrained_headings = headings.clone();
    prepend_document_title_heading_if_missing(&document, 3, &mut unconstrained_headings);
    assert_eq!(unconstrained_headings.len(), 2);
    assert_eq!(unconstrained_headings[0].level, 1);
    prepend_document_title_heading_if_missing(&document, 1, &mut headings);
    assert_eq!(headings.len(), 1);
    assert_eq!(headings[0].level, 1);
    assert_eq!(headings[0].text, "Document Title");

    let extraction_penalized = content_candidate_score_for(
        &ContentCandidateScoreInputs {
            tag_name: "section",
            has_main_role: false,
            has_article_body_itemprop: false,
            text_char_count: 300,
            heading_count: 1,
            link_count: 0,
            paragraph_count: 0,
            positive_signal_count: 0,
            negative_signal_count: 0,
            primary_heading_level: Some(1),
            primary_heading_count: 1,
            primary_heading_depth: Some(1),
            utility_descendant_count: 0,
            uses_exact_path_selector: false,
        },
        CandidatePreference::Extraction,
    );
    let reading_penalized = content_candidate_score_for(
        &ContentCandidateScoreInputs {
            tag_name: "section",
            has_main_role: false,
            has_article_body_itemprop: false,
            text_char_count: 220,
            heading_count: 1,
            link_count: 0,
            paragraph_count: 0,
            positive_signal_count: 0,
            negative_signal_count: 0,
            primary_heading_level: Some(1),
            primary_heading_count: 1,
            primary_heading_depth: Some(1),
            utility_descendant_count: 0,
            uses_exact_path_selector: false,
        },
        CandidatePreference::Reading,
    );
    let article_baseline = content_candidate_score_for(
        &ContentCandidateScoreInputs {
            tag_name: "article",
            has_main_role: false,
            has_article_body_itemprop: false,
            text_char_count: 220,
            heading_count: 1,
            link_count: 0,
            paragraph_count: 2,
            positive_signal_count: 0,
            negative_signal_count: 0,
            primary_heading_level: Some(1),
            primary_heading_count: 1,
            primary_heading_depth: Some(1),
            utility_descendant_count: 0,
            uses_exact_path_selector: false,
        },
        CandidatePreference::Reading,
    );
    assert!(extraction_penalized < article_baseline);
    assert!(reading_penalized < article_baseline);
}
