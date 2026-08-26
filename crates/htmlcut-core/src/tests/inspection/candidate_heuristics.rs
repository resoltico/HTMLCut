use crate::inspect::{
    ContentCandidateTestInput, ContentCandidateTestPreference,
    content_candidate_has_narrative_section_shape_for_tests,
    content_candidate_has_readable_density_for_tests,
    content_candidate_is_excluded_for_utility_chrome_for_tests, content_candidate_scores_for_tests,
    extraction_candidate_score_for_tests,
    extraction_prefers_heading_and_link_light_descendant_for_tests,
    extraction_prefers_near_complete_link_light_descendant_for_tests,
    extraction_prefers_stable_link_light_descendant_for_tests,
    extraction_prefers_utility_light_descendant_for_tests,
    extraction_preserves_large_outer_candidate_for_tests,
    nested_content_candidate_bias_deltas_for_tests, prefers_heavy_link_descendant_for_tests,
};
use crate::tests::memory_source_with_base;
use crate::{InspectionOptions, RuntimeOptions, inspect_source};

fn candidate_input() -> ContentCandidateTestInput {
    ContentCandidateTestInput::default()
}

fn nested_candidates() -> (ContentCandidateTestInput, ContentCandidateTestInput) {
    (
        ContentCandidateTestInput {
            selector: "#outer",
            path: "html > body > main#outer",
            text_char_count: 1_000,
            ..candidate_input()
        },
        ContentCandidateTestInput {
            selector: "#inner",
            path: "html > body > main#outer > article#inner",
            tag_name: "article",
            text_char_count: 900,
            ..candidate_input()
        },
    )
}

#[test]
fn candidate_readability_thresholds_reject_navigation_and_accept_prose() {
    assert!(!content_candidate_has_readable_density_for_tests(
        "article", 19, 0, 0, 0, 0,
    ));
    assert!(content_candidate_has_readable_density_for_tests(
        "article", 20, 0, 0, 0, 0,
    ));
    assert!(!content_candidate_has_readable_density_for_tests(
        "div", 119, 0, 0, 0, 0,
    ));
    assert!(content_candidate_has_readable_density_for_tests(
        "div", 120, 0, 0, 0, 0,
    ));
    assert!(!content_candidate_has_readable_density_for_tests(
        "section", 216, 10, 0, 2, 0,
    ));
    assert!(content_candidate_has_readable_density_for_tests(
        "section", 216, 9, 0, 2, 0,
    ));
    assert!(content_candidate_has_readable_density_for_tests(
        "section", 216, 0, 12, 2, 0,
    ));
    assert!(!content_candidate_has_readable_density_for_tests(
        "section", 216, 0, 13, 2, 0,
    ));
    assert!(content_candidate_has_readable_density_for_tests(
        "section", 3_999, 12, 0, 3, 3,
    ));
    assert!(!content_candidate_has_readable_density_for_tests(
        "section", 3_999, 13, 0, 3, 3,
    ));
    assert!(content_candidate_has_readable_density_for_tests(
        "section", 4_000, 13, 0, 3, 3,
    ));
    assert!(content_candidate_has_readable_density_for_tests(
        "section", 3_999, 0, 18, 3, 3,
    ));
    assert!(!content_candidate_has_readable_density_for_tests(
        "section", 3_999, 0, 19, 3, 3,
    ));
}

#[test]
fn narrative_section_shape_requires_distinct_prose_signals() {
    assert!(content_candidate_has_narrative_section_shape_for_tests(
        "<section id=\"scope\"><p>one</p><p>two</p><p>three</p></section>",
    ));
    assert!(content_candidate_has_narrative_section_shape_for_tests(
        "<section id=\"scope\"><h2>Title</h2><p>one</p><p>two</p></section>",
    ));
    assert!(content_candidate_has_narrative_section_shape_for_tests(
        "<section id=\"scope\"><ul><li>one</li><li>two</li></ul><p>one</p><p>two</p></section>",
    ));
    assert!(!content_candidate_has_narrative_section_shape_for_tests(
        "<section id=\"scope\"><p>one</p><p>two</p></section>",
    ));
}

