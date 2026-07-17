use super::super::*;
use crate::contracts::WhitespaceMode;

#[test]
fn empty_list_items_do_not_emit_stray_bullet_markers() {
    assert_eq!(
        render_html_as_text(
            "<ul><li> </li><li><span>Visible item</span></li></ul>",
            WhitespaceMode::Rendered,
        ),
        "- Visible item"
    );
    let nested_only = render_html_as_text(
        "<ul><li><ul><li>Nested item</li></ul></li></ul>",
        WhitespaceMode::Rendered,
    );
    assert!(nested_only.trim_start().starts_with("- Nested item"));
    assert!(!nested_only.contains("\n- \n"));
    assert_eq!(
        render_html_as_text(
            "<ul><li><ul><li>Nested item</li></ul><ol><li>Second nested item</li></ol></li></ul>",
            WhitespaceMode::Rendered,
        ),
        "    - Nested item\n    1. Second nested item"
    );
}

#[test]
fn immediate_duplicate_headings_are_collapsed_in_reader_text() {
    assert_eq!(
        render_html_as_text(
            "<section><h2>Why Apple is the best place to buy iPhone.</h2><h2>Why Apple is the best place to buy iPhone.</h2><p>Details.</p></section>",
            WhitespaceMode::Rendered,
        ),
        "## Why Apple is the best place to buy iPhone.\n\nDetails."
    );
}
