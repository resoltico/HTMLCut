use super::*;

const PREPARED_ELEMENT_LIMIT: u32 = 1_000_000;

fn million_element_document() -> String {
    let attributes = [
        "id=\"shared\"",
        "itemprop=\"mainEntity\"",
        "role=\"main\"",
        "aria-label=\"Primary target\"",
        "data-01=\"one\"",
        "data-02=\"two\"",
        "data-03=\"three\"",
        "data-04=\"four\"",
        "data-05=\"five\"",
        "data-06=\"six\"",
        "data-07=\"seven\"",
        "data-08=\"eight\"",
        "data-09=\"nine\"",
        "data-10=\"ten\"",
        "class=\"primary-content\"",
    ]
    .join(" ");
    // The parser supplies the document shell; the source reserves one explicit duplicate-ID node
    // and one tail node so the prepared document reaches the published one-million-element limit.
    let span_count = usize::try_from(PREPARED_ELEMENT_LIMIT - 6).expect("published limit fits");
    let mut html = String::with_capacity(256 + span_count.saturating_mul("<span></span>".len()));
    html.push_str(&format!("<html {attributes}><body>xx<div>"));
    for _ in 0..span_count {
        html.push_str("<span></span>");
    }
    html.push_str("<div id=\"shared\"></div></div><span id=\"tail\"></span>");
    html.push_str("</body></html>");
    html
}

#[test]
#[ignore = "resource acceptance: exercises one million prepared elements"]
fn million_element_page_exhaustion_advances_and_a_tail_page_terminates() {
    let document = prepare_document(
        HtmlInput::new("million-elements", million_element_document()).expect("source"),
        PreparationLimits::new(
            NonZeroU32::new(50 * 1024 * 1024).expect("non-zero"),
            NonZeroU32::new(PREPARED_ELEMENT_LIMIT).expect("non-zero"),
            NonZeroU32::new(4_096).expect("non-zero"),
        )
        .expect("published preparation limits"),
    )
    .expect("one-million-element document");
    assert_eq!(document.element_count(), PREPARED_ELEMENT_LIMIT);

    let options = ExplorationOptions {
        cursor: None,
        max_elements: NonZeroU32::new(1_000).expect("non-zero"),
        max_work_units: NonZeroU32::new(3_100_000).expect("non-zero"),
        max_proposals_per_element: NonZeroU32::new(16).expect("non-zero"),
        preview_bytes: NonZeroU32::new(1).expect("non-zero"),
    };
    let first_page = explore(&document, &options).expect("first page");
    let target = first_page
        .elements
        .first()
        .expect("first descriptor")
        .clone();
    assert_eq!(target.element_name.local_name, "html");
    assert!(target.proposal_work_truncated);
    assert!(target.selector_proposals.is_empty());
    assert_eq!(
        first_page.truncation_reason,
        Some(ExplorationTruncationReason::WorkLimit)
    );
    assert_eq!(
        first_page
            .next_cursor
            .as_ref()
            .expect("continuation after consumed target")
            .next_document_ordinal(),
        NonZeroU32::new(2).expect("non-zero")
    );

    let tail_options = ExplorationOptions {
        cursor: Some(crate::interop::v2::exploration_cursor_for_tests(
            &document,
            &options,
            NonZeroU32::new(PREPARED_ELEMENT_LIMIT).expect("non-zero"),
        )),
        ..options
    };
    let tail_page = explore(&document, &tail_options).expect("tail page");
    assert_eq!(tail_page.elements.len(), 1);
    assert_eq!(tail_page.next_cursor, None);
    assert_eq!(tail_page.truncation_reason, None);
}
