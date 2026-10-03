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
fn chunked_value_writes_charge_only_new_byte_blocks_and_empty_attributes_still_cost_work() {
    let budget = SelectorWorkBudget::new(4);
    let mut buffer = ValueBuffer::new(1024, &budget);
    buffer.push(&"a".repeat(63)).unwrap();
    assert_eq!(budget.remaining(), 3);
    buffer.push("b").unwrap();
    assert_eq!(budget.remaining(), 3);
    buffer.push("c").unwrap();
    assert_eq!(budget.remaining(), 2);
    assert_eq!(buffer.finish().len(), 65);
    let html = scraper::Html::parse_document("<p data-empty=''></p>");
    let root = html
        .select(&scraper::Selector::parse("p").unwrap())
        .next()
        .unwrap();
    let budget = SelectorWorkBudget::new(2);
    let value = project(
        root,
        &ValueProjection::Attribute {
            name: "data-empty".into(),
        },
        &HashSet::new(),
        &[],
        None,
        1024,
        &budget,
    )
    .unwrap();
    assert_eq!(value, "");
    assert_eq!(budget.remaining(), 1);
}

#[test]
fn an_empty_markdown_destination_still_costs_one_processing_unit() {
    for (units, accepted) in [(1, false), (2, true)] {
        let budget = SelectorWorkBudget::new(units);
        let mut writer = super::markdown_writer::MarkdownWriter::new(4, &budget);
        let result = writer.destination("");
        assert_eq!(result.is_ok(), accepted);
        if accepted {
            assert_eq!(writer.finish().unwrap(), "<>");
            assert_eq!(budget.remaining(), 0);
        } else {
            assert_eq!(result.unwrap_err().code, ErrorCode::ResourceLimit);
        }
    }
}
