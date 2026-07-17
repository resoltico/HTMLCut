use super::support::*;
use super::*;

#[test]
fn extraction_nested_bias_guard_branches_cover_false_link_and_title_thresholds() {
    let mut link_gap_guard = vec![
        ranked_bias_candidate(BiasFixture {
            selector: "#content",
            path: "html > body > main#content",
            tag_name: "main",
            text_char_count: 1_000,
            heading_count: 20,
            link_count: 29,
            paragraph_count: 5,
            primary_heading_level: None,
            primary_heading_count: 0,
            primary_heading_depth: None,
            utility_descendant_count: 13,
            score: 100,
        }),
        ranked_bias_candidate(BiasFixture {
            selector: ".story",
            path: "html > body > main#content > article.story",
            tag_name: "article",
            text_char_count: 890,
            heading_count: 8,
            link_count: 6,
            paragraph_count: 1,
            primary_heading_level: None,
            primary_heading_count: 0,
            primary_heading_depth: None,
            utility_descendant_count: 0,
            score: 80,
        }),
    ];
    apply_nested_content_candidate_bias_for(&mut link_gap_guard, CandidatePreference::Extraction);
    assert_eq!(link_gap_guard[0].score, 100);
    assert_eq!(link_gap_guard[1].score, 80);

    let mut equal_heading_guard = vec![
        ranked_bias_candidate(BiasFixture {
            selector: "#content",
            path: "html > body > main#content",
            tag_name: "main",
            text_char_count: 1_000,
            heading_count: 4,
            link_count: 40,
            paragraph_count: 10,
            primary_heading_level: None,
            primary_heading_count: 0,
            primary_heading_depth: None,
            utility_descendant_count: 13,
            score: 100,
        }),
        ranked_bias_candidate(BiasFixture {
            selector: "article.story",
            path: "html > body > main#content > article.story",
            tag_name: "article",
            text_char_count: 990,
            heading_count: 5,
            link_count: 10,
            paragraph_count: 1,
            primary_heading_level: None,
            primary_heading_count: 0,
            primary_heading_depth: None,
            utility_descendant_count: 0,
            score: 80,
        }),
    ];
    apply_nested_content_candidate_bias_for(
        &mut equal_heading_guard,
        CandidatePreference::Extraction,
    );
    assert_eq!(equal_heading_guard[0].score, 100);
    assert_eq!(equal_heading_guard[1].score, 80);

    let mut dropped_title_guard = vec![
        ranked_bias_candidate(BiasFixture {
            selector: "#content",
            path: "html > body > main#content",
            tag_name: "main",
            text_char_count: 1_000,
            heading_count: 10,
            link_count: 30,
            paragraph_count: 0,
            primary_heading_level: Some(1),
            primary_heading_count: 1,
            primary_heading_depth: Some(1),
            utility_descendant_count: 13,
            score: 100,
        }),
        ranked_bias_candidate(BiasFixture {
            selector: ".story",
            path: "html > body > main#content > article.story",
            tag_name: "article",
            text_char_count: 960,
            heading_count: 7,
            link_count: 5,
            paragraph_count: 1,
            primary_heading_level: None,
            primary_heading_count: 0,
            primary_heading_depth: None,
            utility_descendant_count: 0,
            score: 80,
        }),
    ];
    apply_nested_content_candidate_bias_for(
        &mut dropped_title_guard,
        CandidatePreference::Extraction,
    );
    assert_eq!(dropped_title_guard[0].score, 100);
    assert_eq!(dropped_title_guard[1].score, 80);

    let mut heading_gap_guard = vec![
        ranked_bias_candidate(BiasFixture {
            selector: "#content",
            path: "html > body > main#content",
            tag_name: "main",
            text_char_count: 1_000,
            heading_count: 10,
            link_count: 30,
            paragraph_count: 10,
            primary_heading_level: None,
            primary_heading_count: 0,
            primary_heading_depth: None,
            utility_descendant_count: 13,
            score: 100,
        }),
        ranked_bias_candidate(BiasFixture {
            selector: ".story",
            path: "html > body > main#content > article.story",
            tag_name: "article",
            text_char_count: 960,
            heading_count: 5,
            link_count: 5,
            paragraph_count: 1,
            primary_heading_level: None,
            primary_heading_count: 0,
            primary_heading_depth: None,
            utility_descendant_count: 0,
            score: 80,
        }),
    ];
    apply_nested_content_candidate_bias_for(
        &mut heading_gap_guard,
        CandidatePreference::Extraction,
    );
    assert_eq!(heading_gap_guard[0].score, 100);
    assert_eq!(heading_gap_guard[1].score, 80);
}

