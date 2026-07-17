use super::*;

#[test]
fn rendering_helpers_handle_blocks_spacing_previews_and_url_resolution() {
    let richer_rendered = render_html_as_text(
        "<blockquote><p>Quote</p></blockquote><dl><dt>Term</dt><dd>Definition</dd></dl><p>Use <code>cargo test</code></p>",
        WhitespaceMode::Rendered,
    );
    assert!(richer_rendered.contains("> Quote"));
    assert!(richer_rendered.contains("Term\n: Definition"));
    assert!(richer_rendered.contains("`cargo test`"));
    let block_definition_rendered = render_html_as_text(
        "<dl><dt>Term</dt><dd><p>Definition</p></dd></dl>",
        WhitespaceMode::Rendered,
    );
    assert_eq!(block_definition_rendered, "Term\n: Definition");
    let collapsed_blockquote = render_html_as_text(
        "<blockquote><p>First</p><p></p><p></p><p>Second</p></blockquote>",
        WhitespaceMode::Rendered,
    );
    assert_eq!(collapsed_blockquote, "> First\n>\n> Second");
    let empty_blockquote =
        render_html_as_text("<blockquote>   </blockquote>", WhitespaceMode::Rendered);
    assert!(empty_blockquote.is_empty());

    assert_eq!(
        collapse_inline_whitespace("  Hello   world "),
        "Hello world"
    );
    assert!(needs_space("Hello", "world"));
    assert!(!needs_space("", "world"));
    assert!(!needs_space("Hello", ""));
    assert!(!needs_space("Hello", "."));
    assert!(!needs_space("Hello ", "world"));
    assert!(!needs_space("-", "world"));

    let mut output = String::from("Hello\n\n");
    push_newline(&mut output, 2);
    assert_eq!(output, "Hello\n\n");

    assert_eq!(
        apply_whitespace_mode(" Hello \n\n World ", WhitespaceMode::Normalize),
        " Hello\n\n World"
    );
    assert_eq!(
        apply_whitespace_mode("A\n\nB", WhitespaceMode::Normalize),
        "A\n\nB"
    );
    assert!(looks_like_full_document("<html><body></body></html>"));
    assert_eq!(
        rewrite_html_urls(
            "<a href=\"guide.html\">Guide</a>",
            Some("https://example.com/docs/"),
            false
        ),
        "<a href=\"https://example.com/docs/guide.html\">Guide</a>"
    );
    assert_eq!(resolve_url("#frag", Some("https://example.com")), "#frag");
    assert_eq!(
        resolve_url("https://openai.com", Some("https://example.com")),
        "https://openai.com"
    );
    assert_eq!(resolve_url("   ", Some("https://example.com")), "   ");
    assert_eq!(resolve_url("guide.html", None), "guide.html");
    assert_eq!(resolve_url("guide.html", Some("not a url")), "guide.html");
    assert_eq!(
        rewrite_html_urls("<p>Hello</p>", None, false),
        "<p>Hello</p>"
    );
    let fragment_without_body = Html::parse_fragment("<p>Fragment only</p>");
    assert_eq!(
        render_document_body_as_text(&fragment_without_body, WhitespaceMode::Rendered),
        "Fragment only"
    );
    assert!(!looks_like_full_document("<body>Hello</body>"));
    assert!(first_body(&parse_wrapped_fragment("<p>Hello</p>")).is_some());
    assert!(first_body_child_element(&parse_wrapped_fragment("plain")).is_none());
    assert!(build_preview(&json!({"k": "v"}), 5).ends_with(ELLIPSIS));
    assert!(!has_errors(&[warning_diagnostic(
        DiagnosticCode::MultipleMatches,
        "x",
        None
    )]));
    assert!(has_errors(&[error_diagnostic(
        DiagnosticCode::SourceLoadFailed,
        "x",
        None
    )]));
    assert_eq!(
        warning_diagnostic(DiagnosticCode::MultipleMatches, "x", None).level,
        DiagnosticLevel::Warning
    );
}
