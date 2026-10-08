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

#[test]
fn direct_sink_operations_preserve_merging_reparenting_and_foster_boundaries() {
    let sink = BoundedSink::new(limits());
    let name = QualName::new(None, ns!(html), local_name!("div"));
    let root = sink.get_document();
    let parent = sink.create_element(name.clone(), Vec::new(), ElementFlags::default());
    sink.append(&root, NodeOrText::AppendNode(parent));
    sink.append(&parent, NodeOrText::AppendText(StrTendril::from("A")));
    sink.append(&parent, NodeOrText::AppendText(StrTendril::from("B")));
    let first = sink.create_element(name.clone(), Vec::new(), ElementFlags::default());
    sink.append(&parent, NodeOrText::AppendNode(first));
    sink.append_before_sibling(&first, NodeOrText::AppendText(StrTendril::from("C")));
    let second = sink.create_element(name.clone(), Vec::new(), ElementFlags::default());
    sink.append(&parent, NodeOrText::AppendNode(second));
    let child = sink.create_element(name.clone(), Vec::new(), ElementFlags::default());
    sink.append(&first, NodeOrText::AppendNode(child));
    sink.reparent_children(&first, &second);
    sink.add_attrs_if_missing(
        &second,
        vec![Attribute {
            name: QualName::new(None, ns!(), local_name!("id")),
            value: StrTendril::from("second"),
        }],
    );
    sink.append_based_on_parent_node(
        &first,
        &parent,
        NodeOrText::AppendText(StrTendril::from("D")),
    );
    let orphan = sink.create_element(name, Vec::new(), ElementFlags::default());
    sink.append_based_on_parent_node(
        &orphan,
        &second,
        NodeOrText::AppendText(StrTendril::from("E")),
    );
    sink.append_before_sibling(&orphan, NodeOrText::AppendText(StrTendril::from("ignored")));
    sink.remove_from_parent(&first);
    let document = sink.finish().unwrap();
    let attached = document
        .tree
        .root()
        .descendants()
        .filter(|node| node.value().is_element())
        .map(|node| node.id())
        .collect::<Vec<_>>();
    let html = crate::ElementRef::wrap(document.tree.get(parent).unwrap())
        .unwrap()
        .html();
    assert_eq!(html, "<div>ABCD<div id=\"second\"><div></div>E</div></div>");
    assert_eq!(attached.len(), 3);
}

#[test]
fn sink_overflow_and_attachment_work_fail_before_governed_growth() {
    let sink = BoundedSink::new(limits());
    sink.nodes.set(u32::MAX);
    assert!(!sink.allocate(1, 0));
    assert_eq!(sink.failure.get(), Some(ParseLimitExceeded::Nodes));
    let sink = BoundedSink::new(limits());
    sink.elements.set(u32::MAX);
    assert!(!sink.allocate(0, 1));
    assert_eq!(sink.failure.get(), Some(ParseLimitExceeded::Elements));
    let sink = BoundedSink::new(limits());
    let root = sink.get_document();
    let node = sink.create_element(
        QualName::new(None, ns!(html), local_name!("div")),
        Vec::new(),
        ElementFlags::default(),
    );
    sink.append(&root, NodeOrText::AppendNode(node));
    sink.remaining.set(0);
    assert!(sink.finish().is_ok()); // Finalization no longer constructs a redundant element index.
    let sink = BoundedSink::new(limits());
    let root = sink.get_document();
    let node = sink.create_element(
        QualName::new(None, ns!(html), local_name!("div")),
        Vec::new(),
        ElementFlags::default(),
    );
    sink.append(&root, NodeOrText::AppendNode(node));
    sink.remaining.set(1);
    assert!(!sink.attach(node, &NodeOrText::AppendText(StrTendril::from("x"))));
    assert_eq!(sink.failure.get(), Some(ParseLimitExceeded::Work));
}