#[test]
fn candidate_scores_apply_declared_weighting_and_caps() {
    let baseline = candidate_input();
    let baseline_scores = content_candidate_scores_for_tests(&baseline);

    for (tag_name, extraction_delta, reading_delta) in [
        ("article", 102, 75),
        ("main", 52, 85),
        ("section", 10, 15),
        ("div", 0, 0),
    ] {
        let scores = content_candidate_scores_for_tests(&ContentCandidateTestInput {
            tag_name,
            ..candidate_input()
        });
        assert_eq!(scores.0 - baseline_scores.0, extraction_delta, "{tag_name}");
        assert_eq!(scores.1 - baseline_scores.1, reading_delta, "{tag_name}");
    }

    let weighted = ContentCandidateTestInput {
        tag_name: "article",
        text_char_count: 7_980,
        heading_count: 8,
        paragraph_count: 16,
        positive_signal_count: 4,
        negative_signal_count: 4,
        utility_descendant_count: 12,
        has_main_role: true,
        has_article_body_itemprop: true,
        uses_exact_path_selector: true,
        primary_heading_level: Some(1),
        primary_heading_count: 2,
        primary_heading_depth: Some(1),
        ..candidate_input()
    };
    assert_eq!(content_candidate_scores_for_tests(&weighted), (-103, 201));
}

#[test]
fn candidate_scores_apply_preference_specific_short_content_penalties() {
    let no_body = ContentCandidateTestInput {
        tag_name: "article",
        text_char_count: 499,
        paragraph_count: 0,
        primary_heading_level: Some(1),
        primary_heading_count: 1,
        primary_heading_depth: Some(1),
        ..candidate_input()
    };
    let one_body = ContentCandidateTestInput {
        text_char_count: 319,
        paragraph_count: 1,
        ..no_body.clone()
    };
    let link_dense = ContentCandidateTestInput {
        text_char_count: 239,
        link_count: 9,
        ..one_body.clone()
    };

    assert_eq!(content_candidate_scores_for_tests(&no_body), (-166, 8));
    assert_eq!(content_candidate_scores_for_tests(&one_body), (-55, 118));
    assert_eq!(content_candidate_scores_for_tests(&link_dense), (-86, 92));
}

#[test]
fn candidate_score_penalties_change_only_at_declared_boundaries() {
    let base = ContentCandidateTestInput {
        tag_name: "article",
        paragraph_count: 0,
        text_char_count: 499,
        ..candidate_input()
    };
    let at_boundary = ContentCandidateTestInput {
        text_char_count: 500,
        ..base.clone()
    };
    let before = content_candidate_scores_for_tests(&base);
    let after = content_candidate_scores_for_tests(&at_boundary);
    assert_eq!((after.0 - before.0, after.1 - before.1), (200, 180));

    let extraction_short = ContentCandidateTestInput {
        paragraph_count: 1,
        text_char_count: 419,
        ..base.clone()
    };
    let extraction_ready = ContentCandidateTestInput {
        text_char_count: 420,
        ..extraction_short.clone()
    };
    let before = content_candidate_scores_for_tests(&extraction_short);
    let after = content_candidate_scores_for_tests(&extraction_ready);
    assert_eq!((after.0 - before.0, after.1 - before.1), (96, 0));

    let reading_short = ContentCandidateTestInput {
        paragraph_count: 1,
        text_char_count: 319,
        ..base.clone()
    };
    let reading_ready = ContentCandidateTestInput {
        text_char_count: 320,
        ..reading_short.clone()
    };
    let before = content_candidate_scores_for_tests(&reading_short);
    let after = content_candidate_scores_for_tests(&reading_ready);
    assert_eq!((after.0 - before.0, after.1 - before.1), (0, 75));

    let title_fragment = ContentCandidateTestInput {
        tag_name: "div",
        paragraph_count: 0,
        text_char_count: 299,
        primary_heading_level: Some(1),
        primary_heading_count: 1,
        primary_heading_depth: Some(1),
        ..candidate_input()
    };
    let title_with_body = ContentCandidateTestInput {
        text_char_count: 300,
        ..title_fragment.clone()
    };
    let before = content_candidate_scores_for_tests(&title_fragment);
    let after = content_candidate_scores_for_tests(&title_with_body);
    assert_eq!((after.0 - before.0, after.1 - before.1), (0, 170));

    let dense_links = ContentCandidateTestInput {
        tag_name: "article",
        paragraph_count: 2,
        link_count: 9,
        text_char_count: 239,
        ..candidate_input()
    };
    let sufficient_text = ContentCandidateTestInput {
        text_char_count: 240,
        ..dense_links.clone()
    };
    let before = content_candidate_scores_for_tests(&dense_links);
    let after = content_candidate_scores_for_tests(&sufficient_text);
    assert_eq!((after.0 - before.0, after.1 - before.1), (-30, -15));
}

