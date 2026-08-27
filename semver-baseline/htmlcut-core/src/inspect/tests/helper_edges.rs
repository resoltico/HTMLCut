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
    assert_eq!(cleaner_descendants.len(), 2);
    assert_eq!(cleaner_descendants[1].inspection.selector, "#content");

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
    assert_eq!(
        selector_stability_rank("article[itemprop=\"articleBody\"]"),
        4
    );
    assert_eq!(selector_stability_rank("article[role=\"main\"]"), 4);

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
fn promotion_eligibility_helpers_preserve_exact_content_boundaries() {
    assert!(title_bearing_reading_ancestor_is_promotable(
        1_000, 2, 70, 850, 4, 10,
    ));
    assert!(!title_bearing_reading_ancestor_is_promotable(
        1_000, 2, 70, 849, 4, 10,
    ));
    assert!(!title_bearing_reading_ancestor_is_promotable(
        1_000, 1, 70, 850, 4, 10,
    ));
    assert!(!title_bearing_reading_ancestor_is_promotable(
        1_000, 3, 70, 850, 6, 10,
    ));
    assert!(!title_bearing_reading_ancestor_is_promotable(
        1_000, 2, 71, 850, 4, 10,
    ));

    assert!(cleaner_reading_descendant_is_promotable(
        1_000, 5, 30, 900, 3, 10,
    ));
    assert!(!cleaner_reading_descendant_is_promotable(
        1_000, 5, 30, 899, 3, 10,
    ));
    assert!(!cleaner_reading_descendant_is_promotable(
        1_000, 6, 30, 900, 3, 10,
    ));
    assert!(!cleaner_reading_descendant_is_promotable(
        1_000, 5, 30, 900, 2, 10,
    ));
    assert!(!cleaner_reading_descendant_is_promotable(
        1_000, 5, 30, 900, 3, 11,
    ));

    assert_eq!(content_tag_rank("section"), 2);
    assert!(outer_wrapper_adds_heading_shell(
        HeadingShellCandidate {
            text_char_count: 1_000,
            heading_count: 26,
            link_count: 12,
            selector: "#outer",
        },
        HeadingShellCandidate {
            text_char_count: 940,
            heading_count: 2,
            link_count: 0,
            selector: "#inner",
        },
    ));
    assert!(!outer_wrapper_adds_heading_shell(
        HeadingShellCandidate {
            text_char_count: 1_000,
            heading_count: 26,
            link_count: 12,
            selector: "#outer",
        },
        HeadingShellCandidate {
            text_char_count: 939,
            heading_count: 2,
            link_count: 0,
            selector: "#inner",
        },
    ));

    let document = Html::parse_document(
        "<main id='scope'><nav>Menu</nav><article>Story</article><footer>Legal</footer></main>",
    );
    let scope = select_first(&document, "#scope").expect("candidate scope");
    assert_eq!(count_utility_descendant_roots(&scope), 2);

    let linked_document = Html::parse_document("<main><a id='target' href='/docs'>Docs</a></main>");
    let link = select_first(&linked_document, "#target").expect("link candidate");
    assert_eq!(samples::path_hint_for_link(&link), build_node_path(&link));
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

#[test]
fn content_candidate_score_policy_helpers_preserve_exact_boundaries() {
    assert_eq!(
        content_candidate_utility_multiplier(CandidatePreference::Extraction, "article", true),
        18
    );
    assert_eq!(
        content_candidate_utility_multiplier(CandidatePreference::Extraction, "div", true),
        24
    );
    assert_eq!(
        content_candidate_utility_multiplier(CandidatePreference::Reading, "article", false),
        18
    );
    assert_eq!(
        content_candidate_utility_multiplier(CandidatePreference::Reading, "main", true),
        12
    );

    assert_eq!(
        content_candidate_body_absence_penalty(CandidatePreference::Extraction, 0, 499),
        200
    );
    assert_eq!(
        content_candidate_body_absence_penalty(CandidatePreference::Extraction, 1, 419),
        95
    );
    assert_eq!(
        content_candidate_body_absence_penalty(CandidatePreference::Reading, 1, 320),
        0
    );

    for (preference, limit, penalty) in [
        (CandidatePreference::Extraction, 420, 200),
        (CandidatePreference::Reading, 300, 170),
    ] {
        assert_eq!(
            content_candidate_title_fragment_penalty(preference, "div", true, 0, limit - 1),
            penalty
        );
        assert_eq!(
            content_candidate_title_fragment_penalty(preference, "div", true, 0, limit),
            0
        );
        assert_eq!(
            content_candidate_title_fragment_penalty(preference, "div", true, 0, limit + 1),
            0
        );
    }
    assert_eq!(
        content_candidate_title_fragment_penalty(
            CandidatePreference::Extraction,
            "article",
            true,
            0,
            419,
        ),
        0
    );

    assert_eq!(
        content_candidate_link_density_penalty(CandidatePreference::Extraction, 1_000, 12, 2),
        60
    );
    assert_eq!(
        content_candidate_link_density_penalty(CandidatePreference::Extraction, 239, 8, 2),
        34
    );
    assert_eq!(
        content_candidate_link_density_penalty(CandidatePreference::Extraction, 1_599, 13, 2),
        25
    );
    assert_eq!(
        content_candidate_link_density_penalty(CandidatePreference::Extraction, 1_600, 13, 2),
        60
    );
    assert_eq!(
        content_candidate_link_density_penalty(CandidatePreference::Extraction, 6_499, 9, 2),
        60
    );
    assert_eq!(
        content_candidate_link_density_penalty(CandidatePreference::Extraction, 6_500, 9, 2),
        34
    );
    assert_eq!(
        content_candidate_link_density_penalty(CandidatePreference::Extraction, 11_999, 7, 2),
        34
    );
    assert_eq!(
        content_candidate_link_density_penalty(CandidatePreference::Extraction, 12_000, 7, 2),
        0
    );

    assert_eq!(
        content_candidate_link_density_penalty(CandidatePreference::Reading, 1_000, 12, 2),
        40
    );
    assert_eq!(
        content_candidate_link_density_penalty(CandidatePreference::Reading, 1_199, 13, 2),
        15
    );
    assert_eq!(
        content_candidate_link_density_penalty(CandidatePreference::Reading, 1_200, 13, 2),
        40
    );
    assert_eq!(
        content_candidate_link_density_penalty(CandidatePreference::Reading, 3_999, 9, 2),
        40
    );
    assert_eq!(
        content_candidate_link_density_penalty(CandidatePreference::Reading, 4_000, 9, 2),
        22
    );
    assert_eq!(
        content_candidate_link_density_penalty(CandidatePreference::Reading, 5_999, 7, 2),
        22
    );
    assert_eq!(
        content_candidate_link_density_penalty(CandidatePreference::Reading, 6_000, 7, 2),
        0
    );
}
