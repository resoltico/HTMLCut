// SPDX-License-Identifier: MPL-2.0
//! Public snapshot-bound group discovery and evidence limits.

use htmlcut_core::{
    ErrorCode, PreparationLimits, PreparedDocument, SnapshotMetadata, SourceSnapshot,
};

fn document(source: &str) -> PreparedDocument {
    PreparedDocument::new(
        SourceSnapshot::new(source, SnapshotMetadata::default()).unwrap(),
        PreparationLimits::default(),
    )
    .unwrap()
}

#[test]
fn a_regular_header_table_reports_rows_without_inventing_a_field_plan() {
    let source = "<table id=population class=wikitable><tr><th>Location</th><th>Population</th></tr><tr><td>India</td><td>1</td></tr><tr><td>China</td><td>2</td></tr><tr></tr></table>";
    let page = document(source);
    let result = page.outline(None, 8).unwrap();
    assert_eq!(result.source_sha256, page.snapshot().source_sha256());
    assert_eq!(result.group_count, 1);
    assert!(result.groups_complete);
    let group = &result.groups[0];
    assert_eq!(group.count, 4);
    assert_eq!(group.selector.as_deref(), Some("#population tr"));
    assert_eq!(group.item_tag, "tr");
    assert_eq!(group.samples[0].text, "Location Population");
    assert_eq!(group.samples[1].text, "India 1");
    let table = group.table.as_ref().unwrap();
    assert_eq!(table.headers, ["Location", "Population"]);
    assert!(table.headers_complete && table.headers_unique);
    assert_eq!(
        (table.header_rows, table.data_rows, table.other_rows),
        (1, 2, 1)
    );
    assert_eq!(
        (table.min_data_cells, table.max_data_cells),
        (Some(2), Some(2))
    );
    assert!(!table.spans_present);
    assert!(
        htmlcut_core::schema("htmlcut.outline").unwrap()["properties"]
            .get("groups")
            .is_some()
    );
}

#[test]
fn scoped_outline_finds_content_after_denser_navigation_without_claiming_completeness() {
    let navigation = (0..12)
        .map(|i| {
            format!(
                "<ul class=nav{i}>{}</ul>",
                "<li>Navigation</li>".repeat(20 + i)
            )
        })
        .collect::<String>();
    let source = format!(
        "<body>{navigation}<section id=content><div class=cards>{}</div></section></body>",
        "<article><h2>Target</h2></article>".repeat(10)
    );
    let page = document(&source);
    let broad = page.outline(None, 8).unwrap();
    assert!(broad.group_count > 8);
    assert!(!broad.groups_complete);
    assert!(broad.groups.iter().all(|group| group.item_tag == "li"));
    let focused = page.outline(Some("#content"), 8).unwrap();
    assert_eq!(focused.group_count, 1);
    assert!(focused.groups_complete);
    assert_eq!(focused.groups[0].count, 10);
    assert_eq!(focused.groups[0].item_tag, "article");
    assert_eq!(
        focused.groups[0].selector.as_deref(),
        Some("#content article")
    );
}

#[test]
fn equal_counts_do_not_make_a_wrong_selector_a_verified_hint() {
    let page = document(
        "<div class=shared><li>A</li><li>B</li><li>C</li></div><div class=shared><li>D</li><li>E</li><li>F</li></div>",
    );
    let result = page.outline(None, 8).unwrap();
    let groups = result
        .groups
        .iter()
        .filter(|group| group.item_tag == "li")
        .collect::<Vec<_>>();
    assert_eq!(groups.len(), 2);
    assert!(
        groups
            .iter()
            .all(|group| group.count == 3 && group.selector.is_none())
    );
    assert_ne!(groups[0].samples[0].text, groups[1].samples[0].text);
}