#[test]
fn extraction_ranking_blends_a_bounded_reading_score() {
    let input = ContentCandidateTestInput {
        tag_name: "article",
        text_char_count: 2_700,
        heading_count: 2,
        paragraph_count: 6,
        positive_signal_count: 2,
        primary_heading_level: Some(1),
        primary_heading_count: 1,
        primary_heading_depth: Some(1),
        ..candidate_input()
    };
    let (extraction, reading) = content_candidate_scores_for_tests(&input);
    assert_eq!(
        extraction_candidate_score_for_tests(&input),
        extraction + (reading.max(0) / 3),
    );
}

#[test]
fn inspection_excludes_utility_candidates_without_requiring_a_utility_ancestor() {
    let source = memory_source_with_base(
        "fixture.html",
        "<html><body><main id=\"content\"><article class=\"article-body\"><h1>Actual article</h1><p>Alpha beta gamma delta epsilon zeta eta theta iota kappa lambda mu.</p><p>Nu xi omicron pi rho sigma tau upsilon phi chi psi omega.</p><p>Further context confirms this is the main narrative.</p></article><section class=\"content body footer sidebar related comments social newsletter\"><h2>Related reads</h2><p>Related recommendation one is deliberately long enough to look readable.</p><p>Related recommendation two is deliberately long enough to look readable.</p><p>Related recommendation three is deliberately long enough to look readable.</p></section></main></body></html>",
        "https://example.test/start.html",
    );
    let inspection = inspect_source(
        &source,
        &RuntimeOptions::default(),
        &InspectionOptions {
            include_source_text: false,
            sample_limit: 6,
        },
    );

    assert!(inspection.ok);
    let document = inspection.document.expect("document inspection");
    for candidates in [document.extraction_candidates, document.reading_candidates] {
        assert!(candidates.iter().all(|candidate| candidate.selector
            != "section.content.body.footer.sidebar.related.comments.social.newsletter"));
    }
}

#[test]
fn candidate_utility_exclusion_rejects_self_and_ancestor_chrome() {
    assert!(content_candidate_is_excluded_for_utility_chrome_for_tests(
        "<section id=\"scope\" class=\"content footer sidebar related\"><p>Related content</p></section>",
    ));
    assert!(content_candidate_is_excluded_for_utility_chrome_for_tests(
        "<aside class=\"related\"><section id=\"scope\" class=\"content\"><p>Related content</p></section></aside>",
    ));
    assert!(!content_candidate_is_excluded_for_utility_chrome_for_tests(
        "<main><section id=\"scope\" class=\"content\"><p>Primary content</p></section></main>",
    ));
}