#[test]
fn promotion_filter_guards_cover_remaining_false_threshold_paths() {
    let mut precise_text_threshold = vec![ranked_content_candidate(PromotionFixture {
        selector: "main.layout",
        path: "html > body > main.layout",
        tag_name: "main",
        text_char_count: 1_000,
        heading_count: 3,
        link_count: 100,
        primary_heading_level: None,
        primary_heading_depth: None,
    })];
    let precise_text_reading = vec![
        ranked_content_candidate(PromotionFixture {
            selector: "#other",
            path: "html > body > aside#other",
            tag_name: "aside",
            text_char_count: 200,
            heading_count: 1,
            link_count: 1,
            primary_heading_level: None,
            primary_heading_depth: None,
        }),
        ranked_content_candidate(PromotionFixture {
            selector: "article.story",
            path: "html > body > main.layout > article.story",
            tag_name: "article",
            text_char_count: 899,
            heading_count: 3,
            link_count: 60,
            primary_heading_level: None,
            primary_heading_depth: None,
        }),
    ];
    promote_precise_reading_descendant_candidate(
        &mut precise_text_threshold,
        &precise_text_reading,
    );
    assert_eq!(precise_text_threshold[0].inspection.selector, "main.layout");

    let mut precise_link_gap = vec![ranked_content_candidate(PromotionFixture {
        selector: "main.layout",
        path: "html > body > main.layout",
        tag_name: "main",
        text_char_count: 1_000,
        heading_count: 3,
        link_count: 79,
        primary_heading_level: None,
        primary_heading_depth: None,
    })];
    let precise_link_reading = vec![
        ranked_content_candidate(PromotionFixture {
            selector: "#other",
            path: "html > body > aside#other",
            tag_name: "aside",
            text_char_count: 200,
            heading_count: 1,
            link_count: 1,
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
    promote_precise_reading_descendant_candidate(&mut precise_link_gap, &precise_link_reading);
    assert_eq!(precise_link_gap[0].inspection.selector, "main.layout");

    let mut cleaner_heading_gap = vec![ranked_content_candidate(PromotionFixture {
        selector: "#content",
        path: "html > body > main#content",
        tag_name: "main",
        text_char_count: 1_000,
        heading_count: 4,
        link_count: 80,
        primary_heading_level: Some(1),
        primary_heading_depth: Some(1),
    })];
    let cleaner_heading_reading = vec![
        cleaner_heading_gap[0].clone(),
        ranked_content_candidate(PromotionFixture {
            selector: ".story-fragment",
            path: "html > body > main#content > article.story-fragment",
            tag_name: "article",
            text_char_count: 930,
            heading_count: 1,
            link_count: 40,
            primary_heading_level: Some(1),
            primary_heading_depth: Some(2),
        }),
    ];
    promote_cleaner_reading_descendant_candidate(
        &mut cleaner_heading_gap,
        &cleaner_heading_reading,
    );
    assert_eq!(cleaner_heading_gap[0].inspection.selector, "#content");
}

#[test]
fn heading_shell_bias_helper_requires_extreme_heading_noise() {
    assert!(outer_wrapper_adds_heading_shell(
        HeadingShellCandidate {
            text_char_count: 23_197,
            heading_count: 215,
            link_count: 174,
            selector: "#repo-content-pjax-container",
        },
        HeadingShellCandidate {
            text_char_count: 23_026,
            heading_count: 53,
            link_count: 172,
            selector: "#wiki-body",
        },
    ));
    assert!(!outer_wrapper_adds_heading_shell(
        HeadingShellCandidate {
            text_char_count: 1_000,
            heading_count: 10,
            link_count: 8,
            selector: "#content",
        },
        HeadingShellCandidate {
            text_char_count: 930,
            heading_count: 5,
            link_count: 6,
            selector: "article.story",
        },
    ));
    assert!(!outer_wrapper_adds_heading_shell(
        HeadingShellCandidate {
            text_char_count: 1_000,
            heading_count: 64,
            link_count: 8,
            selector: "#content",
        },
        HeadingShellCandidate {
            text_char_count: 950,
            heading_count: 40,
            link_count: 6,
            selector: "article.story",
        },
    ));
    assert!(!outer_wrapper_adds_heading_shell(
        HeadingShellCandidate {
            text_char_count: 1_000,
            heading_count: 80,
            link_count: 24,
            selector: "#content",
        },
        HeadingShellCandidate {
            text_char_count: 950,
            heading_count: 40,
            link_count: 6,
            selector: "article.story",
        },
    ));
}

#[test]
fn nested_bias_prefers_inner_candidate_when_outer_wrapper_only_adds_heading_shell() {
    let extraction_outer = ranked_bias_candidate(BiasFixture {
        selector: "#repo-content-pjax-container",
        path: "html > body > main#repo-content-pjax-container",
        tag_name: "main",
        text_char_count: 23_197,
        heading_count: 215,
        link_count: 174,
        paragraph_count: 7,
        primary_heading_level: Some(1),
        primary_heading_count: 1,
        primary_heading_depth: Some(1),
        utility_descendant_count: 0,
        score: 120,
    });
    let extraction_inner = ranked_bias_candidate(BiasFixture {
        selector: "#wiki-body",
        path: "html > body > main#repo-content-pjax-container > div#wiki-body",
        tag_name: "div",
        text_char_count: 23_026,
        heading_count: 53,
        link_count: 172,
        paragraph_count: 7,
        primary_heading_level: None,
        primary_heading_count: 0,
        primary_heading_depth: None,
        utility_descendant_count: 0,
        score: 80,
    });
    let mut extraction_candidates = vec![extraction_outer.clone(), extraction_inner.clone()];
    apply_nested_content_candidate_bias_for(
        &mut extraction_candidates,
        CandidatePreference::Extraction,
    );
    assert!(extraction_candidates[1].score > extraction_candidates[0].score);

    let mut reading_candidates = vec![extraction_outer, extraction_inner];
    apply_nested_content_candidate_bias_for(&mut reading_candidates, CandidatePreference::Reading);
    assert!(reading_candidates[1].score > reading_candidates[0].score);
}

#[test]
fn precise_descendant_promotion_uses_heading_shell_signal_when_link_gap_is_small() {
    let mut extraction_candidates = vec![ranked_content_candidate(PromotionFixture {
        selector: "#js-repo-pjax-container",
        path: "html > body > main#js-repo-pjax-container",
        tag_name: "main",
        text_char_count: 23_197,
        heading_count: 215,
        link_count: 174,
        primary_heading_level: Some(1),
        primary_heading_depth: Some(1),
    })];
    let reading_candidates = vec![
        ranked_content_candidate(PromotionFixture {
            selector: "#wiki-body",
            path: "html > body > main#js-repo-pjax-container > div#repo-content-pjax-container > div#wiki-wrapper > div#wiki-content > div#wiki-body",
            tag_name: "div",
            text_char_count: 23_026,
            heading_count: 53,
            link_count: 172,
            primary_heading_level: None,
            primary_heading_depth: None,
        }),
        extraction_candidates[0].clone(),
    ];

    promote_precise_reading_descendant_candidate(&mut extraction_candidates, &reading_candidates);

    assert_eq!(extraction_candidates[0].inspection.selector, "#wiki-body");
}

#[test]
fn precise_descendant_promotion_uses_heading_shell_signal_when_reading_top_is_current() {
    let extraction = ranked_content_candidate(PromotionFixture {
        selector: "#repo-content-pjax-container",
        path: "html > body > main#repo-content-pjax-container",
        tag_name: "main",
        text_char_count: 23_049,
        heading_count: 214,
        link_count: 172,
        primary_heading_level: Some(1),
        primary_heading_depth: Some(1),
    });
    let mut extraction_candidates = vec![extraction.clone()];
    let reading_candidates = vec![
        extraction,
        ranked_content_candidate(PromotionFixture {
            selector: "#wiki-body",
            path: "html > body > main#repo-content-pjax-container > div#wiki-wrapper > div#wiki-content > div#wiki-body",
            tag_name: "div",
            text_char_count: 23_026,
            heading_count: 53,
            link_count: 172,
            primary_heading_level: None,
            primary_heading_depth: None,
        }),
    ];

    promote_precise_reading_descendant_candidate(&mut extraction_candidates, &reading_candidates);

    assert_eq!(extraction_candidates[0].inspection.selector, "#wiki-body");
}

#[test]
fn precise_descendant_promotion_rejects_unstable_heading_shell_descendants() {
    let extraction = ranked_content_candidate(PromotionFixture {
        selector: "#repo-content-pjax-container",
        path: "html > body > main#repo-content-pjax-container",
        tag_name: "main",
        text_char_count: 23_049,
        heading_count: 214,
        link_count: 172,
        primary_heading_level: Some(1),
        primary_heading_depth: Some(1),
    });
    let mut extraction_candidates = vec![extraction.clone()];
    let reading_candidates = vec![
        extraction,
        ranked_content_candidate(PromotionFixture {
            selector: "div.markdown-body:nth-of-type(2)",
            path: "html > body > main#repo-content-pjax-container > div#wiki-wrapper > div#wiki-content > div#wiki-body > div.markdown-body:nth-of-type(2)",
            tag_name: "div",
            text_char_count: 23_026,
            heading_count: 53,
            link_count: 172,
            primary_heading_level: None,
            primary_heading_depth: None,
        }),
    ];

    promote_precise_reading_descendant_candidate(&mut extraction_candidates, &reading_candidates);

    assert_eq!(
        extraction_candidates[0].inspection.selector,
        "#repo-content-pjax-container"
    );
}

#[test]
fn precise_descendant_promotion_rejects_descendants_with_extra_links_without_shell_signal() {
    let extraction = ranked_content_candidate(PromotionFixture {
        selector: "#content",
        path: "html > body > main#content",
        tag_name: "main",
        text_char_count: 1_000,
        heading_count: 10,
        link_count: 10,
        primary_heading_level: Some(1),
        primary_heading_depth: Some(1),
    });
    let mut extraction_candidates = vec![extraction.clone()];
    let reading_candidates = vec![
        extraction,
        ranked_content_candidate(PromotionFixture {
            selector: "article.story",
            path: "html > body > main#content > article.story",
            tag_name: "article",
            text_char_count: 960,
            heading_count: 8,
            link_count: 11,
            primary_heading_level: Some(2),
            primary_heading_depth: Some(2),
        }),
    ];

    promote_precise_reading_descendant_candidate(&mut extraction_candidates, &reading_candidates);

    assert_eq!(extraction_candidates[0].inspection.selector, "#content");
}
