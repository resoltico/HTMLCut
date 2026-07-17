use super::support::*;
use super::*;

#[test]
fn title_bearing_reading_ancestor_promotion_restores_near_full_wrappers_that_keep_the_title() {
    let mut extraction_candidates = vec![ranked_content_candidate(PromotionFixture {
        selector: "article.article-body",
        path: "html > body > main#content > article.article-body",
        tag_name: "article",
        text_char_count: 950,
        heading_count: 3,
        link_count: 12,
        primary_heading_level: Some(2),
        primary_heading_depth: Some(3),
    })];
    let reading_candidates = vec![
        ranked_content_candidate(PromotionFixture {
            selector: "#content",
            path: "html > body > main#content",
            tag_name: "main",
            text_char_count: 980,
            heading_count: 4,
            link_count: 22,
            primary_heading_level: Some(1),
            primary_heading_depth: Some(4),
        }),
        extraction_candidates[0].clone(),
    ];

    promote_title_bearing_reading_ancestor_candidate(
        &mut extraction_candidates,
        &reading_candidates,
    );

    assert_eq!(extraction_candidates[0].inspection.selector, "#content");
}

#[test]
fn selector_rank_and_nested_bias_helpers_cover_new_extraction_branches() {
    assert_eq!(selector_stability_rank("article:nth-of-type(2)"), 0);
    assert_eq!(selector_stability_rank("#main"), 5);
    assert_eq!(selector_stability_rank("[role=\"main\"]"), 4);
    assert_eq!(selector_stability_rank("article.card"), 3);
    assert_eq!(selector_stability_rank(".card"), 2);
    assert_eq!(selector_stability_rank("article"), 1);

    let mut heading_and_link_heavy_outer = vec![
        ranked_bias_candidate(BiasFixture {
            selector: "main.wrapper",
            path: "html > body > main.wrapper",
            tag_name: "main",
            text_char_count: 1_000,
            heading_count: 20,
            link_count: 40,
            paragraph_count: 3,
            primary_heading_level: None,
            primary_heading_count: 0,
            primary_heading_depth: None,
            utility_descendant_count: 0,
            score: 0,
        }),
        ranked_bias_candidate(BiasFixture {
            selector: "main.wrapper > article.story",
            path: "html > body > main.wrapper > article.story",
            tag_name: "article",
            text_char_count: 900,
            heading_count: 4,
            link_count: 12,
            paragraph_count: 3,
            primary_heading_level: None,
            primary_heading_count: 0,
            primary_heading_depth: None,
            utility_descendant_count: 0,
            score: 0,
        }),
    ];
    apply_nested_content_candidate_bias_for(
        &mut heading_and_link_heavy_outer,
        CandidatePreference::Extraction,
    );
    assert!(heading_and_link_heavy_outer[1].score > heading_and_link_heavy_outer[0].score);

    let mut stable_inner_with_fewer_links = vec![
        ranked_bias_candidate(BiasFixture {
            selector: "#content",
            path: "html > body > main#content",
            tag_name: "main",
            text_char_count: 1_000,
            heading_count: 3,
            link_count: 36,
            paragraph_count: 3,
            primary_heading_level: Some(1),
            primary_heading_count: 1,
            primary_heading_depth: Some(1),
            utility_descendant_count: 0,
            score: 0,
        }),
        ranked_bias_candidate(BiasFixture {
            selector: "article.story",
            path: "html > body > main#content > article.story",
            tag_name: "article",
            text_char_count: 970,
            heading_count: 4,
            link_count: 10,
            paragraph_count: 3,
            primary_heading_level: Some(1),
            primary_heading_count: 1,
            primary_heading_depth: Some(2),
            utility_descendant_count: 0,
            score: 0,
        }),
    ];
    apply_nested_content_candidate_bias_for(
        &mut stable_inner_with_fewer_links,
        CandidatePreference::Extraction,
    );
    assert!(stable_inner_with_fewer_links[1].score > stable_inner_with_fewer_links[0].score);

    let mut near_equal_inner = vec![
        ranked_bias_candidate(BiasFixture {
            selector: "main.wrapper",
            path: "html > body > main.wrapper",
            tag_name: "main",
            text_char_count: 1_000,
            heading_count: 5,
            link_count: 30,
            paragraph_count: 3,
            primary_heading_level: None,
            primary_heading_count: 0,
            primary_heading_depth: None,
            utility_descendant_count: 0,
            score: 0,
        }),
        ranked_bias_candidate(BiasFixture {
            selector: "article.story",
            path: "html > body > main.wrapper > article.story",
            tag_name: "article",
            text_char_count: 980,
            heading_count: 5,
            link_count: 10,
            paragraph_count: 3,
            primary_heading_level: None,
            primary_heading_count: 0,
            primary_heading_depth: None,
            utility_descendant_count: 0,
            score: 0,
        }),
    ];
    apply_nested_content_candidate_bias_for(&mut near_equal_inner, CandidatePreference::Extraction);
    assert!(near_equal_inner[1].score > near_equal_inner[0].score);

    let mut stable_selector_inner = vec![
        ranked_bias_candidate(BiasFixture {
            selector: "#content",
            path: "html > body > main#content",
            tag_name: "main",
            text_char_count: 1_000,
            heading_count: 4,
            link_count: 30,
            paragraph_count: 3,
            primary_heading_level: Some(2),
            primary_heading_count: 1,
            primary_heading_depth: Some(1),
            utility_descendant_count: 0,
            score: 0,
        }),
        ranked_bias_candidate(BiasFixture {
            selector: "article.story",
            path: "html > body > main#content > article.story",
            tag_name: "article",
            text_char_count: 950,
            heading_count: 5,
            link_count: 10,
            paragraph_count: 3,
            primary_heading_level: Some(2),
            primary_heading_count: 1,
            primary_heading_depth: Some(2),
            utility_descendant_count: 0,
            score: 0,
        }),
    ];
    apply_nested_content_candidate_bias_for(
        &mut stable_selector_inner,
        CandidatePreference::Extraction,
    );
    assert!(stable_selector_inner[1].score > stable_selector_inner[0].score);

    let mut reading_preference_link_shell = vec![
        ranked_bias_candidate(BiasFixture {
            selector: "#content",
            path: "html > body > main#content",
            tag_name: "main",
            text_char_count: 1_000,
            heading_count: 3,
            link_count: 200,
            paragraph_count: 3,
            primary_heading_level: None,
            primary_heading_count: 0,
            primary_heading_depth: None,
            utility_descendant_count: 0,
            score: 0,
        }),
        ranked_bias_candidate(BiasFixture {
            selector: "article.story",
            path: "html > body > main#content > article.story",
            tag_name: "article",
            text_char_count: 980,
            heading_count: 3,
            link_count: 20,
            paragraph_count: 3,
            primary_heading_level: None,
            primary_heading_count: 0,
            primary_heading_depth: None,
            utility_descendant_count: 0,
            score: 0,
        }),
    ];
    apply_nested_content_candidate_bias_for(
        &mut reading_preference_link_shell,
        CandidatePreference::Reading,
    );
    assert!(reading_preference_link_shell[1].score > reading_preference_link_shell[0].score);

    let mut extraction_preference_link_shell = vec![
        ranked_bias_candidate(BiasFixture {
            selector: "#content",
            path: "html > body > main#content",
            tag_name: "main",
            text_char_count: 1_000,
            heading_count: 3,
            link_count: 200,
            paragraph_count: 3,
            primary_heading_level: None,
            primary_heading_count: 0,
            primary_heading_depth: None,
            utility_descendant_count: 0,
            score: 0,
        }),
        ranked_bias_candidate(BiasFixture {
            selector: "article.story",
            path: "html > body > main#content > article.story",
            tag_name: "article",
            text_char_count: 980,
            heading_count: 3,
            link_count: 20,
            paragraph_count: 3,
            primary_heading_level: None,
            primary_heading_count: 0,
            primary_heading_depth: None,
            utility_descendant_count: 0,
            score: 0,
        }),
    ];
    apply_nested_content_candidate_bias_for(
        &mut extraction_preference_link_shell,
        CandidatePreference::Extraction,
    );
    assert!(extraction_preference_link_shell[1].score > extraction_preference_link_shell[0].score);
}