#[test]
fn nested_candidate_bias_preserves_title_wrappers_and_penalizes_chrome() {
    let outer = ContentCandidateTestInput {
        selector: "#outer",
        path: "html > body > main#outer",
        text_char_count: 1_000,
        heading_count: 26,
        link_count: 12,
        paragraph_count: 4,
        primary_heading_level: Some(1),
        primary_heading_count: 1,
        primary_heading_depth: Some(1),
        utility_descendant_count: 8,
        ..candidate_input()
    };
    let inner = ContentCandidateTestInput {
        selector: "#inner",
        path: "html > body > main#outer > article#inner",
        tag_name: "article",
        text_char_count: 980,
        heading_count: 2,
        link_count: 0,
        paragraph_count: 4,
        primary_heading_level: Some(1),
        primary_heading_count: 1,
        primary_heading_depth: Some(1),
        ..candidate_input()
    };

    assert_eq!(
        nested_content_candidate_bias_deltas_for_tests(
            &outer,
            &inner,
            ContentCandidateTestPreference::Reading,
        ),
        (-1_650, 1_900),
    );
    assert_eq!(
        nested_content_candidate_bias_deltas_for_tests(
            &outer,
            &inner,
            ContentCandidateTestPreference::Extraction,
        ),
        (-1_950, 2_250),
    );
}

#[test]
fn nested_candidate_bias_prefers_an_equally_complete_cleaner_descendant() {
    let outer = ContentCandidateTestInput {
        selector: "article.outer",
        path: "html > body > article.outer",
        text_char_count: 1_000,
        heading_count: 3,
        link_count: 25,
        paragraph_count: 4,
        utility_descendant_count: 3,
        ..candidate_input()
    };
    let inner = ContentCandidateTestInput {
        selector: "article.inner",
        path: "html > body > article.outer > article.inner",
        tag_name: "article",
        text_char_count: 960,
        heading_count: 2,
        link_count: 2,
        paragraph_count: 4,
        utility_descendant_count: 0,
        ..candidate_input()
    };

    assert_eq!(
        nested_content_candidate_bias_deltas_for_tests(
            &outer,
            &inner,
            ContentCandidateTestPreference::Extraction,
        ),
        (-2_335, 2_845),
    );
}

