use super::support::*;
use super::*;

#[test]
fn precise_reading_descendant_promotion_prefers_near_full_article_descendants() {
    let mut extraction_candidates = vec![
        ranked_content_candidate(PromotionFixture {
            selector: "#content",
            path: "html > body > main#content",
            tag_name: "main",
            text_char_count: 1_000,
            heading_count: 3,
            link_count: 8,
            primary_heading_level: Some(1),
            primary_heading_depth: Some(1),
        }),
        ranked_content_candidate(PromotionFixture {
            selector: "main#content > section.tools",
            path: "html > body > main#content > section.tools",
            tag_name: "section",
            text_char_count: 120,
            heading_count: 1,
            link_count: 6,
            primary_heading_level: None,
            primary_heading_depth: None,
        }),
    ];
    let reading_candidates = vec![
        extraction_candidates[0].clone(),
        ranked_content_candidate(PromotionFixture {
            selector: "article.article-body",
            path: "html > body > main#content > article.article-body",
            tag_name: "article",
            text_char_count: 950,
            heading_count: 3,
            link_count: 5,
            primary_heading_level: Some(1),
            primary_heading_depth: Some(2),
        }),
        ranked_content_candidate(PromotionFixture {
            selector: "section.related-links",
            path: "html > body > main#content > section.related-links",
            tag_name: "section",
            text_char_count: 940,
            heading_count: 2,
            link_count: 8,
            primary_heading_level: None,
            primary_heading_depth: None,
        }),
    ];

    promote_precise_reading_descendant_candidate(&mut extraction_candidates, &reading_candidates);

    assert_eq!(
        extraction_candidates[0].inspection.selector,
        "article.article-body"
    );
    assert_eq!(
        extraction_candidates[0].inspection.path,
        "html > body > main#content > article.article-body"
    );
}

#[test]
fn precise_reading_descendant_promotion_prefers_fewer_links_when_candidates_are_tied() {
    let mut extraction_candidates = vec![ranked_content_candidate(PromotionFixture {
        selector: "#content",
        path: "html > body > main#content",
        tag_name: "main",
        text_char_count: 1_000,
        heading_count: 3,
        link_count: 8,
        primary_heading_level: Some(1),
        primary_heading_depth: Some(1),
    })];
    let reading_candidates = vec![
        extraction_candidates[0].clone(),
        ranked_content_candidate(PromotionFixture {
            selector: "article.feature-a",
            path: "html > body > main#content > article.feature-a",
            tag_name: "article",
            text_char_count: 930,
            heading_count: 3,
            link_count: 6,
            primary_heading_level: Some(1),
            primary_heading_depth: Some(2),
        }),
        ranked_content_candidate(PromotionFixture {
            selector: "article.feature-b",
            path: "html > body > main#content > article.feature-b",
            tag_name: "article",
            text_char_count: 930,
            heading_count: 3,
            link_count: 4,
            primary_heading_level: Some(1),
            primary_heading_depth: Some(2),
        }),
    ];

    promote_precise_reading_descendant_candidate(&mut extraction_candidates, &reading_candidates);

    assert_eq!(
        extraction_candidates[0].inspection.selector,
        "article.feature-b"
    );
}

#[test]
fn precise_reading_descendant_promotion_rejects_descendants_that_drop_too_much_content() {
    let mut extraction_candidates = vec![ranked_content_candidate(PromotionFixture {
        selector: "#content",
        path: "html > body > main#content",
        tag_name: "main",
        text_char_count: 1_000,
        heading_count: 4,
        link_count: 8,
        primary_heading_level: Some(1),
        primary_heading_depth: Some(1),
    })];
    let reading_candidates = vec![
        extraction_candidates[0].clone(),
        ranked_content_candidate(PromotionFixture {
            selector: "article.short-fragment",
            path: "html > body > main#content > article.short-fragment",
            tag_name: "article",
            text_char_count: 800,
            heading_count: 4,
            link_count: 2,
            primary_heading_level: Some(1),
            primary_heading_depth: Some(2),
        }),
        ranked_content_candidate(PromotionFixture {
            selector: "article.too-few-headings",
            path: "html > body > main#content > article.too-few-headings",
            tag_name: "article",
            text_char_count: 980,
            heading_count: 1,
            link_count: 2,
            primary_heading_level: Some(2),
            primary_heading_depth: Some(2),
        }),
    ];

    promote_precise_reading_descendant_candidate(&mut extraction_candidates, &reading_candidates);

    assert_eq!(extraction_candidates[0].inspection.selector, "#content");
}