#[test]
fn template_fragment_allocation_is_charged_even_without_the_flag_hint() {
    let mut policy = limits();
    policy.nodes = 3;
    let sink = BoundedSink::new(policy);
    let template = sink.create_element(
        QualName::new(None, ns!(html), local_name!("template")),
        Vec::new(),
        ElementFlags::default(),
    );
    assert_eq!(template, sink.sentinel);
    assert_eq!(sink.nodes.get(), 2);
    assert_eq!(sink.failure.get(), Some(ParseLimitExceeded::Nodes));
    let mut policy = limits();
    policy.nodes = 4;
    let sink = BoundedSink::new(policy);
    let template = sink.create_element(
        QualName::new(None, ns!(html), local_name!("template")),
        Vec::new(),
        ElementFlags::default(),
    );
    assert_ne!(template, sink.sentinel);
    assert_eq!(sink.nodes.get(), 4);
    assert_ne!(sink.get_template_contents(&template), sink.sentinel);
}

#[test]
fn special_nodes_and_first_failure_obey_construction_limits() {
    let sink = BoundedSink::new(limits());
    let pi = sink.create_pi("target".into(), "data".into());
    assert!(
        sink.inner
            .0
            .borrow()
            .tree
            .get(pi)
            .unwrap()
            .value()
            .as_processing_instruction()
            .is_some()
    );
    let sink = BoundedSink::new(ParseLimits {
        depth: 0,
        ..limits()
    });
    let before = sink.nodes.get();
    sink.append_doctype_to_document("html".into(), "".into(), "".into());
    assert_eq!(sink.failure.get(), Some(ParseLimitExceeded::Depth));
    assert_eq!(sink.nodes.get(), before);
    assert!(!sink.fail(ParseLimitExceeded::Nodes));
    assert_eq!(sink.failure.get(), Some(ParseLimitExceeded::Depth));
    assert_eq!(sink.finish().unwrap_err(), ParseLimitExceeded::Depth);
    assert_eq!(
        Html::parse_document_bounded(
            "<!doctype html>",
            ParseLimits {
                depth: 0,
                ..limits()
            }
        ),
        Err(ParseLimitExceeded::Depth)
    );
}

#[test]
fn foster_text_allocation_and_reparent_work_failure_leave_the_tree_unchanged() {
    let name = QualName::new(None, ns!(html), local_name!("div"));
    let sink = BoundedSink::new(limits());
    let parent = sink.create_element(name.clone(), Vec::new(), ElementFlags::default());
    let sibling = sink.create_element(name.clone(), Vec::new(), ElementFlags::default());
    sink.append(&sink.get_document(), NodeOrText::AppendNode(parent));
    sink.append(&parent, NodeOrText::AppendNode(sibling));
    sink.append_before_sibling(&sibling, NodeOrText::AppendText("foster".into()));
    assert_eq!(
        sink.inner
            .0
            .borrow()
            .tree
            .get(sibling)
            .unwrap()
            .prev_sibling()
            .unwrap()
            .value()
            .as_text()
            .unwrap()
            .text
            .as_ref(),
        "foster"
    );
    let sink = BoundedSink::new(ParseLimits {
        nodes: 3,
        ..limits()
    });
    let sibling = sink.create_element(name.clone(), Vec::new(), ElementFlags::default());
    sink.append(&sink.get_document(), NodeOrText::AppendNode(sibling));
    sink.append_before_sibling(&sibling, NodeOrText::AppendText("too many".into()));
    assert_eq!(sink.failure.get(), Some(ParseLimitExceeded::Nodes));
    assert_eq!(sink.nodes.get(), 3);
    for work in [0, 1, 2] {
        let sink = BoundedSink::new(limits());
        let first = sink.create_element(name.clone(), Vec::new(), ElementFlags::default());
        let second = sink.create_element(name.clone(), Vec::new(), ElementFlags::default());
        let child = sink.create_element(name.clone(), Vec::new(), ElementFlags::default());
        sink.append(&first, NodeOrText::AppendNode(child));
        sink.remaining.set(work);
        sink.reparent_children(&first, &second);
        assert_eq!(sink.failure.get(), Some(ParseLimitExceeded::Work));
        assert_eq!(
            sink.inner
                .0
                .borrow()
                .tree
                .get(child)
                .unwrap()
                .parent()
                .unwrap()
                .id(),
            first
        );
    }
}