#[test]
fn nested_candidate_bias_applies_each_extraction_policy_boundary() {
    let (mut outer, mut inner) = nested_candidates();
    outer.paragraph_count = 1;
    outer.primary_heading_level = Some(1);
    outer.primary_heading_depth = Some(1);
    inner.primary_heading_level = None;
    inner.primary_heading_depth = None;
    inner.text_char_count = 850;
    assert_eq!(
        nested_content_candidate_bias_deltas_for_tests(
            &outer,
            &inner,
            ContentCandidateTestPreference::Extraction,
        ),
        (245, -280),
    );

    let (mut outer, mut inner) = nested_candidates();
    outer.heading_count = 1;
    outer.link_count = 8;
    outer.paragraph_count = 2;
    inner.paragraph_count = 1;
    inner.text_char_count = 920;
    assert_eq!(
        nested_content_candidate_bias_deltas_for_tests(
            &outer,
            &inner,
            ContentCandidateTestPreference::Extraction,
        ),
        (-225, 300),
    );

    let (mut outer, mut inner) = nested_candidates();
    outer.heading_count = 14;
    outer.link_count = 26;
    inner.heading_count = 2;
    inner.link_count = 2;
    inner.text_char_count = 880;
    assert_eq!(
        nested_content_candidate_bias_deltas_for_tests(
            &outer,
            &inner,
            ContentCandidateTestPreference::Extraction,
        ),
        (-680, 850),
    );

    let (mut outer, mut inner) = nested_candidates();
    outer.selector = "#outer";
    outer.heading_count = 0;
    outer.link_count = 20;
    outer.paragraph_count = 2;
    inner.selector = "article.inner";
    inner.paragraph_count = 0;
    inner.text_char_count = 980;
    assert_eq!(
        nested_content_candidate_bias_deltas_for_tests(
            &outer,
            &inner,
            ContentCandidateTestPreference::Extraction,
        ),
        (-320, 410),
    );

    let (mut outer, mut inner) = nested_candidates();
    outer.heading_count = 5;
    outer.link_count = 120;
    inner.text_char_count = 980;
    assert_eq!(
        nested_content_candidate_bias_deltas_for_tests(
            &outer,
            &inner,
            ContentCandidateTestPreference::Extraction,
        ),
        (-900, 1_110),
    );

    let (mut outer, mut inner) = nested_candidates();
    outer.text_char_count = 6_000;
    outer.heading_count = 4;
    outer.paragraph_count = 4;
    inner.text_char_count = 1_000;
    assert_eq!(
        nested_content_candidate_bias_deltas_for_tests(
            &outer,
            &inner,
            ContentCandidateTestPreference::Extraction,
        ),
        (170, -190),
    );

    let (mut outer, mut inner) = nested_candidates();
    outer.heading_count = 2;
    outer.paragraph_count = 3;
    outer.utility_descendant_count = 8;
    inner.paragraph_count = 2;
    inner.text_char_count = 780;
    assert_eq!(
        nested_content_candidate_bias_deltas_for_tests(
            &outer,
            &inner,
            ContentCandidateTestPreference::Extraction,
        ),
        (-170, 235),
    );

    let (mut outer, mut inner) = nested_candidates();
    outer.paragraph_count = 3;
    outer.primary_heading_level = Some(1);
    outer.primary_heading_depth = Some(1);
    inner.primary_heading_level = None;
    inner.primary_heading_depth = None;
    inner.text_char_count = 700;
    assert_eq!(
        nested_content_candidate_bias_deltas_for_tests(
            &outer,
            &inner,
            ContentCandidateTestPreference::Extraction,
        ),
        (35, -30),
    );

    let (mut outer, mut inner) = nested_candidates();
    outer.paragraph_count = 2;
    outer.primary_heading_count = 2;
    inner.primary_heading_count = 1;
    inner.text_char_count = 800;
    assert_eq!(
        nested_content_candidate_bias_deltas_for_tests(
            &outer,
            &inner,
            ContentCandidateTestPreference::Extraction,
        ),
        (-25, 40),
    );

    let (mut outer, mut inner) = nested_candidates();
    outer.heading_count = 4;
    outer.paragraph_count = 2;
    inner.text_char_count = 800;
    assert_eq!(
        nested_content_candidate_bias_deltas_for_tests(
            &outer,
            &inner,
            ContentCandidateTestPreference::Extraction,
        ),
        (-20, 35),
    );

    let (mut outer, mut inner) = nested_candidates();
    outer.heading_count = 2;
    outer.paragraph_count = 1;
    inner.text_char_count = 900;
    assert_eq!(
        nested_content_candidate_bias_deltas_for_tests(
            &outer,
            &inner,
            ContentCandidateTestPreference::Extraction,
        ),
        (-40, 60),
    );
}

#[test]
fn nested_candidate_bias_uses_strict_fallback_boundaries() {
    let (outer, mut inner) = nested_candidates();
    inner.text_char_count = 679;
    assert_eq!(
        nested_content_candidate_bias_deltas_for_tests(
            &outer,
            &inner,
            ContentCandidateTestPreference::Extraction,
        ),
        (0, 0),
    );
    inner.text_char_count = 680;
    assert_eq!(
        nested_content_candidate_bias_deltas_for_tests(
            &outer,
            &inner,
            ContentCandidateTestPreference::Extraction,
        ),
        (-60, 90),
    );

    let (mut outer, mut inner) = nested_candidates();
    outer.link_count = 10;
    inner.link_count = 9;
    inner.text_char_count = 1_000;
    assert_eq!(
        nested_content_candidate_bias_deltas_for_tests(
            &outer,
            &inner,
            ContentCandidateTestPreference::Extraction,
        ),
        (0, 0),
    );
    inner.link_count = 8;
    assert_eq!(
        nested_content_candidate_bias_deltas_for_tests(
            &outer,
            &inner,
            ContentCandidateTestPreference::Extraction,
        ),
        (-60, 90),
    );

    let (mut outer, inner) = nested_candidates();
    outer.paragraph_count = 2;
    outer.utility_descendant_count = 13;
    assert_eq!(
        nested_content_candidate_bias_deltas_for_tests(
            &outer,
            &inner,
            ContentCandidateTestPreference::Extraction,
        ),
        (0, 0),
    );
    outer.utility_descendant_count = 12;
    assert_eq!(
        nested_content_candidate_bias_deltas_for_tests(
            &outer,
            &inner,
            ContentCandidateTestPreference::Extraction,
        ),
        (-60, 90),
    );
}

