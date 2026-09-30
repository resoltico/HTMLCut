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
