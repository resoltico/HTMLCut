use super::*;
use std::io::Write;

#[test]
fn html_buffer_flush_preserves_bytes_and_the_remaining_capacity() {
    let mut buffer = HtmlBuffer {
        bytes: Vec::new(),
        maximum: 4,
    };
    buffer.write_all(b"<br>").unwrap();
    buffer.flush().unwrap();
    assert!(buffer.write_all(b"x").is_err());
    assert_eq!(buffer.bytes, b"<br>");
}

#[test]
fn source_projection_cannot_accidentally_serialize_a_dom_element() {
    let html = scraper::Html::parse_document("<p>text</p>");
    let element = html
        .select(&scraper::Selector::parse("p").unwrap())
        .next()
        .unwrap();
    assert_eq!(
        project(
            element,
            &Projection::Source,
            &HashSet::new(),
            &[],
            None,
            1024,
            &SelectorWorkBudget::new(1000)
        )
        .unwrap_err()
        .code,
        ErrorCode::InternalInvariant
    );
}