#[test]
fn class_scopes_and_duplicate_class_tokens_keep_one_exact_group() {
    let table = document(
        "<table class=post-list><tr><td>First</td></tr><tr><td>Second</td></tr><tr><td>Third</td></tr></table>",
    );
    let result = table.outline(None, 8).unwrap();
    assert_eq!(
        result.groups[0].selector.as_deref(),
        Some("table.post-list tr")
    );
    assert_eq!(result.groups[0].table.as_ref().unwrap().data_rows, 3);

    let classes = (0..8)
        .map(|i| format!("c{i}"))
        .collect::<Vec<_>>()
        .join(" ");
    let source = format!(
        "<ol>{}</ol>",
        format!("<li class='{classes} c0'>Item</li>").repeat(3)
    );
    let result = document(&source).outline(None, 8).unwrap();
    assert_eq!(result.group_count, 1);
    assert!(result.groups[0].class_constrained);
    assert_eq!(result.groups[0].item_classes.len(), 8);
    assert!(result.groups[0].selector.is_some());
}

#[test]
fn irregular_tables_report_ambiguity_instead_of_a_column_mapping() {
    let page = document(
        "<table><tr><th>A</th><th>A</th></tr><tr><td colspan=2>x</td></tr><tr><td>y</td><td>z</td></tr></table>",
    );
    let outline = page.outline(None, 8).unwrap();
    let table = outline
        .groups
        .iter()
        .find_map(|group| group.table.as_ref())
        .unwrap();
    assert_eq!(table.headers, ["A", "A"]);
    assert!(!table.headers_unique);
    assert_eq!(
        (table.min_data_cells, table.max_data_cells),
        (Some(1), Some(2))
    );
    assert!(table.spans_present);
}

#[test]
fn long_or_multiple_header_rows_are_labeled_incomplete() {
    for header_rows in [
        format!("<tr><th>{}</th><th>B</th></tr>", "x".repeat(129)),
        "<tr><th>A</th><th>B</th></tr><tr><th>A</th><th>B</th></tr>".into(),
        format!("<tr>{}</tr>", "<th>H</th>".repeat(9)),
    ] {
        let source = format!(
            "<table>{header_rows}<tr><td>a</td><td>b</td></tr><tr><td>c</td><td>d</td></tr></table>"
        );
        let page = document(&source);
        let table = page
            .outline(None, 8)
            .unwrap()
            .groups
            .into_iter()
            .find_map(|group| group.table)
            .unwrap();
        assert!(!table.headers_complete);
        assert!(table.headers.is_empty());
        assert!(!table.headers_unique);
    }
}

#[test]
fn outline_refuses_an_oversized_encoded_answer() {
    let classes = (0..8)
        .map(|i| format!("c{i}{}", "x".repeat(62)))
        .collect::<Vec<_>>()
        .join(" ");
    let source = (0..16)
        .map(|i| {
            let id = format!("p{i:02}{}", "x".repeat(125));
            format!(
                "<section id='{id}'>{}</section>",
                format!(
                    "<article class='{classes}'>{}</article>",
                    "\u{1}".repeat(64)
                )
                .repeat(3)
            )
        })
        .collect::<String>();
    let page = document(&source);
    assert!(page.outline(None, 1).is_ok());
    assert_eq!(
        page.outline(None, 16).unwrap_err().code,
        ErrorCode::ResourceLimit
    );
}

#[test]
fn outline_rejects_ambiguous_scopes_and_excess_signatures_without_partial_success() {
    let page = document("<section class=scope></section><section class=scope></section>");
    assert_eq!(
        page.outline(Some(".missing"), 8).unwrap_err().code,
        ErrorCode::NoMatch
    );
    assert_eq!(
        page.outline(Some(".scope"), 8).unwrap_err().code,
        ErrorCode::AmbiguousSelection
    );
    assert_eq!(
        page.outline(None, 0).unwrap_err().code,
        ErrorCode::InvalidOptions
    );
    assert_eq!(
        page.outline(Some("["), 8).unwrap_err().code,
        ErrorCode::InvalidSelector
    );

    let unique = (0..1_024)
        .map(|i| format!("<p class=c{i}>x</p>"))
        .collect::<String>();
    let page = document(&format!("<div>{unique}</div>"));
    assert_eq!(
        page.outline(None, 8).unwrap_err().code,
        ErrorCode::ResourceLimit
    );
}

