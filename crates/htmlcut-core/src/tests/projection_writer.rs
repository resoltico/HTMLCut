// SPDX-License-Identifier: MPL-2.0
use super::*;
use std::io::Write;

#[test]
fn html_buffer_flush_preserves_bytes_and_the_remaining_capacity() {
    let mut buffer = HtmlBuffer {
        bytes: Vec::new(),
        maximum: 4,
        limit_exceeded: false,
    };
    buffer.write_all(b"<br>").unwrap();
    buffer.flush().unwrap();
    assert!(buffer.write_all(b"x").is_err());
    assert_eq!(buffer.bytes, b"<br>");
}

#[test]
fn chunked_value_writes_charge_only_new_byte_blocks_and_empty_attributes_still_cost_work() {
    let html = crate::dom::Html::parse_document("<p data-empty=''></p>");
    let root = html
        .select(&crate::dom::Selector::parse("p").unwrap())
        .next()
        .unwrap();
    let budget = WorkBudget::new(2);
    let value = project(
        root,
        &Reading::Attribute("data-empty".into()),
        &HashSet::new(),
        None,
        1024,
        &budget,
    )
    .unwrap();
    assert_eq!(value, "");
    assert_eq!(budget.remaining(), 1);
}

#[test]
fn filtered_html_work_exhaustion_is_distinct_from_the_value_byte_bound() {
    let document = crate::dom::Html::parse_document("<p>A</p>");
    let root = document
        .select(&crate::dom::Selector::parse("p").unwrap())
        .next()
        .unwrap();
    let budget = WorkBudget::new(1);
    let error = project(
        root,
        &Reading::OuterHtml,
        &HashSet::new(),
        None,
        1024,
        &budget,
    )
    .unwrap_err();
    assert_eq!(
        (error.resource_counter.as_deref(), error.configured_bound),
        (Some("max_work"), Some(1))
    );
}

#[test]
fn literal_input_and_output_each_charge_only_their_new_utf8_byte_blocks() {
    for value in [
        "a".repeat(63),
        "a".repeat(64),
        "a".repeat(65),
        format!("{}é", "a".repeat(63)),
    ] {
        let html = crate::dom::Html::parse_document(&format!("<p>{value}</p>"));
        let root = html
            .select(&crate::dom::Selector::parse("p").unwrap())
            .next()
            .unwrap();
        let budget = WorkBudget::new(1000);
        let (actual, complete) = text(root, &HashSet::new(), false, 1024, None, &budget).unwrap();
        assert_eq!(actual, value);
        assert!(complete);
        // Open/close p and its one text node, plus independent input and output byte blocks.
        assert_eq!(
            1000 - budget.remaining(),
            4 + 2 * value.as_bytes().chunks(64).len() as u32
        );
    }
}

#[test]
fn structural_boundary_before_nonwhitespace_pre_content_is_retained() {
    let html = crate::dom::Html::parse_document(
        "<article><p>before</p><pre>code</pre><p>after</p></article>",
    );
    let root = html
        .select(&crate::dom::Selector::parse("article").unwrap())
        .next()
        .unwrap();
    let (actual, complete) = text(
        root,
        &HashSet::new(),
        true,
        1024,
        None,
        &WorkBudget::new(1000),
    )
    .unwrap();
    assert_eq!(actual, "before code after");
    assert!(complete);
}