#[test]
fn delegated_tree_depth_mismatch_fails_before_more_attachments() {
    let sink = BoundedSink::new(ParseLimits {
        depth: 1,
        ..limits()
    });
    let name = QualName::new(None, ns!(html), local_name!("div"));
    let parent = sink.create_element(name.clone(), Vec::new(), ElementFlags::default());
    let child = sink.create_element(name, Vec::new(), ElementFlags::default());
    sink.append(&sink.get_document(), NodeOrText::AppendNode(parent));
    // Simulate an upstream sink mutation bypassing the bounded wrapper. The
    // defensive ancestor check must fail rather than extend the invalid tree.
    sink.inner.append(&parent, NodeOrText::AppendNode(child));
    let nodes = sink.nodes.get();
    assert!(!sink.attach(child, &NodeOrText::AppendText("must not attach".into())));
    assert_eq!(sink.failure.get(), Some(ParseLimitExceeded::Depth));
    assert_eq!(sink.nodes.get(), nodes);
    assert_eq!(sink.finish().unwrap_err(), ParseLimitExceeded::Depth);
}

#[test]
fn late_attachment_work_exhaustion_and_doctype_failure_do_not_mutate() {
    let name = QualName::new(None, ns!(html), local_name!("div"));
    let sink = BoundedSink::new(limits());
    let sibling = sink.create_element(name.clone(), Vec::new(), ElementFlags::default());
    sink.append(&sink.get_document(), NodeOrText::AppendNode(sibling));
    sink.remaining.set(0);
    sink.append_before_sibling(&sibling, NodeOrText::AppendText("must not attach".into()));
    assert_eq!(sink.failure.get(), Some(ParseLimitExceeded::Work));
    assert!(
        sink.inner
            .0
            .borrow()
            .tree
            .get(sibling)
            .unwrap()
            .prev_sibling()
            .is_none()
    );
    let sink = BoundedSink::new(limits());
    let first = sink.create_element(name.clone(), Vec::new(), ElementFlags::default());
    let second = sink.create_element(name.clone(), Vec::new(), ElementFlags::default());
    let child = sink.create_element(name, Vec::new(), ElementFlags::default());
    sink.append(&first, NodeOrText::AppendNode(child));
    sink.remaining.set(3); // Child attachment check fits; the actual mutation does not.
    sink.reparent_children(&first, &second);
    assert_eq!(sink.failure.get(), Some(ParseLimitExceeded::Work));
    assert_eq!(
        sink.inner
            .0
            .borrow()
            .tree
            .get(child)
            .unwrap()
            .parent()
            .unwrap()
            .id(),
        first
    );
    let sink = BoundedSink::new(ParseLimits {
        nodes: 2,
        ..limits()
    });
    sink.append_doctype_to_document("html".into(), "".into(), "".into());
    assert_eq!(sink.failure.get(), Some(ParseLimitExceeded::Nodes));
    sink.append_doctype_to_document("html".into(), "".into(), "".into());
    assert_eq!(sink.nodes.get(), 2);
    assert_eq!(sink.finish().unwrap_err(), ParseLimitExceeded::Nodes);
}

#[test]
fn inserting_an_element_before_an_attached_sibling_preserves_attached_order() {
    let sink = BoundedSink::new(limits());
    let element = |tag| {
        sink.create_element(
            QualName::new(None, ns!(html), tag),
            Vec::new(),
            ElementFlags::default(),
        )
    };
    let parent = element(local_name!("div"));
    let first = element(local_name!("b"));
    let last = element(local_name!("i"));
    let middle = element(local_name!("span"));
    sink.append(&sink.get_document(), NodeOrText::AppendNode(parent));
    sink.append(&parent, NodeOrText::AppendNode(first));
    sink.append(&parent, NodeOrText::AppendNode(last));
    sink.append_before_sibling(&last, NodeOrText::AppendNode(middle));
    let document = sink.finish().unwrap();
    let attached = document
        .tree
        .root()
        .descendants()
        .filter(|node| node.value().is_element())
        .map(|node| node.id())
        .collect::<Vec<_>>();
    assert_eq!(attached, [parent, first, middle, last]);
    assert_eq!(
        crate::ElementRef::wrap(document.tree.get(parent).unwrap())
            .unwrap()
            .html(),
        "<div><b></b><span></span><i></i></div>"
    );
}