#[test]
fn unusual_tags_and_identifier_limits_keep_hints_and_descriptors_honest() {
    let colon = document("<div><x:y>A</x:y><x:y>B</x:y><x:y>C</x:y></div>");
    let group = &colon.outline(None, 4).unwrap().groups[0];
    assert_eq!(group.item_tag, "x:y");
    assert!(group.selector.is_none());

    let long_tag = "x".repeat(129);
    let page = document(&format!("<div><{long_tag}></{long_tag}></div>"));
    assert_eq!(
        page.outline(None, 4).unwrap_err().code,
        ErrorCode::ResourceLimit
    );

    for classes in [
        "x".repeat(65),
        (0..9)
            .map(|i| format!("c{i}"))
            .collect::<Vec<_>>()
            .join(" "),
    ] {
        let source = format!(
            "<section id='{}' class='{classes}'>{}</section>",
            "p".repeat(129),
            "<p>A</p><p>B</p><p>C</p>"
        );
        let group = &document(&source).outline(None, 4).unwrap().groups[0];
        assert_eq!(group.count, 3);
        assert!(group.parent.id.is_none());
        assert!(group.parent.classes.is_empty());
        assert!(!group.parent.identifiers_complete);
    }
}

#[test]
fn mixed_table_children_and_blank_headers_are_reported_without_column_claims() {
    let page = document(
        "<table><tbody><tr> <script></script><th>A</th><th></th></tr><tr><th>mixed</th><td>0</td></tr><tr><td rowspan=2>1</td></tr><tr><td>2</td></tr></tbody></table>",
    );
    let group = page
        .outline(None, 4)
        .unwrap()
        .groups
        .into_iter()
        .find(|group| group.item_tag == "tr")
        .unwrap();
    let table = group.table.unwrap();
    assert!(!table.headers_unique);
    assert_eq!(table.data_rows, 2);
    assert_eq!(table.other_rows, 1);
    assert!(table.spans_present);
}

#[test]
fn selector_candidate_limits_refuse_oversized_global_matches() {
    let source = format!("<ul>{}</ul>", "<li class=item>x</li>".repeat(100_001));
    let page = document(&source);
    assert_eq!(
        page.outline(Some("li"), 1).unwrap_err().code,
        ErrorCode::ResourceLimit
    );
    assert_eq!(
        page.outline(None, 1).unwrap_err().code,
        ErrorCode::ResourceLimit
    );
}

#[test]
fn mixed_members_and_ties_preserve_counts_and_document_order() {
    let page = document(
        "<main><ul id=first> <li class=a>A</li> <p>gap</p> <li class=a>B</li> <li class=a>C</li> <li class=b>D</li> <li class=b>E</li> <li class=b>F</li> </ul><ul id=second><li>G</li><li>H</li><li>I</li></ul></main>",
    );
    let groups = page.outline(None, 2).unwrap();
    assert!(groups.group_count >= 3);
    assert_eq!(groups.groups[0].parent.id.as_deref(), Some("first"));
    assert_eq!(groups.groups[1].parent.id.as_deref(), Some("first"));
    assert_eq!(groups.groups[0].count, 6);
    assert_eq!(groups.groups[1].count, 3);
    let class_group = groups
        .groups
        .iter()
        .find(|group| group.item_classes == ["a"])
        .unwrap();
    assert_eq!(class_group.selector.as_deref(), Some("li.a"));
}

