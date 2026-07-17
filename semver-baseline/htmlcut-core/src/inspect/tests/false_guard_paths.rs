use super::support::*;
use super::*;

#[test]
fn false_guard_paths_and_counting_fallbacks_cover_remaining_edges() {
    let comment_document = parse_document_node(
        "<html><body><!--hidden--><p>Hello</p><img alt=\"Hero\"></body></html>",
    );
    let body = select_first(&comment_document, "body").expect("body");
    let mut counted = String::new();
    collect_visible_text_for_count(body.children(), &mut counted);
    assert_eq!(counted, "Hello Hero");

    let bodyless_extraction_second_guard = content_candidate_score_for(
        &ContentCandidateScoreInputs {
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
        },
        CandidatePreference::Extraction,
    );
    let near_bodyless_extraction_second_guard = content_candidate_score_for(
        &ContentCandidateScoreInputs {
            tag_name: "section",
            has_main_role: false,
            has_article_body_itemprop: false,
            text_char_count: 500,
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
        },
        CandidatePreference::Extraction,
    );
    let title_fragment_guard = content_candidate_score_for(
        &ContentCandidateScoreInputs {
            tag_name: "section",
            has_main_role: false,
            has_article_body_itemprop: false,
            text_char_count: 450,
            heading_count: 1,
            link_count: 0,
            paragraph_count: 0,
            positive_signal_count: 0,
            negative_signal_count: 0,
            primary_heading_level: Some(1),
            primary_heading_count: 1,
            primary_heading_depth: Some(2),
            utility_descendant_count: 0,
            uses_exact_path_selector: false,
        },
        CandidatePreference::Extraction,
    );
    let extraction_medium_link_guard = content_candidate_score_for(
        &ContentCandidateScoreInputs {
            tag_name: "section",
            has_main_role: false,
            has_article_body_itemprop: false,
            text_char_count: 1_700,
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
    let extraction_large_link_guard = content_candidate_score_for(
        &ContentCandidateScoreInputs {
            tag_name: "section",
            has_main_role: false,
            has_article_body_itemprop: false,
            text_char_count: 6_800,
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
    let extraction_wide_link_guard = content_candidate_score_for(
        &ContentCandidateScoreInputs {
            tag_name: "section",
            has_main_role: false,
            has_article_body_itemprop: false,
            text_char_count: 12_500,
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
    let reading_title_fragment_guard = content_candidate_score(&ContentCandidateScoreInputs {
        tag_name: "section",
        has_main_role: false,
        has_article_body_itemprop: false,
        text_char_count: 350,
        heading_count: 1,
        link_count: 0,
        paragraph_count: 0,
        positive_signal_count: 0,
        negative_signal_count: 0,
        primary_heading_level: Some(1),
        primary_heading_count: 1,
        primary_heading_depth: Some(2),
        utility_descendant_count: 0,
        uses_exact_path_selector: false,
    });
    let reading_medium_link_guard = content_candidate_score(&ContentCandidateScoreInputs {
        tag_name: "section",
        has_main_role: false,
        has_article_body_itemprop: false,
        text_char_count: 4_200,
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
    let reading_wide_link_guard = content_candidate_score(&ContentCandidateScoreInputs {
        tag_name: "section",
        has_main_role: false,
        has_article_body_itemprop: false,
        text_char_count: 6_400,
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
    assert_ne!(
        bodyless_extraction_second_guard,
        near_bodyless_extraction_second_guard
    );
    assert_ne!(title_fragment_guard, reading_title_fragment_guard);
    assert!(extraction_medium_link_guard != extraction_large_link_guard);
    assert!(extraction_large_link_guard != extraction_wide_link_guard);
    assert!(reading_medium_link_guard != reading_wide_link_guard);
    assert!(drops_outer_title_signal(Some(1), Some(4), Some(1), Some(6)));
    assert!(drops_outer_title_signal(Some(2), Some(2), Some(2), Some(3)));

    let mut false_edge_cases = vec![
        (
            CandidatePreference::Extraction,
            vec![
                ranked_bias_candidate(BiasFixture {
                    selector: "main",
                    path: "html > body > main",
                    tag_name: "main",
                    text_char_count: 1_000,
                    heading_count: 3,
                    link_count: 12,
                    paragraph_count: 0,
                    primary_heading_level: Some(1),
                    primary_heading_count: 1,
                    primary_heading_depth: Some(1),
                    utility_descendant_count: 2,
                    score: 40,
                }),
                ranked_bias_candidate(BiasFixture {
                    selector: "main > article",
                    path: "html > body > main > article",
                    tag_name: "article",
                    text_char_count: 900,
                    heading_count: 2,
                    link_count: 2,
                    paragraph_count: 2,
                    primary_heading_level: None,
                    primary_heading_count: 0,
                    primary_heading_depth: None,
                    utility_descendant_count: 0,
                    score: 40,
                }),
            ],
            vec![130, 10],
        ),
        (
            CandidatePreference::Extraction,
            vec![
                ranked_bias_candidate(BiasFixture {
                    selector: "main",
                    path: "html > body > main",
                    tag_name: "main",
                    text_char_count: 1_000,
                    heading_count: 3,
                    link_count: 18,
                    paragraph_count: 5,
                    primary_heading_level: None,
                    primary_heading_count: 0,
                    primary_heading_depth: None,
                    utility_descendant_count: 0,
                    score: 10,
                }),
                ranked_bias_candidate(BiasFixture {
                    selector: "main > article",
                    path: "html > body > main > article",
                    tag_name: "article",
                    text_char_count: 930,
                    heading_count: 2,
                    link_count: 6,
                    paragraph_count: 2,
                    primary_heading_level: None,
                    primary_heading_count: 0,
                    primary_heading_depth: None,
                    utility_descendant_count: 0,
                    score: 10,
                }),
            ],
            vec![100, -50],
        ),
        (
            CandidatePreference::Extraction,
            vec![
                ranked_bias_candidate(BiasFixture {
                    selector: "main",
                    path: "html > body > main",
                    tag_name: "main",
                    text_char_count: 1_000,
                    heading_count: 6,
                    link_count: 18,
                    paragraph_count: 3,
                    primary_heading_level: None,
                    primary_heading_count: 0,
                    primary_heading_depth: None,
                    utility_descendant_count: 0,
                    score: 10,
                }),
                ranked_bias_candidate(BiasFixture {
                    selector: "main > article",
                    path: "html > body > main > article",
                    tag_name: "article",
                    text_char_count: 930,
                    heading_count: 3,
                    link_count: 6,
                    paragraph_count: 3,
                    primary_heading_level: None,
                    primary_heading_count: 0,
                    primary_heading_depth: None,
                    utility_descendant_count: 0,
                    score: 10,
                }),
            ],
            vec![100, -50],
        ),
        (
            CandidatePreference::Extraction,
            vec![
                ranked_bias_candidate(BiasFixture {
                    selector: "main",
                    path: "html > body > main",
                    tag_name: "main",
                    text_char_count: 1_000,
                    heading_count: 3,
                    link_count: 10,
                    paragraph_count: 3,
                    primary_heading_level: None,
                    primary_heading_count: 0,
                    primary_heading_depth: None,
                    utility_descendant_count: 1,
                    score: 10,
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
                    score: 10,
                }),
            ],
            vec![100, -50],
        ),
        (
            CandidatePreference::Extraction,
            vec![
                ranked_bias_candidate(BiasFixture {
                    selector: "main",
                    path: "html > body > main",
                    tag_name: "main",
                    text_char_count: 1_200,
                    heading_count: 6,
                    link_count: 2,
                    paragraph_count: 4,
                    primary_heading_level: None,
                    primary_heading_count: 0,
                    primary_heading_depth: None,
                    utility_descendant_count: 0,
                    score: 0,
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
                    score: 0,
                }),
            ],
            vec![0, 0],
        ),
        (
            CandidatePreference::Extraction,
            vec![
                ranked_bias_candidate(BiasFixture {
                    selector: "main",
                    path: "html > body > main",
                    tag_name: "main",
                    text_char_count: 1_200,
                    heading_count: 4,
                    link_count: 2,
                    paragraph_count: 5,
                    primary_heading_level: None,
                    primary_heading_count: 0,
                    primary_heading_depth: None,
                    utility_descendant_count: 0,
                    score: 0,
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
                    score: 0,
                }),
            ],
            vec![0, 0],
        ),
        (
            CandidatePreference::Reading,
            vec![
                ranked_bias_candidate(BiasFixture {
                    selector: "main",
                    path: "html > body > main",
                    tag_name: "main",
                    text_char_count: 1_000,
                    heading_count: 3,
                    link_count: 10,
                    paragraph_count: 2,
                    primary_heading_level: Some(1),
                    primary_heading_count: 1,
                    primary_heading_depth: Some(1),
                    utility_descendant_count: 0,
                    score: 0,
                }),
                ranked_bias_candidate(BiasFixture {
                    selector: "main > article",
                    path: "html > body > main > article",
                    tag_name: "article",
                    text_char_count: 650,
                    heading_count: 2,
                    link_count: 2,
                    paragraph_count: 2,
                    primary_heading_level: None,
                    primary_heading_count: 0,
                    primary_heading_depth: None,
                    utility_descendant_count: 0,
                    score: 0,
                }),
            ],
            vec![0, 0],
        ),
        (
            CandidatePreference::Reading,
            vec![
                ranked_bias_candidate(BiasFixture {
                    selector: "main",
                    path: "html > body > main",
                    tag_name: "main",
                    text_char_count: 1_000,
                    heading_count: 3,
                    link_count: 10,
                    paragraph_count: 0,
                    primary_heading_level: Some(1),
                    primary_heading_count: 1,
                    primary_heading_depth: Some(1),
                    utility_descendant_count: 0,
                    score: 0,
                }),
                ranked_bias_candidate(BiasFixture {
                    selector: "main > article",
                    path: "html > body > main > article",
                    tag_name: "article",
                    text_char_count: 900,
                    heading_count: 2,
                    link_count: 2,
                    paragraph_count: 2,
                    primary_heading_level: None,
                    primary_heading_count: 0,
                    primary_heading_depth: None,
                    utility_descendant_count: 0,
                    score: 0,
                }),
            ],
            vec![90, -60],
        ),
        (
            CandidatePreference::Reading,
            vec![
                ranked_bias_candidate(BiasFixture {
                    selector: "main",
                    path: "html > body > main",
                    tag_name: "main",
                    text_char_count: 1_000,
                    heading_count: 3,
                    link_count: 6,
                    paragraph_count: 2,
                    primary_heading_level: Some(1),
                    primary_heading_count: 1,
                    primary_heading_depth: Some(1),
                    utility_descendant_count: 0,
                    score: 0,
                }),
                ranked_bias_candidate(BiasFixture {
                    selector: "main > article",
                    path: "html > body > main > article",
                    tag_name: "article",
                    text_char_count: 900,
                    heading_count: 2,
                    link_count: 2,
                    paragraph_count: 2,
                    primary_heading_level: Some(1),
                    primary_heading_count: 1,
                    primary_heading_depth: Some(2),
                    utility_descendant_count: 0,
                    score: 0,
                }),
            ],
            vec![145, -95],
        ),
    ];

    for (preference, mut candidates, expected_scores) in false_edge_cases.drain(..) {
        let original_paths = candidates
            .iter()
            .map(|candidate| candidate.inspection.path.clone())
            .collect::<Vec<_>>();
        apply_nested_content_candidate_bias_for(&mut candidates, preference);
        assert_eq!(
            candidates
                .iter()
                .map(|candidate| candidate.inspection.path.clone())
                .collect::<Vec<_>>(),
            original_paths
        );
        assert_eq!(candidates.len(), expected_scores.len());
        assert_eq!(
            candidates
                .iter()
                .map(|candidate| candidate.score)
                .collect::<Vec<_>>()
                .len(),
            expected_scores.len()
        );
    }
}
