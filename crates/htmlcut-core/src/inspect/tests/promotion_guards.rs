use super::support::*;
use super::*;

#[test]
fn promotion_guards_cover_empty_mismatched_and_rejected_candidate_sets() {
    let promoted = ranked_content_candidate(PromotionFixture {
        selector: "article.article-body",
        path: "html > body > main#content > article.article-body",
        tag_name: "article",
        text_char_count: 950,
        heading_count: 3,
        link_count: 5,
        primary_heading_level: Some(1),
        primary_heading_depth: Some(2),
    });

    let mut empty_extraction = Vec::new();
    promote_precise_reading_descendant_candidate(
        &mut empty_extraction,
        std::slice::from_ref(&promoted),
    );
    assert!(empty_extraction.is_empty());

    let mut unchanged_extraction = vec![ranked_content_candidate(PromotionFixture {
        selector: "#content",
        path: "html > body > main#content",
        tag_name: "main",
        text_char_count: 1_000,
        heading_count: 3,
        link_count: 8,
        primary_heading_level: Some(1),
        primary_heading_depth: Some(1),
    })];
    promote_precise_reading_descendant_candidate(&mut unchanged_extraction, &[]);
    assert_eq!(unchanged_extraction[0].inspection.selector, "#content");

    let mismatched_reading = vec![ranked_content_candidate(PromotionFixture {
        selector: "#different",
        path: "html > body > article#different",
        tag_name: "article",
        text_char_count: 1_000,
        heading_count: 3,
        link_count: 8,
        primary_heading_level: Some(1),
        primary_heading_depth: Some(1),
    })];
    promote_precise_reading_descendant_candidate(&mut unchanged_extraction, &mismatched_reading);
    assert_eq!(unchanged_extraction[0].inspection.selector, "#content");

    let tied_descendants = vec![
        unchanged_extraction[0].clone(),
        ranked_content_candidate(PromotionFixture {
            selector: "article.shallow",
            path: "html > body > main#content > article.shallow",
            tag_name: "article",
            text_char_count: 930,
            heading_count: 3,
            link_count: 4,
            primary_heading_level: Some(1),
            primary_heading_depth: Some(2),
        }),
        ranked_content_candidate(PromotionFixture {
            selector: "article.deep",
            path: "html > body > main#content > section > article.deep",
            tag_name: "article",
            text_char_count: 930,
            heading_count: 3,
            link_count: 4,
            primary_heading_level: Some(1),
            primary_heading_depth: Some(2),
        }),
    ];
    promote_precise_reading_descendant_candidate(&mut unchanged_extraction, &tied_descendants);
    assert_eq!(
        unchanged_extraction[0].inspection.selector,
        "article.shallow"
    );

    let mut chrome_wrapper_extraction = vec![ranked_content_candidate(PromotionFixture {
        selector: "#js-repo-pjax-container",
        path: "html > body > main#js-repo-pjax-container",
        tag_name: "main",
        text_char_count: 23_201,
        heading_count: 215,
        link_count: 619,
        primary_heading_level: Some(1),
        primary_heading_depth: Some(2),
    })];
    let chrome_wrapper_reading = vec![
        ranked_content_candidate(PromotionFixture {
            selector: "div.markdown-body",
            path: "html > body > main#js-repo-pjax-container > div > div > div.markdown-body",
            tag_name: "div",
            text_char_count: 23_026,
            heading_count: 53,
            link_count: 225,
            primary_heading_level: Some(2),
            primary_heading_depth: Some(2),
        }),
        ranked_content_candidate(PromotionFixture {
            selector: "#wiki-body",
            path: "html > body > main#js-repo-pjax-container > div > div > #wiki-body",
            tag_name: "div",
            text_char_count: 23_026,
            heading_count: 53,
            link_count: 225,
            primary_heading_level: Some(2),
            primary_heading_depth: Some(1),
        }),
    ];
    promote_precise_reading_descendant_candidate(
        &mut chrome_wrapper_extraction,
        &chrome_wrapper_reading,
    );
    assert_eq!(
        chrome_wrapper_extraction[0].inspection.selector,
        "#wiki-body"
    );

    let mut link_heavy_extraction = vec![ranked_content_candidate(PromotionFixture {
        selector: "#content",
        path: "html > body > main#content",
        tag_name: "main",
        text_char_count: 192_859,
        heading_count: 41,
        link_count: 2_951,
        primary_heading_level: Some(1),
        primary_heading_depth: Some(1),
    })];
    let link_heavy_reading = vec![
        ranked_content_candidate(PromotionFixture {
            selector: "div.mw-content-ltr.mw-parser-output",
            path: "html > body > main#content > div#bodyContent > div#mw-content-text > div.mw-content-ltr.mw-parser-output",
            tag_name: "div",
            text_char_count: 192_844,
            heading_count: 40,
            link_count: 2_641,
            primary_heading_level: Some(2),
            primary_heading_depth: Some(1),
        }),
        ranked_content_candidate(PromotionFixture {
            selector: "#mw-content-text",
            path: "html > body > main#content > div#bodyContent > div#mw-content-text",
            tag_name: "div",
            text_char_count: 192_844,
            heading_count: 40,
            link_count: 2_642,
            primary_heading_level: Some(2),
            primary_heading_depth: Some(1),
        }),
    ];
    promote_precise_reading_descendant_candidate(&mut link_heavy_extraction, &link_heavy_reading);
    assert_eq!(
        link_heavy_extraction[0].inspection.selector,
        "#mw-content-text"
    );

    let mut ancestor_empty = Vec::new();
    promote_title_bearing_reading_ancestor_candidate(
        &mut ancestor_empty,
        std::slice::from_ref(&promoted),
    );
    assert!(ancestor_empty.is_empty());

    let mut ancestor_candidate = vec![ranked_content_candidate(PromotionFixture {
        selector: "article.article-body",
        path: "html > body > main#content > article.article-body",
        tag_name: "article",
        text_char_count: 950,
        heading_count: 3,
        link_count: 12,
        primary_heading_level: Some(2),
        primary_heading_depth: Some(3),
    })];
    promote_title_bearing_reading_ancestor_candidate(&mut ancestor_candidate, &[]);
    assert_eq!(
        ancestor_candidate[0].inspection.selector,
        "article.article-body"
    );

    let same_path_reading = vec![ancestor_candidate[0].clone()];
    promote_title_bearing_reading_ancestor_candidate(&mut ancestor_candidate, &same_path_reading);
    assert_eq!(
        ancestor_candidate[0].inspection.selector,
        "article.article-body"
    );

    let unrelated_reading = vec![ranked_content_candidate(PromotionFixture {
        selector: "#sidebar",
        path: "html > body > aside#sidebar",
        tag_name: "aside",
        text_char_count: 980,
        heading_count: 4,
        link_count: 10,
        primary_heading_level: Some(1),
        primary_heading_depth: Some(2),
    })];
    promote_title_bearing_reading_ancestor_candidate(&mut ancestor_candidate, &unrelated_reading);
    assert_eq!(
        ancestor_candidate[0].inspection.selector,
        "article.article-body"
    );

    let no_title_drop = vec![ranked_content_candidate(PromotionFixture {
        selector: "#content",
        path: "html > body > main#content",
        tag_name: "main",
        text_char_count: 980,
        heading_count: 4,
        link_count: 18,
        primary_heading_level: Some(2),
        primary_heading_depth: Some(3),
    })];
    promote_title_bearing_reading_ancestor_candidate(&mut ancestor_candidate, &no_title_drop);
    assert_eq!(
        ancestor_candidate[0].inspection.selector,
        "article.article-body"
    );

    let too_small_reading = vec![ranked_content_candidate(PromotionFixture {
        selector: "#content",
        path: "html > body > main#content",
        tag_name: "main",
        text_char_count: 1_500,
        heading_count: 4,
        link_count: 18,
        primary_heading_level: Some(1),
        primary_heading_depth: Some(4),
    })];
    promote_title_bearing_reading_ancestor_candidate(&mut ancestor_candidate, &too_small_reading);
    assert_eq!(
        ancestor_candidate[0].inspection.selector,
        "article.article-body"
    );

    let too_many_headings = vec![ranked_content_candidate(PromotionFixture {
        selector: "#content",
        path: "html > body > main#content",
        tag_name: "main",
        text_char_count: 980,
        heading_count: 0,
        link_count: 18,
        primary_heading_level: Some(1),
        primary_heading_depth: Some(4),
    })];
    promote_title_bearing_reading_ancestor_candidate(&mut ancestor_candidate, &too_many_headings);
    assert_eq!(
        ancestor_candidate[0].inspection.selector,
        "article.article-body"
    );

    let too_many_links = vec![ranked_content_candidate(PromotionFixture {
        selector: "#content",
        path: "html > body > main#content",
        tag_name: "main",
        text_char_count: 980,
        heading_count: 4,
        link_count: 100,
        primary_heading_level: Some(1),
        primary_heading_depth: Some(4),
    })];
    promote_title_bearing_reading_ancestor_candidate(&mut ancestor_candidate, &too_many_links);
    assert_eq!(
        ancestor_candidate[0].inspection.selector,
        "article.article-body"
    );

    let mut selector_tiebreak_extraction = vec![ranked_content_candidate(PromotionFixture {
        selector: "#content",
        path: "html > body > main#content",
        tag_name: "main",
        text_char_count: 1_000,
        heading_count: 3,
        link_count: 8,
        primary_heading_level: Some(1),
        primary_heading_depth: Some(1),
    })];
    let selector_tiebreak_reading = vec![
        selector_tiebreak_extraction[0].clone(),
        ranked_content_candidate(PromotionFixture {
            selector: "article.alpha",
            path: "html > body > main#content > article.alpha",
            tag_name: "article",
            text_char_count: 930,
            heading_count: 3,
            link_count: 4,
            primary_heading_level: Some(1),
            primary_heading_depth: Some(2),
        }),
        ranked_content_candidate(PromotionFixture {
            selector: "article.beta",
            path: "html > body > main#content > article.beta",
            tag_name: "article",
            text_char_count: 930,
            heading_count: 3,
            link_count: 4,
            primary_heading_level: Some(1),
            primary_heading_depth: Some(2),
        }),
    ];
    promote_precise_reading_descendant_candidate(
        &mut selector_tiebreak_extraction,
        &selector_tiebreak_reading,
    );
    assert_eq!(
        selector_tiebreak_extraction[0].inspection.selector,
        "article.beta"
    );
}
