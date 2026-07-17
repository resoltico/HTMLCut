use super::support::*;
use super::*;

#[test]
fn nested_candidate_bias_ranks_structure_and_filters_noise() {
    let mut sorted_candidates = [
        ranked_candidate(CandidateFixture {
            path: "a",
            selector: "selector-b",
            text_char_count: 500,
            heading_count: 2,
            link_count: 0,
            primary_heading_level: None,
            primary_heading_count: 0,
            primary_heading_depth: None,
            utility_descendant_count: 0,
            score: 100,
        }),
        ranked_candidate(CandidateFixture {
            path: "b",
            selector: "selector-a",
            text_char_count: 700,
            heading_count: 1,
            link_count: 0,
            primary_heading_level: None,
            primary_heading_count: 0,
            primary_heading_depth: None,
            utility_descendant_count: 0,
            score: 100,
        }),
        ranked_candidate(CandidateFixture {
            path: "c",
            selector: "selector-c",
            text_char_count: 700,
            heading_count: 3,
            link_count: 0,
            primary_heading_level: None,
            primary_heading_count: 0,
            primary_heading_depth: None,
            utility_descendant_count: 0,
            score: 100,
        }),
    ];
    sorted_candidates.sort_by(compare_content_candidates);
    assert_eq!(sorted_candidates[0].inspection.selector, "selector-c");
    assert_eq!(sorted_candidates[1].inspection.selector, "selector-a");
    assert_eq!(sorted_candidates[2].inspection.selector, "selector-b");

    let mut shallow_heading_pair = vec![
        ranked_candidate(CandidateFixture {
            path: "article:nth-of-type(1)",
            selector: "article",
            text_char_count: 1_000,
            heading_count: 2,
            link_count: 60,
            primary_heading_level: Some(1),
            primary_heading_count: 1,
            primary_heading_depth: Some(1),
            utility_descendant_count: 20,
            score: 500,
        }),
        ranked_candidate(CandidateFixture {
            path: "article:nth-of-type(1) > div:nth-of-type(1)",
            selector: "article > div",
            text_char_count: 800,
            heading_count: 1,
            link_count: 5,
            primary_heading_level: None,
            primary_heading_count: 0,
            primary_heading_depth: None,
            utility_descendant_count: 0,
            score: 500,
        }),
    ];
    apply_nested_content_candidate_bias(&mut shallow_heading_pair);
    assert_eq!(shallow_heading_pair[0].score, 590);
    assert_eq!(shallow_heading_pair[1].score, 400);

    let mut heading_rich_outer = vec![
        ranked_candidate(CandidateFixture {
            path: "main:nth-of-type(1)",
            selector: "main",
            text_char_count: 1_000,
            heading_count: 6,
            link_count: 6,
            primary_heading_level: None,
            primary_heading_count: 0,
            primary_heading_depth: None,
            utility_descendant_count: 1,
            score: 300,
        }),
        ranked_candidate(CandidateFixture {
            path: "main:nth-of-type(1) > section:nth-of-type(1)",
            selector: "main > section",
            text_char_count: 850,
            heading_count: 1,
            link_count: 10,
            primary_heading_level: None,
            primary_heading_count: 0,
            primary_heading_depth: None,
            utility_descendant_count: 1,
            score: 300,
        }),
    ];
    apply_nested_content_candidate_bias(&mut heading_rich_outer);
    assert_eq!(heading_rich_outer[0].score, 390);
    assert_eq!(heading_rich_outer[1].score, 190);

    let mut modest_heading_outer = vec![
        ranked_candidate(CandidateFixture {
            path: "div:nth-of-type(1)",
            selector: "div",
            text_char_count: 1_000,
            heading_count: 3,
            link_count: 2,
            primary_heading_level: None,
            primary_heading_count: 0,
            primary_heading_depth: None,
            utility_descendant_count: 1,
            score: 200,
        }),
        ranked_candidate(CandidateFixture {
            path: "div:nth-of-type(1) > section:nth-of-type(1)",
            selector: "div > section",
            text_char_count: 950,
            heading_count: 1,
            link_count: 3,
            primary_heading_level: None,
            primary_heading_count: 0,
            primary_heading_depth: None,
            utility_descendant_count: 1,
            score: 200,
        }),
    ];
    apply_nested_content_candidate_bias(&mut modest_heading_outer);
    assert_eq!(modest_heading_outer[0].score, 255);
    assert_eq!(modest_heading_outer[1].score, 125);

    let mut utility_heavy_outer = vec![
        ranked_candidate(CandidateFixture {
            path: "main:nth-of-type(1)",
            selector: "main",
            text_char_count: 1_000,
            heading_count: 1,
            link_count: 2,
            primary_heading_level: None,
            primary_heading_count: 0,
            primary_heading_depth: None,
            utility_descendant_count: 20,
            score: 150,
        }),
        ranked_candidate(CandidateFixture {
            path: "main:nth-of-type(1) > article:nth-of-type(1)",
            selector: "main > article",
            text_char_count: 900,
            heading_count: 1,
            link_count: 0,
            primary_heading_level: None,
            primary_heading_count: 0,
            primary_heading_depth: None,
            utility_descendant_count: 0,
            score: 150,
        }),
    ];
    apply_nested_content_candidate_bias(&mut utility_heavy_outer);
    assert_eq!(utility_heavy_outer[0].score, 55);
    assert_eq!(utility_heavy_outer[1].score, 270);

    let mut primary_heading_count_outer = vec![
        ranked_candidate(CandidateFixture {
            path: "main:nth-of-type(1)",
            selector: "main",
            text_char_count: 1_000,
            heading_count: 2,
            link_count: 6,
            primary_heading_level: Some(1),
            primary_heading_count: 2,
            primary_heading_depth: Some(1),
            utility_descendant_count: 1,
            score: 400,
        }),
        ranked_candidate(CandidateFixture {
            path: "main:nth-of-type(1) > article:nth-of-type(1)",
            selector: "main > article",
            text_char_count: 900,
            heading_count: 2,
            link_count: 8,
            primary_heading_level: Some(1),
            primary_heading_count: 1,
            primary_heading_depth: Some(2),
            utility_descendant_count: 3,
            score: 400,
        }),
    ];
    apply_nested_content_candidate_bias(&mut primary_heading_count_outer);
    assert_eq!(primary_heading_count_outer[0].score, 485);
    assert_eq!(primary_heading_count_outer[1].score, 285);

    let mut too_short_inner = vec![
        ranked_candidate(CandidateFixture {
            path: "section:nth-of-type(1)",
            selector: "section",
            text_char_count: 1_000,
            heading_count: 1,
            link_count: 20,
            primary_heading_level: None,
            primary_heading_count: 0,
            primary_heading_depth: None,
            utility_descendant_count: 2,
            score: 175,
        }),
        ranked_candidate(CandidateFixture {
            path: "section:nth-of-type(1) > div:nth-of-type(1)",
            selector: "section > div",
            text_char_count: 500,
            heading_count: 1,
            link_count: 0,
            primary_heading_level: None,
            primary_heading_count: 0,
            primary_heading_depth: None,
            utility_descendant_count: 2,
            score: 175,
        }),
    ];
    apply_nested_content_candidate_bias(&mut too_short_inner);
    assert_eq!(too_short_inner[0].score, 175);
    assert_eq!(too_short_inner[1].score, 175);

    let mut link_heavy_inner = vec![
        ranked_candidate(CandidateFixture {
            path: "section:nth-of-type(1)",
            selector: "section",
            text_char_count: 1_000,
            heading_count: 1,
            link_count: 50,
            primary_heading_level: None,
            primary_heading_count: 0,
            primary_heading_depth: None,
            utility_descendant_count: 0,
            score: 220,
        }),
        ranked_candidate(CandidateFixture {
            path: "section:nth-of-type(1) > div:nth-of-type(1)",
            selector: "section > div",
            text_char_count: 900,
            heading_count: 1,
            link_count: 41,
            primary_heading_level: None,
            primary_heading_count: 0,
            primary_heading_depth: None,
            utility_descendant_count: 0,
            score: 220,
        }),
    ];
    apply_nested_content_candidate_bias(&mut link_heavy_inner);
    assert_eq!(link_heavy_inner[0].score, 220);
    assert_eq!(link_heavy_inner[1].score, 220);

    let mut shallow_heading_link_guard = vec![
        ranked_candidate(CandidateFixture {
            path: "article:nth-of-type(1)",
            selector: "article",
            text_char_count: 1_000,
            heading_count: 2,
            link_count: 90,
            primary_heading_level: Some(1),
            primary_heading_count: 1,
            primary_heading_depth: Some(1),
            utility_descendant_count: 20,
            score: 320,
        }),
        ranked_candidate(CandidateFixture {
            path: "article:nth-of-type(1) > div:nth-of-type(1)",
            selector: "article > div",
            text_char_count: 800,
            heading_count: 1,
            link_count: 5,
            primary_heading_level: None,
            primary_heading_count: 0,
            primary_heading_depth: None,
            utility_descendant_count: 0,
            score: 320,
        }),
    ];
    apply_nested_content_candidate_bias(&mut shallow_heading_link_guard);
    assert_eq!(shallow_heading_link_guard[0].score, 225);
    assert_eq!(shallow_heading_link_guard[1].score, 440);

    let mut extraction_primary_heading_guard = vec![
        ranked_candidate(CandidateFixture {
            path: "main:nth-of-type(1)",
            selector: "main",
            text_char_count: 1_000,
            heading_count: 3,
            link_count: 24,
            primary_heading_level: Some(1),
            primary_heading_count: 1,
            primary_heading_depth: Some(1),
            utility_descendant_count: 8,
            score: 300,
        }),
        ranked_candidate(CandidateFixture {
            path: "main:nth-of-type(1) > div:nth-of-type(1)",
            selector: "main > div",
            text_char_count: 950,
            heading_count: 2,
            link_count: 8,
            primary_heading_level: None,
            primary_heading_count: 0,
            primary_heading_depth: None,
            utility_descendant_count: 0,
            score: 300,
        }),
    ];
    apply_nested_content_candidate_bias_for(
        &mut extraction_primary_heading_guard,
        CandidatePreference::Extraction,
    );
    assert!(extraction_primary_heading_guard[0].score > extraction_primary_heading_guard[1].score);

    let mut primary_heading_utility_guard = vec![
        ranked_candidate(CandidateFixture {
            path: "main:nth-of-type(1)",
            selector: "main",
            text_char_count: 1_000,
            heading_count: 2,
            link_count: 6,
            primary_heading_level: Some(1),
            primary_heading_count: 2,
            primary_heading_depth: Some(1),
            utility_descendant_count: 12,
            score: 410,
        }),
        ranked_candidate(CandidateFixture {
            path: "main:nth-of-type(1) > article:nth-of-type(1)",
            selector: "main > article",
            text_char_count: 900,
            heading_count: 2,
            link_count: 8,
            primary_heading_level: Some(1),
            primary_heading_count: 1,
            primary_heading_depth: Some(2),
            utility_descendant_count: 1,
            score: 410,
        }),
    ];
    apply_nested_content_candidate_bias(&mut primary_heading_utility_guard);
    assert_eq!(primary_heading_utility_guard[0].score, 315);
    assert_eq!(primary_heading_utility_guard[1].score, 530);

    let mut heading_rich_link_guard = vec![
        ranked_candidate(CandidateFixture {
            path: "main:nth-of-type(1)",
            selector: "main",
            text_char_count: 1_000,
            heading_count: 6,
            link_count: 30,
            primary_heading_level: None,
            primary_heading_count: 0,
            primary_heading_depth: None,
            utility_descendant_count: 20,
            score: 250,
        }),
        ranked_candidate(CandidateFixture {
            path: "main:nth-of-type(1) > section:nth-of-type(1)",
            selector: "main > section",
            text_char_count: 850,
            heading_count: 1,
            link_count: 0,
            primary_heading_level: None,
            primary_heading_count: 0,
            primary_heading_depth: None,
            utility_descendant_count: 1,
            score: 250,
        }),
    ];
    apply_nested_content_candidate_bias(&mut heading_rich_link_guard);
    assert_eq!(heading_rich_link_guard[0].score, 250);
    assert_eq!(heading_rich_link_guard[1].score, 250);

    let mut heading_rich_utility_guard = vec![
        ranked_candidate(CandidateFixture {
            path: "main:nth-of-type(1)",
            selector: "main",
            text_char_count: 1_000,
            heading_count: 6,
            link_count: 10,
            primary_heading_level: None,
            primary_heading_count: 0,
            primary_heading_depth: None,
            utility_descendant_count: 20,
            score: 255,
        }),
        ranked_candidate(CandidateFixture {
            path: "main:nth-of-type(1) > section:nth-of-type(1)",
            selector: "main > section",
            text_char_count: 850,
            heading_count: 1,
            link_count: 0,
            primary_heading_level: None,
            primary_heading_count: 0,
            primary_heading_depth: None,
            utility_descendant_count: 1,
            score: 255,
        }),
    ];
    apply_nested_content_candidate_bias(&mut heading_rich_utility_guard);
    assert_eq!(heading_rich_utility_guard[0].score, 255);
    assert_eq!(heading_rich_utility_guard[1].score, 255);

    let mut modest_heading_utility_guard = vec![
        ranked_candidate(CandidateFixture {
            path: "div:nth-of-type(1)",
            selector: "div",
            text_char_count: 1_000,
            heading_count: 3,
            link_count: 2,
            primary_heading_level: None,
            primary_heading_count: 0,
            primary_heading_depth: None,
            utility_descendant_count: 8,
            score: 210,
        }),
        ranked_candidate(CandidateFixture {
            path: "div:nth-of-type(1) > section:nth-of-type(1)",
            selector: "div > section",
            text_char_count: 950,
            heading_count: 1,
            link_count: 3,
            primary_heading_level: None,
            primary_heading_count: 0,
            primary_heading_depth: None,
            utility_descendant_count: 1,
            score: 210,
        }),
    ];
    apply_nested_content_candidate_bias(&mut modest_heading_utility_guard);
    assert_eq!(modest_heading_utility_guard[0].score, 210);
    assert_eq!(modest_heading_utility_guard[1].score, 210);

    let mut modest_heading_link_guard = vec![
        ranked_candidate(CandidateFixture {
            path: "div:nth-of-type(1)",
            selector: "div",
            text_char_count: 1_000,
            heading_count: 3,
            link_count: 15,
            primary_heading_level: None,
            primary_heading_count: 0,
            primary_heading_depth: None,
            utility_descendant_count: 20,
            score: 215,
        }),
        ranked_candidate(CandidateFixture {
            path: "div:nth-of-type(1) > section:nth-of-type(1)",
            selector: "div > section",
            text_char_count: 950,
            heading_count: 1,
            link_count: 0,
            primary_heading_level: None,
            primary_heading_count: 0,
            primary_heading_depth: None,
            utility_descendant_count: 1,
            score: 215,
        }),
    ];
    apply_nested_content_candidate_bias(&mut modest_heading_link_guard);
    assert_eq!(modest_heading_link_guard[0].score, 120);
    assert_eq!(modest_heading_link_guard[1].score, 335);
}
