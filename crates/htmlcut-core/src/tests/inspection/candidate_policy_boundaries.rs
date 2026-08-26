use crate::inspect::{
    ContentCandidateTestInput, extraction_prefers_heading_and_link_light_descendant_for_tests,
    extraction_prefers_near_complete_link_light_descendant_for_tests,
    extraction_prefers_stable_link_light_descendant_for_tests,
    extraction_prefers_utility_light_descendant_for_tests,
    extraction_preserves_large_outer_candidate_for_tests,
    extraction_preserves_title_bearing_outer_wrapper_for_tests,
    prefers_heavy_link_descendant_for_tests, prefers_utility_light_descendant_for_tests,
    preserves_heading_rich_outer_candidate_for_tests,
    preserves_primary_heading_outer_candidate_for_tests,
    preserves_title_bearing_outer_candidate_for_tests,
};

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
fn title_bearing_outer_wrapper_preservation_requires_every_exact_boundary() {
    let (mut outer, mut inner) = nested_candidates();
    outer.paragraph_count = 4;
    inner.text_char_count = 850;
    assert!(extraction_preserves_title_bearing_outer_wrapper_for_tests(
        &outer, &inner, true
    ));

    assert!(!extraction_preserves_title_bearing_outer_wrapper_for_tests(
        &outer, &inner, false
    ));

    inner.text_char_count = 849;
    assert!(!extraction_preserves_title_bearing_outer_wrapper_for_tests(
        &outer, &inner, true
    ));
    inner.text_char_count = 850;

    outer.paragraph_count = 0;
    assert!(!extraction_preserves_title_bearing_outer_wrapper_for_tests(
        &outer, &inner, true
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
fn title_bearing_outer_candidate_preservation_requires_every_exact_boundary() {
    let (mut outer, mut inner) = nested_candidates();
    outer.paragraph_count = 4;
    outer.link_count = 75;
    inner.link_count = 5;
    inner.text_char_count = 700;
    assert!(preserves_title_bearing_outer_candidate_for_tests(
        &outer, &inner, true
    ));

    assert!(!preserves_title_bearing_outer_candidate_for_tests(
        &outer, &inner, false
    ));

    inner.text_char_count = 699;
    assert!(!preserves_title_bearing_outer_candidate_for_tests(
        &outer, &inner, true
    ));
    inner.text_char_count = 700;

    outer.paragraph_count = 0;
    assert!(!preserves_title_bearing_outer_candidate_for_tests(
        &outer, &inner, true
    ));
    outer.paragraph_count = 4;

    outer.link_count = 76;
    assert!(!preserves_title_bearing_outer_candidate_for_tests(
        &outer, &inner, true
    ));
}

#[test]
fn primary_heading_outer_candidate_preservation_requires_every_exact_boundary() {
    let (mut outer, mut inner) = nested_candidates();
    outer.paragraph_count = 4;
    outer.primary_heading_count = 2;
    outer.link_count = 25;
    outer.utility_descendant_count = 9;
    inner.primary_heading_count = 1;
    inner.link_count = 5;
    inner.utility_descendant_count = 3;
    inner.text_char_count = 800;
    assert!(preserves_primary_heading_outer_candidate_for_tests(
        &outer, &inner
    ));

    outer.paragraph_count = 0;
    assert!(!preserves_primary_heading_outer_candidate_for_tests(
        &outer, &inner
    ));
    outer.paragraph_count = 4;

    outer.primary_heading_count = 1;
    assert!(!preserves_primary_heading_outer_candidate_for_tests(
        &outer, &inner
    ));
    outer.primary_heading_count = 2;

    inner.text_char_count = 799;
    assert!(!preserves_primary_heading_outer_candidate_for_tests(
        &outer, &inner
    ));
    inner.text_char_count = 800;

    outer.link_count = 26;
    assert!(!preserves_primary_heading_outer_candidate_for_tests(
        &outer, &inner
    ));
    outer.link_count = 25;

    outer.utility_descendant_count = 10;
    assert!(!preserves_primary_heading_outer_candidate_for_tests(
        &outer, &inner
    ));
}

#[test]
fn heading_rich_outer_candidate_preservation_requires_every_exact_boundary() {
    let (mut outer, mut inner) = nested_candidates();
    outer.paragraph_count = 4;
    outer.heading_count = 7;
    outer.link_count = 25;
    outer.utility_descendant_count = 9;
    inner.heading_count = 3;
    inner.link_count = 5;
    inner.utility_descendant_count = 3;
    inner.text_char_count = 800;
    assert!(preserves_heading_rich_outer_candidate_for_tests(
        &outer, &inner
    ));

    outer.paragraph_count = 0;
    assert!(!preserves_heading_rich_outer_candidate_for_tests(
        &outer, &inner
    ));
    outer.paragraph_count = 4;

    inner.text_char_count = 799;
    assert!(!preserves_heading_rich_outer_candidate_for_tests(
        &outer, &inner
    ));
    inner.text_char_count = 800;

    outer.heading_count = 6;
    assert!(!preserves_heading_rich_outer_candidate_for_tests(
        &outer, &inner
    ));
    outer.heading_count = 7;

    outer.link_count = 26;
    assert!(!preserves_heading_rich_outer_candidate_for_tests(
        &outer, &inner
    ));
    outer.link_count = 25;

    outer.utility_descendant_count = 10;
    assert!(!preserves_heading_rich_outer_candidate_for_tests(
        &outer, &inner
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

#[test]
fn utility_light_descendant_preference_requires_each_nonzero_alternative() {
    let (mut outer, mut inner) = nested_candidates();
    outer.heading_count = 5;
    outer.link_count = 30;
    outer.paragraph_count = 5;
    outer.utility_descendant_count = 12;
    inner.heading_count = 3;
    inner.link_count = 10;
    inner.paragraph_count = 4;
    inner.utility_descendant_count = 4;
    inner.text_char_count = 780;
    assert!(prefers_utility_light_descendant_for_tests(&outer, &inner));

    inner.text_char_count = 779;
    assert!(!prefers_utility_light_descendant_for_tests(&outer, &inner));
    inner.text_char_count = 780;

    inner.paragraph_count = 3;
    assert!(!prefers_utility_light_descendant_for_tests(&outer, &inner));
    inner.paragraph_count = 4;

    outer.heading_count = 6;
    assert!(!prefers_utility_light_descendant_for_tests(&outer, &inner));
    outer.heading_count = 5;

    outer.utility_descendant_count = 11;
    outer.link_count = 18;
    assert!(!prefers_utility_light_descendant_for_tests(&outer, &inner));

    outer.utility_descendant_count = 5;
    outer.link_count = 19;
    assert!(prefers_utility_light_descendant_for_tests(&outer, &inner));
}