#[test]
fn nested_candidate_bias_preserves_nonzero_threshold_arithmetic() {
    let (mut outer, mut inner) = nested_candidates();
    outer.heading_count = 5;
    outer.link_count = 13;
    outer.paragraph_count = 4;
    outer.utility_descendant_count = 5;
    inner.heading_count = 3;
    inner.link_count = 5;
    inner.paragraph_count = 3;
    inner.utility_descendant_count = 3;
    inner.text_char_count = 920;
    assert_eq!(
        nested_content_candidate_bias_deltas_for_tests(
            &outer,
            &inner,
            ContentCandidateTestPreference::Extraction,
        ),
        (-205, 270),
    );

    let (mut outer, mut inner) = nested_candidates();
    outer.heading_count = 16;
    outer.link_count = 144;
    outer.paragraph_count = 8;
    outer.utility_descendant_count = 15;
    inner.heading_count = 4;
    inner.link_count = 12;
    inner.paragraph_count = 7;
    inner.utility_descendant_count = 3;
    inner.text_char_count = 980;
    assert_eq!(
        nested_content_candidate_bias_deltas_for_tests(
            &outer,
            &inner,
            ContentCandidateTestPreference::Extraction,
        ),
        (-1_520, 1_870),
    );
}

#[test]
fn utility_light_descendant_preference_requires_every_nonzero_threshold() {
    let (mut outer, mut inner) = nested_candidates();
    outer.heading_count = 5;
    outer.link_count = 13;
    outer.paragraph_count = 4;
    outer.utility_descendant_count = 5;
    inner.heading_count = 3;
    inner.link_count = 5;
    inner.paragraph_count = 3;
    inner.utility_descendant_count = 3;
    inner.text_char_count = 920;
    assert!(extraction_prefers_utility_light_descendant_for_tests(
        &outer, &inner
    ));

    inner.text_char_count = 919;
    assert!(!extraction_prefers_utility_light_descendant_for_tests(
        &outer, &inner
    ));
    inner.text_char_count = 920;

    inner.paragraph_count = 2;
    assert!(!extraction_prefers_utility_light_descendant_for_tests(
        &outer, &inner
    ));
    inner.paragraph_count = 3;

    outer.heading_count = 6;
    assert!(!extraction_prefers_utility_light_descendant_for_tests(
        &outer, &inner
    ));
    outer.heading_count = 5;

    outer.link_count = 12;
    outer.utility_descendant_count = 4;
    assert!(!extraction_prefers_utility_light_descendant_for_tests(
        &outer, &inner
    ));

    outer.link_count = 13;
    assert!(extraction_prefers_utility_light_descendant_for_tests(
        &outer, &inner
    ));

    outer.link_count = 12;
    outer.utility_descendant_count = 5;
    assert!(extraction_prefers_utility_light_descendant_for_tests(
        &outer, &inner
    ));
}

#[test]
fn heading_and_link_light_descendant_preference_requires_exact_nonzero_gaps() {
    let (mut outer, mut inner) = nested_candidates();
    outer.heading_count = 15;
    outer.link_count = 29;
    inner.heading_count = 3;
    inner.link_count = 5;
    inner.text_char_count = 880;
    assert!(extraction_prefers_heading_and_link_light_descendant_for_tests(&outer, &inner));

    inner.text_char_count = 879;
    assert!(!extraction_prefers_heading_and_link_light_descendant_for_tests(&outer, &inner));
    inner.text_char_count = 880;

    outer.heading_count = 14;
    assert!(!extraction_prefers_heading_and_link_light_descendant_for_tests(&outer, &inner));
    outer.heading_count = 15;

    outer.link_count = 28;
    assert!(!extraction_prefers_heading_and_link_light_descendant_for_tests(&outer, &inner));
}