#[test]
fn bounded_sink_charges_warnings_without_retaining_them_and_keeps_quirks_metadata() {
    let sink = BoundedSink::new(limits());
    sink.set_quirks_mode(QuirksMode::LimitedQuirks);
    let remaining = sink.remaining.get();
    sink.parse_error(Cow::Borrowed("synthetic parser diagnostic"));
    assert_eq!(sink.remaining.get(), remaining - 1);
    let document = sink.finish().unwrap();
    assert_eq!(document.quirks_mode, QuirksMode::LimitedQuirks);
    #[cfg(feature = "errors")]
    assert!(document.errors.is_empty());
    let sink = BoundedSink::new(ParseLimits {
        work: 0,
        ..limits()
    });
    sink.parse_error(Cow::Borrowed("must be bounded"));
    assert_eq!(sink.failure.get(), Some(ParseLimitExceeded::Work));
    assert_eq!(sink.finish().unwrap_err(), ParseLimitExceeded::Work);
}

#[test]
fn fixed_node_floor_is_preflighted_before_parser_work_and_two_nodes_are_not_enough() {
    for (nodes, work, expected) in [
        (0, 0, ParseLimitExceeded::Nodes),
        (1, 0, ParseLimitExceeded::Nodes),
        (2, 0, ParseLimitExceeded::Work),
        (2, 1, ParseLimitExceeded::Work),
        (2, 100, ParseLimitExceeded::Nodes),
    ] {
        assert_eq!(
            Html::parse_document_bounded(
                "",
                ParseLimits {
                    nodes,
                    work,
                    ..limits()
                }
            )
            .unwrap_err(),
            expected
        );
    }
}

#[test]
fn reparenting_a_wide_subtree_at_exact_depth_preserves_sibling_height() {
    let sink = BoundedSink::new(ParseLimits {
        depth: 3,
        ..limits()
    });
    let element = |tag| {
        sink.create_element(
            QualName::new(None, ns!(html), tag),
            Vec::new(),
            ElementFlags::default(),
        )
    };
    let origin = element(local_name!("div"));
    let target = element(local_name!("section"));
    let branch = element(local_name!("b"));
    let first = element(local_name!("i"));
    let second = element(local_name!("span"));
    sink.append(&sink.get_document(), NodeOrText::AppendNode(origin));
    sink.append(&sink.get_document(), NodeOrText::AppendNode(target));
    sink.append(&origin, NodeOrText::AppendNode(branch));
    sink.append(&branch, NodeOrText::AppendNode(first));
    sink.append(&branch, NodeOrText::AppendNode(second));
    sink.reparent_children(&origin, &target);
    let document = sink.finish().unwrap();
    let attached = document
        .tree
        .root()
        .descendants()
        .filter(|node| node.value().is_element())
        .map(|node| node.id())
        .collect::<Vec<_>>();
    assert_eq!(attached, [origin, target, branch, first, second]);
    assert_eq!(
        crate::ElementRef::wrap(document.tree.get(target).unwrap())
            .unwrap()
            .html(),
        "<section><b><i></i><span></span></b></section>"
    );
}

#[test]
fn script_bookkeeping_callbacks_are_charged_even_when_scripts_are_not_executed() {
    let sink = BoundedSink::new(limits());
    let script = sink.create_element(
        QualName::new(None, ns!(html), local_name!("script")),
        Vec::new(),
        ElementFlags::default(),
    );
    sink.remaining.set(1);
    sink.mark_script_already_started(&script);
    assert_eq!(sink.remaining.get(), 0);
    assert!(!sink.stop_requested());
    sink.mark_script_already_started(&script);
    assert_eq!(sink.failure.get(), Some(ParseLimitExceeded::Work));
    assert_eq!(sink.finish().unwrap_err(), ParseLimitExceeded::Work);
}

#[test]
fn ancestor_depth_check_accepts_the_existing_exact_depth() {
    let sink = BoundedSink::new(ParseLimits {
        depth: 1,
        ..limits()
    });
    let element = sink.create_element(
        QualName::new(None, ns!(html), local_name!("div")),
        Vec::new(),
        ElementFlags::default(),
    );
    sink.append(&sink.get_document(), NodeOrText::AppendNode(element));
    assert_eq!(sink.parent_depth(element), Some(1));
    assert!(!sink.stop_requested());
}

