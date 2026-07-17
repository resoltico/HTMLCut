use super::*;

#[test]
fn scoring_penalties_and_heading_helpers_stay_ordered() {
    let dense_links_short = content_candidate_score(&ContentCandidateScoreInputs {
        tag_name: "aside",
        has_main_role: false,
        has_article_body_itemprop: false,
        text_char_count: 200,
        heading_count: 0,
        link_count: 9,
        paragraph_count: 1,
        positive_signal_count: 0,
        negative_signal_count: 0,
        primary_heading_level: None,
        primary_heading_count: 0,
        primary_heading_depth: None,
        utility_descendant_count: 0,
        uses_exact_path_selector: false,
    });
    let dense_links_medium = content_candidate_score(&ContentCandidateScoreInputs {
        tag_name: "section",
        has_main_role: false,
        has_article_body_itemprop: false,
        text_char_count: 1_000,
        heading_count: 1,
        link_count: 13,
        paragraph_count: 2,
        positive_signal_count: 0,
        negative_signal_count: 0,
        primary_heading_level: Some(3),
        primary_heading_count: 1,
        primary_heading_depth: Some(1),
        utility_descendant_count: 0,
        uses_exact_path_selector: false,
    });
    let dense_links_large = content_candidate_score(&ContentCandidateScoreInputs {
        tag_name: "div",
        has_main_role: false,
        has_article_body_itemprop: false,
        text_char_count: 3_000,
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
    });
    let dense_links_wide = content_candidate_score(&ContentCandidateScoreInputs {
        tag_name: "div",
        has_main_role: false,
        has_article_body_itemprop: false,
        text_char_count: 5_000,
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
    });
    let bodyless = content_candidate_score(&ContentCandidateScoreInputs {
        tag_name: "section",
        has_main_role: false,
        has_article_body_itemprop: false,
        text_char_count: 100,
        heading_count: 0,
        link_count: 0,
        paragraph_count: 0,
        positive_signal_count: 0,
        negative_signal_count: 0,
        primary_heading_level: None,
        primary_heading_count: 0,
        primary_heading_depth: None,
        utility_descendant_count: 0,
        uses_exact_path_selector: false,
    });
    let near_bodyless = content_candidate_score(&ContentCandidateScoreInputs {
        tag_name: "section",
        has_main_role: false,
        has_article_body_itemprop: false,
        text_char_count: 200,
        heading_count: 0,
        link_count: 0,
        paragraph_count: 1,
        positive_signal_count: 0,
        negative_signal_count: 0,
        primary_heading_level: None,
        primary_heading_count: 0,
        primary_heading_depth: None,
        utility_descendant_count: 0,
        uses_exact_path_selector: false,
    });
    let bodyless_but_long = content_candidate_score(&ContentCandidateScoreInputs {
        tag_name: "section",
        has_main_role: false,
        has_article_body_itemprop: false,
        text_char_count: 520,
        heading_count: 0,
        link_count: 0,
        paragraph_count: 0,
        positive_signal_count: 0,
        negative_signal_count: 0,
        primary_heading_level: None,
        primary_heading_count: 0,
        primary_heading_depth: None,
        utility_descendant_count: 0,
        uses_exact_path_selector: false,
    });
    let near_bodyless_but_long = content_candidate_score(&ContentCandidateScoreInputs {
        tag_name: "section",
        has_main_role: false,
        has_article_body_itemprop: false,
        text_char_count: 400,
        heading_count: 0,
        link_count: 0,
        paragraph_count: 1,
        positive_signal_count: 0,
        negative_signal_count: 0,
        primary_heading_level: None,
        primary_heading_count: 0,
        primary_heading_depth: None,
        utility_descendant_count: 0,
        uses_exact_path_selector: false,
    });
    let dense_links_medium_guard = content_candidate_score(&ContentCandidateScoreInputs {
        tag_name: "section",
        has_main_role: false,
        has_article_body_itemprop: false,
        text_char_count: 1_400,
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
    });
    let dense_links_wide_guard = content_candidate_score(&ContentCandidateScoreInputs {
        tag_name: "section",
        has_main_role: false,
        has_article_body_itemprop: false,
        text_char_count: 6_200,
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
    });
    let dense_links_medium_guard_baseline = content_candidate_score(&ContentCandidateScoreInputs {
        tag_name: "section",
        has_main_role: false,
        has_article_body_itemprop: false,
        text_char_count: 1_400,
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
    });
    assert!(dense_links_short < dense_links_medium);
    assert!(dense_links_large != dense_links_wide);
    assert!(bodyless < near_bodyless);
    assert!(bodyless < bodyless_but_long);
    assert!(near_bodyless < near_bodyless_but_long);
    assert!(dense_links_medium_guard < dense_links_medium_guard_baseline);
    assert!(dense_links_wide_guard > dense_links_wide);
    assert_eq!(primary_heading_bonus(1), 130);
    assert_eq!(primary_heading_bonus(2), 78);
    assert_eq!(primary_heading_bonus(3), 20);
    assert_eq!(primary_heading_bonus(9), 0);
    assert!(has_shallow_primary_heading(Some(1), Some(5)));
    assert!(!has_shallow_primary_heading(Some(1), Some(6)));
    assert!(has_shallow_primary_heading(Some(2), Some(2)));
    assert!(!has_shallow_primary_heading(Some(2), Some(3)));
    assert!(drops_outer_title_signal(Some(1), Some(4), Some(2), Some(1)));
    assert!(!drops_outer_title_signal(
        Some(1),
        Some(4),
        Some(1),
        Some(3)
    ));
    assert!(drops_outer_title_signal(Some(2), Some(2), None, None));
    assert!(!drops_outer_title_signal(
        Some(2),
        Some(2),
        Some(1),
        Some(2)
    ));
}