#[test]
fn near_complete_link_light_descendant_preference_requires_exact_nonzero_gaps() {
    let (mut outer, mut inner) = nested_candidates();
    outer.heading_count = 7;
    outer.link_count = 25;
    inner.heading_count = 3;
    inner.link_count = 5;
    inner.text_char_count = 980;
    assert!(extraction_prefers_near_complete_link_light_descendant_for_tests(&outer, &inner));

    inner.text_char_count = 979;
    assert!(!extraction_prefers_near_complete_link_light_descendant_for_tests(&outer, &inner));
    inner.text_char_count = 980;

    outer.heading_count = 2;
    assert!(!extraction_prefers_near_complete_link_light_descendant_for_tests(&outer, &inner));
    outer.heading_count = 7;

    outer.link_count = 24;
    assert!(!extraction_prefers_near_complete_link_light_descendant_for_tests(&outer, &inner));
}

#[test]
fn heavy_link_descendant_preference_requires_exact_nonzero_gaps() {
    let (mut outer, mut inner) = nested_candidates();
    outer.link_count = 144;
    inner.link_count = 24;
    inner.text_char_count = 980;
    assert!(prefers_heavy_link_descendant_for_tests(&outer, &inner));

    inner.text_char_count = 979;
    assert!(!prefers_heavy_link_descendant_for_tests(&outer, &inner));
    inner.text_char_count = 980;

    outer.link_count = 143;
    assert!(!prefers_heavy_link_descendant_for_tests(&outer, &inner));
}

#[test]
fn stable_link_light_descendant_preference_requires_every_exact_boundary() {
    let (mut outer, mut inner) = nested_candidates();
    outer.selector = "article.outer";
    outer.heading_count = 7;
    outer.link_count = 25;
    inner.selector = "#inner";
    inner.heading_count = 3;
    inner.link_count = 5;
    inner.text_char_count = 950;
    assert!(extraction_prefers_stable_link_light_descendant_for_tests(
        &outer, &inner, false
    ));

    assert!(!extraction_prefers_stable_link_light_descendant_for_tests(
        &outer, &inner, true
    ));
    inner.text_char_count = 949;
    assert!(!extraction_prefers_stable_link_light_descendant_for_tests(
        &outer, &inner, false
    ));
    inner.text_char_count = 950;

    outer.heading_count = 8;
    assert!(!extraction_prefers_stable_link_light_descendant_for_tests(
        &outer, &inner, false
    ));
    outer.heading_count = 7;

    outer.link_count = 24;
    assert!(!extraction_prefers_stable_link_light_descendant_for_tests(
        &outer, &inner, false
    ));
    outer.link_count = 25;

    outer.selector = "#outer";
    inner.selector = "article.inner";
    assert!(!extraction_prefers_stable_link_light_descendant_for_tests(
        &outer, &inner, false
    ));
}

#[test]
fn large_outer_candidate_preservation_requires_true_multiplicative_boundaries() {
    let (mut outer, mut inner) = nested_candidates();
    outer.text_char_count = 1_800;
    outer.heading_count = 7;
    outer.paragraph_count = 7;
    inner.text_char_count = 300;
    inner.heading_count = 3;
    inner.paragraph_count = 3;
    assert!(extraction_preserves_large_outer_candidate_for_tests(
        &outer, &inner
    ));

    outer.text_char_count = 1_000;
    assert!(!extraction_preserves_large_outer_candidate_for_tests(
        &outer, &inner
    ));
    outer.text_char_count = 1_800;

    outer.paragraph_count = 6;
    assert!(!extraction_preserves_large_outer_candidate_for_tests(
        &outer, &inner
    ));
    outer.paragraph_count = 7;

    outer.heading_count = 6;
    assert!(!extraction_preserves_large_outer_candidate_for_tests(
        &outer, &inner
    ));
}
