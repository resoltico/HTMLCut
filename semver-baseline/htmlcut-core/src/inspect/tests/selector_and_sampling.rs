use super::support::*;
use super::*;

#[test]
fn selector_and_sampling_helpers_cover_remaining_branches() {
    let document = parse_document_node(
        "<html><body>\
            <div class=\"content\"></div>\
            <section id=\"heading-scope\"></section>\
            <section id=\"link-scope\"></section>\
            <h2>Fallback Heading</h2>\
            <a href=\"/fallback\">Fallback Link</a>\
            <main id=\"main-content\" role=\"main\" itemprop=\"articleBody\" class=\"content story main\">\
                <h1>Main Title</h1>\
                <p>Alpha beta gamma delta epsilon zeta eta theta iota kappa lambda mu.</p>\
                <a href=\"/guide\">Guide</a>\
                <nav class=\"tools\"><a href=\"/edit\">Edit</a></nav>\
            </main>\
            <section class=\"story feature\">\
                <h2>Feature Title</h2>\
                <p>Support body text for selector testing.</p>\
            </section>\
            <section class=\"story feature duplicate\">\
                <h2>Feature Title Two</h2>\
                <p>Support body text for selector testing.</p>\
            </section>\
            <a href=\"/empty\"><img src=\"hero.png\" alt=\"\"></a>\
        </body></html>",
    );

    assert!(build_content_candidates(&document, 0).is_empty());
    let empty_role_main =
        parse_document_node("<html><body><div role=\"main\"></div></body></html>");
    assert!(build_content_candidates(&empty_role_main, 3).is_empty());
    let candidates = build_content_candidates(&document, 5);
    assert!(
        candidates
            .iter()
            .all(|candidate| candidate.text_char_count > 0)
    );
    let heading_scope = build_node_path(&select_first(&document, "#heading-scope").expect("scope"));
    assert!(build_heading_samples(&document, 0, std::slice::from_ref(&heading_scope)).is_empty());
    let headings = build_heading_samples(&document, 3, std::slice::from_ref(&heading_scope));
    assert_eq!(headings[0].text, "Fallback Heading");
    let main_scope = build_node_path(&select_first(&document, "#main-content").expect("main"));
    let main_headings = build_heading_samples(&document, 3, std::slice::from_ref(&main_scope));
    assert_eq!(main_headings[0].text, "Main Title");
    let feature_scope =
        build_node_path(&select_first(&document, "section.feature").expect("first feature"));
    let duplicate_feature_scope = build_node_path(
        &select_first(&document, "section.feature.duplicate").expect("duplicate feature"),
    );
    let combined_headings = build_heading_samples(
        &document,
        3,
        &[main_scope.clone(), feature_scope, duplicate_feature_scope],
    );
    assert_eq!(
        combined_headings
            .iter()
            .map(|heading| heading.text.as_str())
            .collect::<Vec<_>>(),
        vec!["Main Title"]
    );

    let heading_selector = Selector::parse("h2").expect("heading selector");
    let mut seen_heading_paths = BTreeSet::new();
    let duplicate_heading_document =
        parse_document_node("<section><h2>Only Heading</h2></section>");
    let duplicate_heading = select_first(&duplicate_heading_document, "h2").expect("heading");
    seen_heading_paths.insert(build_node_path(&duplicate_heading));
    assert!(
        sample_headings_from_scope(
            &duplicate_heading_document,
            None,
            3,
            &heading_selector,
            &mut seen_heading_paths,
        )
        .is_empty()
    );
    let mixed_heading_selector = Selector::parse("div, h2").expect("mixed selector");
    let mixed_heading_document =
        parse_document_node("<section><div>Ignore</div><h2>Only Heading</h2></section>");
    let mixed_headings = sample_headings_from_scope(
        &mixed_heading_document,
        None,
        3,
        &mixed_heading_selector,
        &mut BTreeSet::new(),
    );
    assert_eq!(mixed_headings.len(), 1);
    assert_eq!(mixed_headings[0].text, "Only Heading");
    let utility_heading_document = parse_document_node(
        "<section><nav><h2>Ignore Utility</h2></nav><h2>Keep Heading</h2></section>",
    );
    let utility_headings = sample_headings_from_scope(
        &utility_heading_document,
        None,
        3,
        &heading_selector,
        &mut BTreeSet::new(),
    );
    assert_eq!(utility_headings.len(), 1);
    assert_eq!(utility_headings[0].text, "Keep Heading");
    let heading_element_utility_document = parse_document_node(
        "<section><h2 class=\"editsection\">Ignore Utility</h2><h2>Keep</h2></section>",
    );
    let utility_element_headings = sample_headings_from_scope(
        &heading_element_utility_document,
        None,
        3,
        &heading_selector,
        &mut BTreeSet::new(),
    );
    assert_eq!(utility_element_headings.len(), 1);
    assert_eq!(utility_element_headings[0].text, "Keep");

    let link_scope = build_node_path(&select_first(&document, "#link-scope").expect("scope"));
    assert!(build_link_samples(&document, Some("https://example.test/base/"), 0, &[]).is_empty());
    let links = build_link_samples(
        &document,
        Some("https://example.test/base/"),
        3,
        std::slice::from_ref(&link_scope),
    );
    assert_eq!(links[0].href.as_deref(), Some("/fallback"));
    let main_links = build_link_samples(
        &document,
        Some("https://example.test/base/"),
        3,
        std::slice::from_ref(&main_scope),
    );
    assert_eq!(main_links[0].href.as_deref(), Some("/guide"));
    let dual_scope_document = parse_document_node(
        "<html><body>\
            <section class=\"content primary\"><a href=\"/first\">First</a></section>\
            <section class=\"content secondary\"><a href=\"/second\">Second</a></section>\
        </body></html>",
    );
    let primary_scope = build_node_path(
        &select_first(&dual_scope_document, "section.primary").expect("primary section"),
    );
    let secondary_scope = build_node_path(
        &select_first(&dual_scope_document, "section.secondary").expect("secondary section"),
    );
    let combined_links = build_link_samples(
        &dual_scope_document,
        Some("https://example.test/base/"),
        3,
        &[primary_scope, secondary_scope],
    );
    assert_eq!(combined_links[0].href.as_deref(), Some("/first"));

    let link_selector = Selector::parse("a").expect("link selector");
    let mut seen_link_paths = BTreeSet::new();
    let guide = select_first(&document, "main a[href=\"/guide\"]").expect("guide link");
    seen_link_paths.insert(build_node_path(&guide));
    assert!(
        sample_links_from_scope(
            &document,
            Some("https://example.test/base/"),
            Some(&build_node_path(
                &select_first(&document, "#main-content").expect("main")
            )),
            3,
            &link_selector,
            &mut seen_link_paths,
        )
        .is_empty()
    );
    assert!(
        sample_links_from_scope(
            &document,
            Some("https://example.test/base/"),
            Some(&build_node_path(
                &select_first(&document, "#main-content").expect("main")
            )),
            3,
            &link_selector,
            &mut BTreeSet::new(),
        )
        .iter()
        .any(|link| link.href.as_deref() == Some("/guide"))
    );
    assert!(
        sample_links_from_scope(
            &document,
            Some("https://example.test/base/"),
            None,
            10,
            &link_selector,
            &mut BTreeSet::new(),
        )
        .iter()
        .all(|link| !link.text.is_empty())
    );
    let utility_link_document = parse_document_node(
        "<section><nav><a href=\"/ignore\">Ignore</a></nav><a href=\"/keep\">Keep</a></section>",
    );
    let utility_links = sample_links_from_scope(
        &utility_link_document,
        Some("https://example.test/base/"),
        None,
        3,
        &link_selector,
        &mut BTreeSet::new(),
    );
    assert_eq!(utility_links.len(), 1);
    assert_eq!(utility_links[0].href.as_deref(), Some("/keep"));
    let utility_link_element_document = parse_document_node(
        "<section><a class=\"editsection\" href=\"/ignore\">Ignore</a><a href=\"/keep\">Keep</a></section>",
    );
    let utility_element_links = sample_links_from_scope(
        &utility_link_element_document,
        Some("https://example.test/base/"),
        None,
        3,
        &link_selector,
        &mut BTreeSet::new(),
    );
    assert_eq!(utility_element_links.len(), 1);
    assert_eq!(utility_element_links[0].href.as_deref(), Some("/keep"));
    let empty_text_link_document = parse_document_node(
        "<section><a href=\"/image-only\"><img alt=\"\" src=\"hero.png\"></a></section>",
    );
    assert!(
        sample_links_from_scope(
            &empty_text_link_document,
            Some("https://example.test/base/"),
            None,
            3,
            &link_selector,
            &mut BTreeSet::new(),
        )
        .is_empty()
    );
    let same_page_link_document = parse_document_node(
        "<article><a href=\"https://example.test/guide#overview\">Overview</a><a href=\"/next\">Next</a></article>",
    );
    let same_page_links = sample_links_from_scope(
        &same_page_link_document,
        Some("https://example.test/guide"),
        None,
        4,
        &link_selector,
        &mut BTreeSet::new(),
    );
    assert_eq!(same_page_links.len(), 1);
    assert_eq!(same_page_links[0].href.as_deref(), Some("/next"));
    let meaningful_link_document = parse_document_node(
        "<section>\
            <nav><a href=\"/ignore\">Ignore</a></nav>\
            <a href=\"https://example.test/guide#overview\">Overview</a>\
            <a href=\"/terms\">Terms apply</a>\
            <a href=\"/article\">Article link</a>\
            <a href=\"/image-only\"><img alt=\"\" src=\"hero.png\"></a>\
        </section>",
    );
    assert_eq!(
        count_meaningful_links(
            &select_first(&meaningful_link_document, "section").expect("section"),
            &link_selector,
        ),
        2
    );
    let utility_count_document = parse_document_node(
        "<section><a class=\"editsection\" href=\"/ignore\">Ignore</a><a href=\"/article\">Article link</a></section>",
    );
    assert_eq!(
        count_meaningful_links(
            &select_first(&utility_count_document, "section").expect("section"),
            &link_selector,
        ),
        1
    );

    assert_eq!(
        select_elements_in_scope(&document, Some("missing-scope"), &link_selector).count(),
        document.select(&link_selector).count()
    );

    let feature = select_first(&document, "section.feature").expect("feature");
    let feature_path = build_node_path(&feature);
    let feature_candidates = selector_candidates_for_element(&feature, &feature_path);
    assert!(feature_candidates.contains(&"section.story.feature".to_owned()));
    assert!(feature_candidates.contains(&"section".to_owned()));

    let main = select_first(&document, "#main-content").expect("main");
    let main_path = build_node_path(&main);
    let main_candidates = selector_candidates_for_element(&main, &main_path);
    assert!(main_candidates.contains(&"#main-content".to_owned()));
    assert!(main_candidates.contains(&"main[role=\"main\"]".to_owned()));
    assert!(main_candidates.contains(&"[role=\"main\"]".to_owned()));
    assert!(main_candidates.contains(&"main[itemprop=\"articleBody\"]".to_owned()));
    assert_eq!(
        recommend_content_selector(&document, &feature, &feature_path),
        feature_path
    );
    assert_eq!(
        recommend_content_selector(&document, &main, &main_path),
        "#main-content"
    );
    let plain_document = parse_document_node(
        "<html><body><section><p>One</p></section><section><p>Two</p></section></body></html>",
    );
    let plain_section = select_first(&plain_document, "section").expect("plain section");
    let plain_path = build_node_path(&plain_section);
    assert_eq!(
        recommend_content_selector(&plain_document, &plain_section, &plain_path),
        plain_path
    );

    assert!(selector_uniquely_matches(
        &document,
        "#main-content",
        main.id()
    ));
    assert!(!selector_uniquely_matches(
        &document,
        "section",
        feature.id()
    ));
    assert!(!selector_uniquely_matches(
        &document,
        "section[",
        feature.id()
    ));
    assert!(!selector_uniquely_matches(
        &document,
        "#missing",
        feature.id()
    ));
    let invalid_id_document = parse_document_node("<div id=\"9 hero\"></div>");
    let invalid_id = select_first(&invalid_id_document, "div").expect("invalid id");
    assert_eq!(id_selector("9 hero"), "[id=\"9 hero\"]");
    assert!(
        selector_candidates_for_element(&invalid_id, "div:nth-of-type(1)")
            .contains(&"[id=\"9 hero\"]".to_owned())
    );
    let blank_metadata_document = parse_document_node(
        "<div id=\"  \" role=\"  \" itemprop=\"  \" class=\"content hero\"></div>",
    );
    let blank_metadata = select_first(&blank_metadata_document, "div").expect("blank metadata");
    let blank_candidates = selector_candidates_for_element(&blank_metadata, "div:nth-of-type(1)");
    assert!(
        !blank_candidates
            .iter()
            .any(|candidate| candidate.starts_with('#'))
    );
    assert!(
        !blank_candidates
            .iter()
            .any(|candidate| candidate.contains("[role="))
    );
    assert!(
        !blank_candidates
            .iter()
            .any(|candidate| candidate.contains("[itemprop="))
    );
    assert!(blank_candidates.contains(&"div.hero".to_owned()));

    assert!(!simple_css_identifier(""));
    assert!(!simple_css_identifier("9feature"));
    assert!(!simple_css_identifier("feature!"));
    assert!(simple_css_identifier("_feature1"));
    assert!(simple_css_identifier("-feature"));
    assert!(simple_css_identifier("feature-card"));
    assert_eq!(css_string_literal("a\\\"b"), "a\\\\\\\"b");

    let role_and_itemprop_document = parse_document_node(
        "<div role=\"main\"></div><section itemprop=\"articleBody\"></section>",
    );
    assert!(element_attr_equals_ignore_ascii_case(
        &select_first(&role_and_itemprop_document, "div").expect("role main"),
        "role",
        "main",
    ));
    assert!(!element_attr_equals_ignore_ascii_case(
        &select_first(&role_and_itemprop_document, "div").expect("role main"),
        "itemprop",
        "articleBody",
    ));
    assert!(is_content_candidate_container(
        &select_first(&role_and_itemprop_document, "div").expect("role main"),
        0,
    ));
    assert!(is_content_candidate_container(
        &select_first(&role_and_itemprop_document, "section").expect("article body"),
        0,
    ));
    assert!(element_attr_equals_ignore_ascii_case(
        &select_first(&role_and_itemprop_document, "section").expect("article body"),
        "itemprop",
        "articleBody",
    ));
    assert!(!element_attr_equals_ignore_ascii_case(
        &select_first(&role_and_itemprop_document, "section").expect("article body"),
        "role",
        "main",
    ));
    let narrative_section_document = parse_document_node(
        "<section><h2>Design</h2><p>All-screen front.</p><p>Durable body.</p><ul><li>Feature one</li><li>Feature two</li></ul></section>",
    );
    let narrative_section =
        select_first(&narrative_section_document, "section").expect("narrative section");
    assert!(element_has_narrative_section_shape(&narrative_section));
    assert!(is_content_candidate_container(&narrative_section, 0));
    let shallow_section_document =
        parse_document_node("<section><h2>Design</h2><p>Single paragraph.</p></section>");
    let shallow_section =
        select_first(&shallow_section_document, "section").expect("shallow section");
    assert!(!element_has_narrative_section_shape(&shallow_section));
    assert!(!is_content_candidate_container(&shallow_section, 0));

    assert_eq!(descendant_element_depth(&main, &main), Some(0));
    let main_heading = select_first(&document, "#main-content h1").expect("heading");
    assert_eq!(descendant_element_depth(&main, &main_heading), Some(1));
    let fallback_heading = select_first(&document, "body > h2").expect("fallback heading");
    assert_eq!(descendant_element_depth(&main, &fallback_heading), None);

    assert_eq!(count_utility_descendant_roots(&main), 1);
    let nav = select_first(&document, "nav.tools").expect("nav");
    assert!(!has_utility_chrome_ancestor_before(&nav, main.id()));
    let nav_link = select_first(&document, "nav.tools a").expect("nav link");
    assert!(has_utility_chrome_ancestor_before(&nav_link, main.id()));
    assert!(!has_utility_chrome_ancestor_before(
        &fallback_heading,
        main.id()
    ));

    let alpha_tiebreak = ranked_candidate(CandidateFixture {
        path: "alpha-path",
        selector: "#alpha",
        score: 40,
        text_char_count: 200,
        heading_count: 2,
        link_count: 0,
        primary_heading_level: None,
        primary_heading_count: 0,
        primary_heading_depth: None,
        utility_descendant_count: 0,
    });
    let beta_tiebreak = ranked_candidate(CandidateFixture {
        path: "beta-path",
        selector: "#beta",
        score: 40,
        text_char_count: 200,
        heading_count: 2,
        link_count: 0,
        primary_heading_level: None,
        primary_heading_count: 0,
        primary_heading_depth: None,
        utility_descendant_count: 0,
    });
    assert_eq!(
        compare_content_candidates(&alpha_tiebreak, &beta_tiebreak),
        Ordering::Less
    );

    let medium_bodyless = content_candidate_score(&ContentCandidateScoreInputs {
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
    let dense_medium_links = content_candidate_score(&ContentCandidateScoreInputs {
        tag_name: "section",
        has_main_role: false,
        has_article_body_itemprop: false,
        text_char_count: 900,
        heading_count: 1,
        link_count: 14,
        paragraph_count: 2,
        positive_signal_count: 0,
        negative_signal_count: 0,
        primary_heading_level: None,
        primary_heading_count: 0,
        primary_heading_depth: None,
        utility_descendant_count: 0,
        uses_exact_path_selector: false,
    });
    let dense_large_links = content_candidate_score(&ContentCandidateScoreInputs {
        tag_name: "section",
        has_main_role: false,
        has_article_body_itemprop: false,
        text_char_count: 3_000,
        heading_count: 1,
        link_count: 12,
        paragraph_count: 2,
        positive_signal_count: 0,
        negative_signal_count: 0,
        primary_heading_level: None,
        primary_heading_count: 0,
        primary_heading_depth: None,
        utility_descendant_count: 0,
        uses_exact_path_selector: false,
    });
    let dense_wide_links = content_candidate_score(&ContentCandidateScoreInputs {
        tag_name: "section",
        has_main_role: false,
        has_article_body_itemprop: false,
        text_char_count: 5_000,
        heading_count: 1,
        link_count: 10,
        paragraph_count: 2,
        positive_signal_count: 0,
        negative_signal_count: 0,
        primary_heading_level: None,
        primary_heading_count: 0,
        primary_heading_depth: None,
        utility_descendant_count: 0,
        uses_exact_path_selector: false,
    });
    let supportive_body = content_candidate_score(&ContentCandidateScoreInputs {
        tag_name: "section",
        has_main_role: false,
        has_article_body_itemprop: false,
        text_char_count: 260,
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
    let dense_medium_baseline = content_candidate_score(&ContentCandidateScoreInputs {
        tag_name: "section",
        has_main_role: false,
        has_article_body_itemprop: false,
        text_char_count: 900,
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
    let dense_large_baseline = content_candidate_score(&ContentCandidateScoreInputs {
        tag_name: "section",
        has_main_role: false,
        has_article_body_itemprop: false,
        text_char_count: 3_000,
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
    let dense_wide_baseline = content_candidate_score(&ContentCandidateScoreInputs {
        tag_name: "section",
        has_main_role: false,
        has_article_body_itemprop: false,
        text_char_count: 5_000,
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
    assert!(medium_bodyless < supportive_body);
    assert!(dense_medium_links < dense_medium_baseline);
    assert!(dense_large_links < dense_large_baseline);
    assert!(dense_wide_links < dense_wide_baseline);
}
