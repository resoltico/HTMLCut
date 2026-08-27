use super::*;

fn advances_to_char_boundary(value: &str, cursor: usize, next: usize) -> bool {
    next > cursor && is_in_bounds_char_boundary(value, next)
}

pub(crate) fn rewrite_css_urls_for_tests(value: &str, base_url: Option<&str>) -> String {
    rewrite_css_urls(value, base_url)
}

pub(crate) fn css_progress_rejection_for_tests(kind: CssProgressFault) -> bool {
    fn no_progress(_: &str, _: usize) -> usize {
        0
    }

    match kind {
        CssProgressFault::Url => rewrite_css_url_function_at_with_next(
            "url(asset.png)",
            0,
            "https://example.test/",
            no_progress,
        )
        .is_none(),
        CssProgressFault::Whitespace => {
            skip_ascii_whitespace_with_next("  asset", 0, no_progress) == 0
        }
        CssProgressFault::String => {
            find_css_string_end_with_next("'\\\\asset'", 0, no_progress).is_none()
        }
        CssProgressFault::PlainString => {
            find_css_string_end_with_next("'asset'", 0, no_progress).is_none()
        }
        CssProgressFault::EscapedString => {
            fn stall_after_escape(_: &str, cursor: usize) -> usize {
                if cursor == 1 { 2 } else { 0 }
            }
            find_css_string_end_with_next("'\\\\asset'", 0, stall_after_escape).is_none()
        }
    }
}

pub(crate) fn css_dispatch_rejection_for_tests(kind: CssDispatchFault) -> bool {
    fn no_progress(_: &str, _: usize) -> usize {
        0
    }
    fn no_comment(_: &str, _: usize) -> Option<usize> {
        None
    }
    fn no_url(_: &str, _: usize, _: &str) -> Option<(String, usize)> {
        None
    }
    fn no_import(_: &str, _: usize, _: &str) -> Option<(String, usize)> {
        None
    }
    fn stalled_comment(_: &str, _: usize) -> Option<usize> {
        Some(0)
    }
    fn stalled_url(_: &str, _: usize, _: &str) -> Option<(String, usize)> {
        Some((String::new(), 0))
    }
    fn stalled_import(_: &str, _: usize, _: &str) -> Option<(String, usize)> {
        Some((String::new(), 0))
    }

    let value = match kind {
        CssDispatchFault::Comment => rewrite_css_urls_with_steps(
            "/* note */",
            Some("https://example.test/"),
            stalled_comment,
            no_url,
            no_import,
            next_char_boundary,
        ),
        CssDispatchFault::Url => rewrite_css_urls_with_steps(
            "url(asset.png)",
            Some("https://example.test/"),
            no_comment,
            stalled_url,
            no_import,
            next_char_boundary,
        ),
        CssDispatchFault::Import => rewrite_css_urls_with_steps(
            "@import \"theme.css\"",
            Some("https://example.test/"),
            no_comment,
            no_url,
            stalled_import,
            next_char_boundary,
        ),
        CssDispatchFault::Character => rewrite_css_urls_with_steps(
            "plain",
            Some("https://example.test/"),
            no_comment,
            no_url,
            no_import,
            no_progress,
        ),
    };
    value
        == match kind {
            CssDispatchFault::Comment => "/* note */",
            CssDispatchFault::Url => "url(asset.png)",
            CssDispatchFault::Import => "@import \"theme.css\"",
            CssDispatchFault::Character => "plain",
        }
}

pub(crate) fn css_budget_rejection_for_tests(kind: CssBudgetFault) -> bool {
    match kind {
        CssBudgetFault::Dispatcher => {
            rewrite_css_urls_with_steps_and_budget(
                "plain",
                Some("https://example.test/"),
                css_comment_end,
                rewrite_css_url_function_at,
                rewrite_css_import_string_at,
                next_char_boundary,
                0,
            ) == "plain"
        }
        CssBudgetFault::Url => rewrite_css_url_function_at_with_next_and_budget(
            "url(asset.png)",
            0,
            "https://example.test/",
            next_char_boundary,
            0,
        )
        .is_none(),
        CssBudgetFault::Ignorable => {
            skip_css_ignorable_with_comment_and_budget("/* comment */", 0, css_comment_end, 0) == 0
        }
        CssBudgetFault::Whitespace => {
            skip_ascii_whitespace_with_next_and_budget(" asset", 0, next_char_boundary, 0) == 0
        }
        CssBudgetFault::String => {
            find_css_string_end_with_next_and_budget("'asset'", 0, next_char_boundary, 0).is_none()
        }
    }
}

