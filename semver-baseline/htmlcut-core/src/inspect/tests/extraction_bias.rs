use super::support::*;
use super::*;

#[test]
fn extraction_specific_nested_bias_and_ordering_helpers_cover_remaining_paths() {
    let mut near_full_inner = vec![
        ranked_bias_candidate(BiasFixture {
            selector: "main",
            path: "html > body > main",
            tag_name: "main",
            text_char_count: 1_000,
            heading_count: 3,
            link_count: 18,
            paragraph_count: 3,
            primary_heading_level: None,
            primary_heading_count: 0,
            primary_heading_depth: None,
            utility_descendant_count: 0,
            score: 200,
        }),
        ranked_bias_candidate(BiasFixture {
            selector: "main > article",
            path: "html > body > main > article",
            tag_name: "article",
            text_char_count: 930,
            heading_count: 2,
            link_count: 6,
            paragraph_count: 3,
            primary_heading_level: None,
            primary_heading_count: 0,
            primary_heading_depth: None,
            utility_descendant_count: 0,
            score: 200,
        }),
    ];
    apply_nested_content_candidate_bias_for(&mut near_full_inner, CandidatePreference::Extraction);
    assert!(near_full_inner[1].score > near_full_inner[0].score);

    let mut utility_driven_inner = vec![
        ranked_bias_candidate(BiasFixture {
            selector: "main",
            path: "html > body > main",
            tag_name: "main",
            text_char_count: 1_000,
            heading_count: 3,
            link_count: 9,
            paragraph_count: 3,
            primary_heading_level: None,
            primary_heading_count: 0,
            primary_heading_depth: None,
            utility_descendant_count: 4,
            score: 200,
        }),
        ranked_bias_candidate(BiasFixture {
            selector: "main > article",
            path: "html > body > main > article",
            tag_name: "article",
            text_char_count: 930,
            heading_count: 2,
            link_count: 6,
            paragraph_count: 3,
            primary_heading_level: None,
            primary_heading_count: 0,
            primary_heading_depth: None,
            utility_descendant_count: 0,
            score: 200,
        }),
    ];
    apply_nested_content_candidate_bias_for(
        &mut utility_driven_inner,
        CandidatePreference::Extraction,
    );
    assert!(utility_driven_inner[1].score > utility_driven_inner[0].score);

    let mut overwhelmingly_large_outer = vec![
        ranked_bias_candidate(BiasFixture {
            selector: "main",
            path: "html > body > main",
            tag_name: "main",
            text_char_count: 1_200,
            heading_count: 6,
            link_count: 2,
            paragraph_count: 6,
            primary_heading_level: None,
            primary_heading_count: 0,
            primary_heading_depth: None,
            utility_descendant_count: 0,
            score: 120,
        }),
        ranked_bias_candidate(BiasFixture {
            selector: "main > section",
            path: "html > body > main > section",
            tag_name: "section",
            text_char_count: 150,
            heading_count: 1,
            link_count: 0,
            paragraph_count: 1,
            primary_heading_level: None,
            primary_heading_count: 0,
            primary_heading_depth: None,
            utility_descendant_count: 0,
            score: 120,
        }),
    ];
    apply_nested_content_candidate_bias_for(
        &mut overwhelmingly_large_outer,
        CandidatePreference::Extraction,
    );
    assert!(overwhelmingly_large_outer[0].score > overwhelmingly_large_outer[1].score);

    let mut utility_heavy_outer = vec![
        ranked_bias_candidate(BiasFixture {
            selector: "main",
            path: "html > body > main",
            tag_name: "main",
            text_char_count: 1_000,
            heading_count: 2,
            link_count: 3,
            paragraph_count: 2,
            primary_heading_level: None,
            primary_heading_count: 0,
            primary_heading_depth: None,
            utility_descendant_count: 10,
            score: 80,
        }),
        ranked_bias_candidate(BiasFixture {
            selector: "main > article",
            path: "html > body > main > article",
            tag_name: "article",
            text_char_count: 800,
            heading_count: 2,
            link_count: 0,
            paragraph_count: 2,
            primary_heading_level: None,
            primary_heading_count: 0,
            primary_heading_depth: None,
            utility_descendant_count: 0,
            score: 80,
        }),
    ];
    apply_nested_content_candidate_bias_for(
        &mut utility_heavy_outer,
        CandidatePreference::Extraction,
    );
    assert!(utility_heavy_outer[1].score > utility_heavy_outer[0].score);

    let mut heading_rich_outer = vec![
        ranked_bias_candidate(BiasFixture {
            selector: "div.wrapper",
            path: "html > body > div.wrapper",
            tag_name: "div",
            text_char_count: 1_000,
            heading_count: 6,
            link_count: 3,
            paragraph_count: 2,
            primary_heading_level: None,
            primary_heading_count: 0,
            primary_heading_depth: None,
            utility_descendant_count: 0,
            score: 160,
        }),
        ranked_bias_candidate(BiasFixture {
            selector: "div.wrapper > section",
            path: "html > body > div.wrapper > section",
            tag_name: "section",
            text_char_count: 850,
            heading_count: 1,
            link_count: 4,
            paragraph_count: 2,
            primary_heading_level: None,
            primary_heading_count: 0,
            primary_heading_depth: None,
            utility_descendant_count: 0,
            score: 160,
        }),
    ];
    apply_nested_content_candidate_bias_for(
        &mut heading_rich_outer,
        CandidatePreference::Extraction,
    );
    assert!(heading_rich_outer[0].score > heading_rich_outer[1].score);

    let mut modest_heading_outer = vec![
        ranked_bias_candidate(BiasFixture {
            selector: "div.wrapper",
            path: "html > body > div.wrapper",
            tag_name: "div",
            text_char_count: 1_000,
            heading_count: 4,
            link_count: 3,
            paragraph_count: 2,
            primary_heading_level: None,
            primary_heading_count: 0,
            primary_heading_depth: None,
            utility_descendant_count: 0,
            score: 210,
        }),
        ranked_bias_candidate(BiasFixture {
            selector: "div.wrapper > section",
            path: "html > body > div.wrapper > section",
            tag_name: "section",
            text_char_count: 920,
            heading_count: 2,
            link_count: 4,
            paragraph_count: 2,
            primary_heading_level: None,
            primary_heading_count: 0,
            primary_heading_depth: None,
            utility_descendant_count: 0,
            score: 210,
        }),
    ];
    apply_nested_content_candidate_bias_for(
        &mut modest_heading_outer,
        CandidatePreference::Extraction,
    );
    assert!(modest_heading_outer[0].score > modest_heading_outer[1].score);

    let extraction_unknown_tag = content_candidate_score_for(
        &ContentCandidateScoreInputs {
            tag_name: "aside",
            has_main_role: false,
            has_article_body_itemprop: false,
            text_char_count: 800,
            heading_count: 1,
            link_count: 0,
            paragraph_count: 2,
            positive_signal_count: 0,
            negative_signal_count: 0,
            primary_heading_level: None,
            primary_heading_count: 0,
            primary_heading_depth: None,
            utility_descendant_count: 0,
            uses_exact_path_selector: false,
        },
        CandidatePreference::Extraction,
    );
    let extraction_dense_short = content_candidate_score_for(
        &ContentCandidateScoreInputs {
            tag_name: "section",
            has_main_role: false,
            has_article_body_itemprop: false,
            text_char_count: 220,
            heading_count: 1,
            link_count: 9,
            paragraph_count: 1,
            positive_signal_count: 0,
            negative_signal_count: 0,
            primary_heading_level: None,
            primary_heading_count: 0,
            primary_heading_depth: None,
            utility_descendant_count: 0,
            uses_exact_path_selector: false,
        },
        CandidatePreference::Extraction,
    );
    let extraction_dense_medium = content_candidate_score_for(
        &ContentCandidateScoreInputs {
            tag_name: "section",
            has_main_role: false,
            has_article_body_itemprop: false,
            text_char_count: 1_500,
            heading_count: 1,
            link_count: 13,
            paragraph_count: 2,
            positive_signal_count: 0,
            negative_signal_count: 0,
            primary_heading_level: None,
            primary_heading_count: 0,
            primary_heading_depth: None,
            utility_descendant_count: 0,
            uses_exact_path_selector: false,
        },
        CandidatePreference::Extraction,
    );
    let extraction_dense_large = content_candidate_score_for(
        &ContentCandidateScoreInputs {
            tag_name: "section",
            has_main_role: false,
            has_article_body_itemprop: false,
            text_char_count: 6_000,
            heading_count: 1,
            link_count: 9,
            paragraph_count: 2,
            positive_signal_count: 0,
            negative_signal_count: 0,
            primary_heading_level: None,
            primary_heading_count: 0,
            primary_heading_depth: None,
            utility_descendant_count: 0,
            uses_exact_path_selector: false,
        },
        CandidatePreference::Extraction,
    );
    let extraction_dense_wide = content_candidate_score_for(
        &ContentCandidateScoreInputs {
            tag_name: "section",
            has_main_role: false,
            has_article_body_itemprop: false,
            text_char_count: 10_000,
            heading_count: 1,
            link_count: 7,
            paragraph_count: 2,
            positive_signal_count: 0,
            negative_signal_count: 0,
            primary_heading_level: None,
            primary_heading_count: 0,
            primary_heading_depth: None,
            utility_descendant_count: 0,
            uses_exact_path_selector: false,
        },
        CandidatePreference::Extraction,
    );
    assert!(extraction_unknown_tag > extraction_dense_short);
    assert!(extraction_dense_medium != extraction_dense_large);
    assert!(extraction_dense_large != extraction_dense_wide);

    let extraction_link_order_left = ranked_bias_candidate(BiasFixture {
        selector: "#alpha",
        path: "html > body > article",
        tag_name: "article",
        text_char_count: 500,
        heading_count: 2,
        link_count: 2,
        paragraph_count: 2,
        primary_heading_level: None,
        primary_heading_count: 0,
        primary_heading_depth: None,
        utility_descendant_count: 0,
        score: 100,
    });
    let extraction_link_order_right = ranked_bias_candidate(BiasFixture {
        selector: "#beta",
        path: "html > body > article",
        tag_name: "article",
        text_char_count: 500,
        heading_count: 2,
        link_count: 4,
        paragraph_count: 2,
        primary_heading_level: None,
        primary_heading_count: 0,
        primary_heading_depth: None,
        utility_descendant_count: 0,
        score: 100,
    });
    assert_eq!(
        compare_content_candidates_for(
            &extraction_link_order_left,
            &extraction_link_order_right,
            CandidatePreference::Extraction,
        ),
        Ordering::Less
    );

    let extraction_depth_left = ranked_bias_candidate(BiasFixture {
        selector: "#deep",
        path: "html > body > main > article",
        tag_name: "article",
        text_char_count: 500,
        heading_count: 2,
        link_count: 2,
        paragraph_count: 2,
        primary_heading_level: None,
        primary_heading_count: 0,
        primary_heading_depth: None,
        utility_descendant_count: 0,
        score: 100,
    });
    let extraction_depth_right = ranked_bias_candidate(BiasFixture {
        selector: "#shallow",
        path: "html > body > main",
        tag_name: "main",
        text_char_count: 500,
        heading_count: 2,
        link_count: 2,
        paragraph_count: 2,
        primary_heading_level: None,
        primary_heading_count: 0,
        primary_heading_depth: None,
        utility_descendant_count: 0,
        score: 100,
    });
    assert_eq!(
        compare_content_candidates_for(
            &extraction_depth_left,
            &extraction_depth_right,
            CandidatePreference::Extraction,
        ),
        Ordering::Less
    );
    assert_eq!(path_depth("html > body > main > article"), 3);

    let extraction_text_left = ranked_bias_candidate(BiasFixture {
        selector: "#longer",
        path: "html > body > main",
        tag_name: "main",
        text_char_count: 700,
        heading_count: 2,
        link_count: 2,
        paragraph_count: 2,
        primary_heading_level: None,
        primary_heading_count: 0,
        primary_heading_depth: None,
        utility_descendant_count: 0,
        score: 100,
    });
    let extraction_text_right = ranked_bias_candidate(BiasFixture {
        selector: "#shorter",
        path: "html > body > navx",
        tag_name: "section",
        text_char_count: 500,
        heading_count: 2,
        link_count: 2,
        paragraph_count: 2,
        primary_heading_level: None,
        primary_heading_count: 0,
        primary_heading_depth: None,
        utility_descendant_count: 0,
        score: 100,
    });
    assert_eq!(
        compare_content_candidates_for(
            &extraction_text_left,
            &extraction_text_right,
            CandidatePreference::Extraction,
        ),
        Ordering::Less
    );

    let extraction_selector_alpha = ranked_bias_candidate(BiasFixture {
        selector: "#alpha",
        path: "html > body > nav1",
        tag_name: "section",
        text_char_count: 500,
        heading_count: 2,
        link_count: 2,
        paragraph_count: 2,
        primary_heading_level: None,
        primary_heading_count: 0,
        primary_heading_depth: None,
        utility_descendant_count: 0,
        score: 100,
    });
    let extraction_selector_beta = ranked_bias_candidate(BiasFixture {
        selector: "#beta",
        path: "html > body > nav2",
        tag_name: "section",
        text_char_count: 500,
        heading_count: 2,
        link_count: 2,
        paragraph_count: 2,
        primary_heading_level: None,
        primary_heading_count: 0,
        primary_heading_depth: None,
        utility_descendant_count: 0,
        score: 100,
    });
    assert_eq!(
        compare_content_candidates_for(
            &extraction_selector_alpha,
            &extraction_selector_beta,
            CandidatePreference::Extraction,
        ),
        Ordering::Less
    );
    assert_eq!(content_tag_rank("div"), 1);
    assert_eq!(content_tag_rank("aside"), 0);
    assert_eq!(content_tag_rank("main"), 3);
}