#[test]
fn adoption_counterexample_rejects_at_three_elements_and_accepts_at_four() {
    assert_eq!(
        Html::parse_document_bounded(
            "<i></i>",
            ParseLimits {
                elements: 3,
                ..limits()
            }
        ),
        Err(ParseLimitExceeded::Elements),
    );
    let parsed = Html::parse_document_bounded(
        "<i></i>",
        ParseLimits {
            elements: 4,
            ..limits()
        },
    )
    .unwrap();
    assert_eq!(
        parsed.html(),
        "<html><head></head><body><i></i></body></html>"
    );
}

#[test]
fn authored_current_token_and_eof_cases_refuse_without_returning_a_partial_tree() {
    // Each source forces a distinct tree-builder path. These are fixed authored
    // rejection controls, including unfinished tokens emitted during EOF.
    for source in [
        "<b><i>X</b>Y</i>",
        "<template><b>X</b></template>",
        "<table>X<tr><td>Y</table>",
        "<svg><foreignObject><p>X</p></foreignObject></svg>",
        "<main><!--é unfinished comment",
        "<script>é unfinished raw data",
        "<main>é unfinished text",
    ] {
        for (policy, expected) in [
            (
                ParseLimits {
                    elements: 3,
                    ..limits()
                },
                ParseLimitExceeded::Elements,
            ),
            (
                ParseLimits {
                    nodes: 2,
                    ..limits()
                },
                ParseLimitExceeded::Nodes,
            ),
            (
                ParseLimits {
                    depth: 1,
                    ..limits()
                },
                ParseLimitExceeded::Depth,
            ),
            (
                ParseLimits {
                    work: 1,
                    ..limits()
                },
                ParseLimitExceeded::Work,
            ),
        ] {
            assert_eq!(
                Html::parse_document_bounded(source, policy),
                Err(expected),
                "{source}"
            );
        }
    }
}

#[test]
fn unfinished_utf8_text_comment_and_script_tokens_preserve_complete_values() {
    let selector = |name| crate::Selector::parse(name).unwrap();
    let text = Html::parse_document_bounded("<main>é unfinished text", limits()).unwrap();
    assert_eq!(
        text.select(&selector("main"))
            .next()
            .unwrap()
            .text()
            .collect::<String>(),
        "é unfinished text"
    );
    let script = Html::parse_document_bounded("<script>é unfinished raw data", limits()).unwrap();
    assert_eq!(
        script
            .select(&selector("script"))
            .next()
            .unwrap()
            .text()
            .collect::<String>(),
        "é unfinished raw data"
    );
    let comment = Html::parse_document_bounded("<main><!--é unfinished comment", limits()).unwrap();
    assert_eq!(
        comment
            .select(&selector("main"))
            .next()
            .unwrap()
            .inner_html(),
        "<!--é unfinished comment-->"
    );
}

#[test]
fn refusal_during_adoption_template_foster_foreign_and_eof_allocation_is_safe() {
    for (source, policy, expected) in [
        (
            "<b><p>X</b>Y",
            ParseLimits {
                elements: 5,
                ..limits()
            },
            ParseLimitExceeded::Elements,
        ),
        (
            "<template>",
            ParseLimits {
                nodes: 5,
                ..limits()
            },
            ParseLimitExceeded::Nodes,
        ),
        (
            "<table>X",
            ParseLimits {
                nodes: 6,
                ..limits()
            },
            ParseLimitExceeded::Nodes,
        ),
        (
            "<svg><g>",
            ParseLimits {
                elements: 4,
                ..limits()
            },
            ParseLimitExceeded::Elements,
        ),
        (
            "<main><!--é",
            ParseLimits {
                nodes: 6,
                ..limits()
            },
            ParseLimitExceeded::Nodes,
        ),
        (
            "<script>é",
            ParseLimits {
                nodes: 5,
                ..limits()
            },
            ParseLimitExceeded::Nodes,
        ),
        (
            "<main>é",
            ParseLimits {
                nodes: 6,
                ..limits()
            },
            ParseLimitExceeded::Nodes,
        ),
        (
            "<main>é",
            ParseLimits {
                depth: 3,
                ..limits()
            },
            ParseLimitExceeded::Depth,
        ),
    ] {
        assert_eq!(
            Html::parse_document_bounded(source, policy),
            Err(expected),
            "{source}"
        );
    }
}
