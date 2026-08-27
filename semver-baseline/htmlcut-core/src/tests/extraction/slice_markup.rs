use super::*;

#[test]
fn slice_finders_cover_literal_regex_and_empty_reader_edges() {
    let literal = build_finder("<p>", PatternMode::Literal, None).expect("literal finder");
    assert_eq!(
        literal.find("<p>Hello</p>", 0).expect("literal hit").start,
        0
    );
    assert!(literal.find("<p>Hello</p>", 10).is_none());

    let regex = build_finder(r"h\w+", PatternMode::Regex, Some("i")).expect("regex finder");
    assert_eq!(regex.find("Hello", 0).expect("regex hit").start, 0);
    assert!(regex.find("Hello", 5).is_none());

    let mut empty = Cursor::new(Vec::<u8>::new());
    assert_eq!(
        read_limited_to_string(&mut empty, 10, "Input").expect("empty input"),
        ""
    );
    assert!(!position_inside_markup_for_tests("plain text only", 0));
    assert!(!position_inside_markup_for_tests("plain text only", 5));
    assert!(!position_inside_markup_for_tests("plain text only", 99));
    assert!(!crate::extract::markup_position_is_in_bounds_for_tests(
        0, 10
    ));
    assert!(crate::extract::markup_position_is_in_bounds_for_tests(
        10, 10
    ));
    assert!(!crate::extract::markup_position_is_in_bounds_for_tests(
        11, 10
    ));
    assert!(crate::extract::markup_cursor_step_is_valid_for_tests(
        2, 3, 3
    ));
    assert!(!crate::extract::markup_cursor_step_is_valid_for_tests(
        2, 2, 3
    ));
    assert!(!crate::extract::markup_cursor_step_is_valid_for_tests(
        2, 4, 3
    ));

    let script_text = r#"<script>if (x < y && y > 0) { alert("ok"); }</script><p>done</p>"#;
    let script_text_position = script_text.find("y &&").expect("script text position");
    assert!(!position_inside_markup_for_tests(
        script_text,
        script_text_position
    ));

    let attribute_text = r#"<div data-label="a > b">done</div>"#;
    let content_position = attribute_text.find("done").expect("content position");
    assert!(!position_inside_markup_for_tests(
        attribute_text,
        content_position
    ));

    assert!(
        !crate::extract::position_inside_markup_rejects_invalid_progress_for_tests(
            "<div>text</div>",
            2,
            false,
        )
    );
    assert!(
        !crate::extract::position_inside_markup_rejects_out_of_bounds_progress_for_tests(
            "<div>text</div>",
            2,
        )
    );
    assert_eq!(
        crate::extract::position_inside_markup_stalled_step_count_for_tests("<tag>", 2),
        (false, 1),
    );
    assert!(
        !crate::extract::position_inside_markup_rejects_invalid_progress_for_tests(
            "<div>text</div>",
            2,
            true,
        )
    );

    for attribute_text in [
        r#"<div data-label="a > b">done</div>"#,
        "<div data-label='a > b'>done</div>",
    ] {
        let quoted_position = attribute_text.find("a >").expect("quoted markup position");
        assert!(position_inside_markup_for_tests(
            attribute_text,
            quoted_position
        ));
        let closing_position = attribute_text.find("</div>").expect("closing tag position") + 1;
        assert!(position_inside_markup_for_tests(
            attribute_text,
            closing_position
        ));
    }

    let embedded_delimiter_attribute = r#"<div data-label="a > b">done</div>"#;
    let after_embedded_delimiter = embedded_delimiter_attribute
        .find("b\"")
        .expect("embedded delimiter tail");
    assert!(position_inside_markup_for_tests(
        embedded_delimiter_attribute,
        after_embedded_delimiter
    ));
    let body_after_attribute = embedded_delimiter_attribute
        .find("done")
        .expect("body after attribute");
    assert!(!position_inside_markup_for_tests(
        embedded_delimiter_attribute,
        body_after_attribute
    ));

    let comment_text = "<!-- alpha < beta -->done";
    let comment_position = comment_text.find("alpha").expect("comment position");
    assert!(position_inside_markup_for_tests(
        comment_text,
        comment_position
    ));
    let after_comment_position = comment_text.find("done").expect("after comment position");
    assert!(!position_inside_markup_for_tests(
        comment_text,
        after_comment_position
    ));
}