pub(crate) fn css_bounds_rejection_for_tests(kind: CssBoundsFault) -> bool {
    fn no_comment(_: &str, _: usize) -> Option<usize> {
        None
    }
    fn no_url(_: &str, _: usize, _: &str) -> Option<(String, usize)> {
        None
    }
    fn no_import(_: &str, _: usize, _: &str) -> Option<(String, usize)> {
        None
    }
    fn comment_after_end(value: &str, _: usize) -> Option<usize> {
        Some(value.len() + 1)
    }
    fn url_after_end(value: &str, _: usize, _: &str) -> Option<(String, usize)> {
        Some((String::new(), value.len() + 1))
    }
    fn import_after_end(value: &str, _: usize, _: &str) -> Option<(String, usize)> {
        Some((String::new(), value.len() + 1))
    }
    fn after_end(value: &str, _: usize) -> usize {
        value.len() + 1
    }
    fn after_escape_then_end(value: &str, cursor: usize) -> usize {
        if cursor == 1 { 2 } else { value.len() + 1 }
    }

    match kind {
        CssBoundsFault::DispatcherComment => {
            rewrite_css_urls_with_steps(
                "/* note */",
                Some("https://example.test/"),
                comment_after_end,
                no_url,
                no_import,
                next_char_boundary,
            ) == "/* note */"
        }
        CssBoundsFault::DispatcherUrl => {
            rewrite_css_urls_with_steps(
                "url(asset.png)",
                Some("https://example.test/"),
                no_comment,
                url_after_end,
                no_import,
                next_char_boundary,
            ) == "url(asset.png)"
        }
        CssBoundsFault::DispatcherImport => {
            rewrite_css_urls_with_steps(
                "@import \"theme.css\"",
                Some("https://example.test/"),
                no_comment,
                no_url,
                import_after_end,
                next_char_boundary,
            ) == "@import \"theme.css\""
        }
        CssBoundsFault::DispatcherCharacter => {
            rewrite_css_urls_with_steps(
                "plain",
                Some("https://example.test/"),
                no_comment,
                no_url,
                no_import,
                after_end,
            ) == "plain"
        }
        CssBoundsFault::Url => rewrite_css_url_function_at_with_next(
            "url(asset.png)",
            0,
            "https://example.test/",
            after_end,
        )
        .is_none(),
        CssBoundsFault::Ignorable => {
            skip_css_ignorable_with_comment("/* comment */", 0, comment_after_end) == 0
        }
        CssBoundsFault::Whitespace => skip_ascii_whitespace_with_next(" asset", 0, after_end) == 0,
        CssBoundsFault::EscapedStringFirstStep => {
            find_css_string_end_with_next("'\\\\asset'", 0, after_end).is_none()
        }
        CssBoundsFault::EscapedStringSecondStep => {
            find_css_string_end_with_next("'\\\\asset'", 0, after_escape_then_end).is_none()
        }
        CssBoundsFault::PlainString => {
            find_css_string_end_with_next("'asset'", 0, after_end).is_none()
        }
    }
}

pub(crate) fn css_progress_is_valid_for_tests(value: &str, cursor: usize, next: usize) -> bool {
    advances_to_char_boundary(value, cursor, next)
}

pub(crate) fn css_progress_does_not_advance_for_tests(cursor: usize, next: usize) -> bool {
    cursor_does_not_advance(cursor, next)
}

pub(crate) fn css_scan_budget_exhausts_for_tests() -> bool {
    let mut remaining_steps = 3;
    let outcomes = [
        consume_scan_step_budget(&mut remaining_steps),
        consume_scan_step_budget(&mut remaining_steps),
        consume_scan_step_budget(&mut remaining_steps),
        consume_scan_step_budget(&mut remaining_steps),
    ];
    outcomes == [true, true, true, false]
}

pub(crate) fn css_ignorable_rejects_nonadvancing_comment_for_tests() -> bool {
    skip_css_ignorable_with_comment("/* comment */", 0, |_, _| Some(0)) == 0
}

#[derive(Clone, Copy)]
pub(crate) enum CssProgressFault {
    Url,
    Whitespace,
    String,
    PlainString,
    EscapedString,
}

#[derive(Clone, Copy)]
pub(crate) enum CssDispatchFault {
    Comment,
    Url,
    Import,
    Character,
}

#[derive(Clone, Copy)]
pub(crate) enum CssBudgetFault {
    Dispatcher,
    Url,
    Ignorable,
    Whitespace,
    String,
}

#[derive(Clone, Copy)]
pub(crate) enum CssBoundsFault {
    DispatcherComment,
    DispatcherUrl,
    DispatcherImport,
    DispatcherCharacter,
    Url,
    Ignorable,
    Whitespace,
    EscapedStringFirstStep,
    EscapedStringSecondStep,
    PlainString,
}