#[test]
fn a_later_larger_group_evicts_the_later_of_equal_sized_candidates() {
    let page = document(
        "<main><ul id=first><li class=a>A</li><li class=a>B</li><li class=a>C</li><li class=b>D</li><li class=b>E</li><li class=b>F</li></ul><ul id=second><li>G</li><li>H</li><li>I</li><li>J</li></ul></main>",
    );
    let outline = page.outline(None, 3).unwrap();
    assert_eq!(outline.group_count, 4);
    assert_eq!(
        outline
            .groups
            .iter()
            .map(|group| group.count)
            .collect::<Vec<_>>(),
        [6, 4, 3]
    );
    assert_eq!(outline.groups[2].item_classes, ["a"]);
}

#[test]
fn nested_exclusions_and_foreign_elements_do_not_create_groups() {
    let page = document(
        "<body><template><div><p>a</p><p>b</p><p>c</p></div></template><pre><div><p>a</p><p>b</p><p>c</p></div><section><p>d</p><p>e</p><p>f</p></section></pre><svg><g><circle/><circle/><circle/></g></svg><section><p>yes</p><p>yes</p><p>yes</p></section></body>",
    );
    let groups = page.outline(None, 4).unwrap();
    assert_eq!(groups.group_count, 1);
    assert_eq!(groups.groups[0].parent.tag, "section");
}

#[test]
fn the_signature_limit_also_applies_to_a_new_plain_tag() {
    let classes = (0..1_023)
        .map(|i| format!("<p class=c{i}>x</p>"))
        .collect::<String>();
    let page = document(&format!("<div>{classes}<aside>x</aside></div>"));
    assert_eq!(
        page.outline(None, 4).unwrap_err().code,
        ErrorCode::ResourceLimit
    );
}

#[test]
fn unsafe_css_identifiers_are_never_put_in_selector_hints() {
    let page = document(
        "<x:y id=1bad><li class='a:b'>A</li><li class='a:b'>B</li><li class='a:b'>C</li></x:y>",
    );
    let groups = page.outline(None, 4).unwrap();
    let group = groups
        .groups
        .iter()
        .find(|group| group.item_tag == "li")
        .unwrap();
    assert_eq!(group.count, 3);
    assert_eq!(group.selector.as_deref(), Some("li"));
}

#[test]
fn identifier_and_header_byte_caps_accept_their_exact_boundaries() {
    let class = format!("c{}", "x".repeat(63));
    let page = document(&format!(
        "<ul>{}</ul>",
        format!("<li class='{class}'>x</li>").repeat(3)
    ));
    let group = &page.outline(None, 4).unwrap().groups[0];
    assert!(group.class_constrained);
    assert_eq!(group.item_classes, [class]);

    let header = "H".repeat(128);
    let page = document(&format!(
        "<table><tr><th>{header}</th></tr><tr><td>a</td></tr><tr><td>b</td></tr></table>"
    ));
    let table = page.outline(None, 4).unwrap().groups[0]
        .table
        .clone()
        .unwrap();
    assert!(table.headers_complete);
    assert_eq!(table.headers, [header]);
}

#[test]
fn the_tag_byte_cap_accepts_exactly_128_bytes() {
    let tag = "x".repeat(128);
    let page = document(&format!(
        "<div>{}</div>",
        format!("<{tag}>x</{tag}>").repeat(3)
    ));
    let outline = page.outline(None, 4).unwrap();
    assert_eq!(outline.group_count, 1);
    assert_eq!(outline.groups[0].item_tag, tag);
}

#[test]
fn equal_sized_groups_in_separate_parents_follow_document_order() {
    let padding = "<!--x-->".repeat(100);
    let page = document(&format!(
        "<main><section id=first>{padding}<p>A</p><p>B</p><p>C</p></section><section id=second><p>D</p><p>E</p><p>F</p></section></main>"
    ));
    let outline = page.outline(None, 1).unwrap();
    assert_eq!(outline.group_count, 2);
    assert_eq!(outline.groups[0].parent.id.as_deref(), Some("first"));
}
