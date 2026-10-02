use super::*;

#[test]
fn empty_parser_text_does_not_force_a_boundary_before_visible_content() {
    let mut html = scraper::Html::parse_document("<div><p>A</p></div>");
    let root_id = html
        .select(&scraper::Selector::parse("div").unwrap())
        .next()
        .unwrap()
        .id();
    html.tree
        .get_mut(root_id)
        .unwrap()
        .append(Node::Text(scraper::node::Text { text: "".into() }));
    let root = ElementRef::wrap(html.tree.get(root_id).unwrap()).unwrap();
    assert_eq!(
        render(
            root,
            &HashSet::new(),
            false,
            false,
            None,
            1024,
            &SelectorWorkBudget::new(1000)
        )
        .unwrap(),
        "A"
    );
    html.tree
        .get_mut(root_id)
        .unwrap()
        .append(Node::Text(scraper::node::Text { text: "B".into() }));
    let root = ElementRef::wrap(html.tree.get(root_id).unwrap()).unwrap();
    assert_eq!(
        render(
            root,
            &HashSet::new(),
            false,
            false,
            None,
            1024,
            &SelectorWorkBudget::new(1000)
        )
        .unwrap(),
        "A\nB"
    );
}

#[test]
fn fence_generation_accepts_its_exact_byte_bound() {
    let html = scraper::Html::parse_document("<pre>``</pre>");
    let root = html
        .select(&scraper::Selector::parse("pre").unwrap())
        .next()
        .unwrap();
    for maximum in [2, 3, 4] {
        let result = pre_fence(
            *root,
            &HashSet::new(),
            maximum,
            &SelectorWorkBudget::new(1000),
        );
        if maximum < 3 {
            assert_eq!(result.unwrap_err().code, ErrorCode::ResourceLimit);
        } else {
            assert_eq!(result.unwrap(), "```");
        }
    }
}

#[test]
fn inconsistent_internal_html_rows_and_cells_refuse_missing_context() {
    for name in ["tr", "td", "th"] {
        let mut document = scraper::Html::parse_document("<div><span>X</span></div>");
        let id = document
            .select(&scraper::Selector::parse("span").unwrap())
            .next()
            .unwrap()
            .id();
        if let scraper::Node::Element(element) = document.tree.get_mut(id).unwrap().value() {
            element.name.local = name.into();
        } else {
            panic!("element fixture");
        }
        let root = document
            .select(&scraper::Selector::parse("div").unwrap())
            .next()
            .unwrap();
        let error = render(
            root,
            &std::collections::HashSet::new(),
            false,
            false,
            None,
            1000,
            &selectors::work_budget::SelectorWorkBudget::new(1000),
        )
        .unwrap_err();
        assert_eq!(error.code, crate::ErrorCode::InternalInvariant);
        assert_eq!(error.code.exit_class(), 6);
    }
}
