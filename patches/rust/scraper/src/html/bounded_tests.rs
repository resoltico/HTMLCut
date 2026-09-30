use super::*;

fn limits() -> ParseLimits {
    ParseLimits {
        elements: 100,
        nodes: 1000,
        depth: 100,
        work: 100_000,
    }
}

#[test]
fn constructed_elements_nodes_and_depth_have_exact_boundaries() {
    let source = "<p>A</p>";
    let mut policy = limits();
    for (elements, expected) in [(3, false), (4, true), (5, true)] {
        policy.elements = elements;
        assert_eq!(
            Html::parse_document_bounded(source, policy).is_ok(),
            expected
        );
    }
    policy = limits();
    for (nodes, expected) in [(6, false), (7, true), (8, true)] {
        policy.nodes = nodes;
        assert_eq!(
            Html::parse_document_bounded(source, policy).is_ok(),
            expected
        );
    }
    policy = limits();
    for (depth, expected) in [(3, false), (4, true), (5, true)] {
        policy.depth = depth;
        assert_eq!(
            Html::parse_document_bounded(source, policy).is_ok(),
            expected
        );
    }
}

#[test]
fn first_failure_is_sticky_and_allocation_stops_before_an_adversarial_tail() {
    let mut policy = limits();
    policy.elements = 1;
    let sink = BoundedSink::new(policy);
    let html = QualName::new(None, ns!(html), local_name!("html"));
    let first = sink.create_element(html.clone(), Vec::new(), ElementFlags::default());
    assert_ne!(first, sink.sentinel);
    assert_eq!(
        sink.create_element(html.clone(), Vec::new(), ElementFlags::default()),
        sink.sentinel
    );
    for _ in 0..100 {
        assert_eq!(
            sink.create_element(html.clone(), Vec::new(), ElementFlags::default()),
            sink.sentinel
        );
        sink.create_comment(StrTendril::from("comment"));
        sink.create_pi(StrTendril::from("target"), StrTendril::from("data"));
        sink.parse_error(Cow::Borrowed("error"));
        sink.set_quirks_mode(QuirksMode::Quirks);
        sink.append(&first, NodeOrText::AppendText(StrTendril::from("tail")));
        sink.add_attrs_if_missing(&first, Vec::new());
        sink.remove_from_parent(&first);
        sink.reparent_children(&first, &sink.get_document());
        sink.append_before_sibling(&first, NodeOrText::AppendNode(sink.sentinel));
        sink.append_based_on_parent_node(&first, &first, NodeOrText::AppendNode(sink.sentinel));
        sink.mark_script_already_started(&first);
    }
    assert_eq!(sink.elements.get(), 1);
    assert_eq!(sink.nodes.get(), 3);
    assert_eq!(sink.get_template_contents(&first), sink.sentinel);
    assert_eq!(sink.finish(), Err(ParseLimitExceeded::Elements));
    let huge_tail = format!("<p>{}", "<div>x</div>".repeat(10_000));
    assert_eq!(
        Html::parse_document_bounded(&huge_tail, policy),
        Err(ParseLimitExceeded::Elements)
    );
}

#[test]
fn template_fragments_comments_text_merging_and_foster_parenting_are_accounted() {
    let source = "<!doctype html><!--before--><main>A&amp;B<template>T<span>M</span></template><table>outside<tr><td>cell</td></tr></table><b><i>misnested</b>tail</i></main>";
    let ordinary = Html::parse_document(source);
    let bounded = Html::parse_document_bounded(source, limits()).unwrap();
    assert_eq!(bounded.html(), ordinary.html());
    let template = bounded
        .select(&crate::Selector::parse("template").unwrap())
        .next()
        .unwrap();
    assert_eq!(template.text().collect::<String>(), "TM");
    let mut policy = limits();
    policy.nodes = 2;
    assert_eq!(
        Html::parse_document_bounded("<!--comment-->", policy),
        Err(ParseLimitExceeded::Nodes)
    );
    assert_eq!(
        Html::parse_document_bounded("", policy),
        Err(ParseLimitExceeded::Nodes)
    );
    policy.nodes = 1;
    assert_eq!(
        Html::parse_document_bounded("", policy),
        Err(ParseLimitExceeded::Nodes)
    );
}

#[test]
fn attachment_reparenting_work_is_finite_and_deep_trees_do_not_recurse() {
    let mut policy = limits();
    policy.work = 1;
    assert_eq!(
        Html::parse_document_bounded("<p>x</p>", policy),
        Err(ParseLimitExceeded::Work)
    );
    let source = format!("{}x{}", "<div>".repeat(2048), "</div>".repeat(2048));
    policy = ParseLimits {
        elements: 3000,
        nodes: 4000,
        depth: 2051,
        work: 10_000_000,
    };
    assert!(Html::parse_document_bounded(&source, policy).is_ok());
    policy.depth = 100;
    assert_eq!(
        Html::parse_document_bounded(&source, policy),
        Err(ParseLimitExceeded::Depth)
    );
}
