//! CSS URL token rewriting for style attributes and style elements.

use super::base::resolve_url;

pub(super) fn rewrite_css_urls(value: &str, base_url: Option<&str>) -> String {
    rewrite_css_urls_with_steps(
        value,
        base_url,
        css_comment_end,
        rewrite_css_url_function_at,
        rewrite_css_import_string_at,
        next_char_boundary,
    )
}

fn rewrite_css_urls_with_steps(
    value: &str,
    base_url: Option<&str>,
    comment_end: fn(&str, usize) -> Option<usize>,
    rewrite_url: fn(&str, usize, &str) -> Option<(String, usize)>,
    rewrite_import: fn(&str, usize, &str) -> Option<(String, usize)>,
    next_char: fn(&str, usize) -> usize,
) -> String {
    let Some(base_url) = base_url else {
        return value.to_owned();
    };

    let mut rewritten = String::with_capacity(value.len());
    let mut cursor = 0usize;
    while cursor < value.len() {
        if let Some(end) = comment_end(value, cursor) {
            if advances_to_char_boundary(value, cursor, end) {
                rewritten.push_str(&value[cursor..end]);
                cursor = end;
                continue;
            }
            return value.to_owned();
        }

        if let Some((replacement, next)) = rewrite_url(value, cursor, base_url) {
            if advances_to_char_boundary(value, cursor, next) {
                rewritten.push_str(&replacement);
                cursor = next;
                continue;
            }
            return value.to_owned();
        }

        if let Some((replacement, next)) = rewrite_import(value, cursor, base_url) {
            if advances_to_char_boundary(value, cursor, next) {
                rewritten.push_str(&replacement);
                cursor = next;
                continue;
            }
            return value.to_owned();
        }

        let next = next_char(value, cursor);
        if !advances_to_char_boundary(value, cursor, next) {
            return value.to_owned();
        }
        rewritten.push_str(&value[cursor..next]);
        cursor = next;
    }

    rewritten
}

pub(super) fn css_comment_end(value: &str, cursor: usize) -> Option<usize> {
    if !value[cursor..].starts_with("/*") {
        return None;
    }

    Some(
        value[cursor + 2..]
            .find("*/")
            .map(|offset| cursor + 2 + offset + 2)
            .unwrap_or(value.len()),
    )
}

pub(super) fn rewrite_css_import_string_at(
    value: &str,
    cursor: usize,
    base_url: &str,
) -> Option<(String, usize)> {
    if !value[cursor..].starts_with('@') || !starts_with_css_keyword(value, cursor + 1, "import") {
        return None;
    }

    let mut index = cursor + 1 + "import".len();
    index = skip_css_ignorable(value, index);
    let quote = value[index..].chars().next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }

    let content_start = index + quote.len_utf8();
    let content_end = find_css_string_end(value, index)?;
    let resolved = resolve_url(&value[content_start..content_end], Some(base_url));
    let next = content_end + quote.len_utf8();

    Some((
        format!(
            "{}{}{}",
            &value[cursor..content_start],
            resolved,
            &value[content_end..next]
        ),
        next,
    ))
}

pub(super) fn rewrite_css_url_function_at(
    value: &str,
    cursor: usize,
    base_url: &str,
) -> Option<(String, usize)> {
    rewrite_css_url_function_at_with_next(value, cursor, base_url, next_char_boundary)
}

fn rewrite_css_url_function_at_with_next(
    value: &str,
    cursor: usize,
    base_url: &str,
    next_char: fn(&str, usize) -> usize,
) -> Option<(String, usize)> {
    if !starts_with_css_keyword(value, cursor, "url") {
        return None;
    }
    if cursor > 0
        && value[..cursor]
            .chars()
            .next_back()
            .is_some_and(is_css_identifier_char)
    {
        return None;
    }

    let mut index = cursor + "url".len();
    index = skip_ascii_whitespace_with_next(value, index, next_char);
    if !value[index..].starts_with('(') {
        return None;
    }

    let mut content_start = skip_ascii_whitespace_with_next(value, index + 1, next_char);
    let quote = value[content_start..].chars().next()?;
    if quote == '"' || quote == '\'' {
        let raw_start = content_start + quote.len_utf8();
        let raw_end = find_css_string_end_with_next(value, content_start, next_char)?;
        let after_quote =
            skip_ascii_whitespace_with_next(value, raw_end + quote.len_utf8(), next_char);
        if !value[after_quote..].starts_with(')') {
            return None;
        }
        let resolved = resolve_url(&value[raw_start..raw_end], Some(base_url));
        let next = after_quote + 1;
        return Some((
            format!(
                "{}{}{}",
                &value[cursor..raw_start],
                resolved,
                &value[raw_end..next]
            ),
            next,
        ));
    }

    let raw_start = content_start;
    while content_start < value.len() {
        let ch = value[content_start..].chars().next()?;
        if ch == ')' {
            break;
        }
        let next = next_char(value, content_start);
        if !advances_to_char_boundary(value, content_start, next) {
            return None;
        }
        content_start = next;
    }
    if content_start >= value.len() {
        return None;
    }
    debug_assert!(value[content_start..].starts_with(')'));

    let mut raw_end = content_start;
    while raw_end > raw_start
        && value[..raw_end]
            .chars()
            .next_back()
            .is_some_and(char::is_whitespace)
    {
        raw_end = previous_char_boundary(value, raw_end);
    }
    if raw_end == raw_start {
        return None;
    }

    let resolved = resolve_url(&value[raw_start..raw_end], Some(base_url));
    let next = content_start + 1;
    Some((
        format!(
            "{}{}{}",
            &value[cursor..raw_start],
            resolved,
            &value[raw_end..next]
        ),
        next,
    ))
}

fn skip_css_ignorable(value: &str, cursor: usize) -> usize {
    skip_css_ignorable_with_comment(value, cursor, css_comment_end)
}

fn skip_css_ignorable_with_comment(
    value: &str,
    mut cursor: usize,
    comment_end: fn(&str, usize) -> Option<usize>,
) -> usize {
    loop {
        let next = skip_ascii_whitespace(value, cursor);
        if let Some(end) = comment_end(value, next)
            && advances_to_char_boundary(value, cursor, end)
        {
            cursor = end;
            continue;
        }
        return next;
    }
}

pub(super) fn skip_ascii_whitespace(value: &str, cursor: usize) -> usize {
    skip_ascii_whitespace_with_next(value, cursor, next_char_boundary)
}

fn skip_ascii_whitespace_with_next(
    value: &str,
    mut cursor: usize,
    next_char: fn(&str, usize) -> usize,
) -> usize {
    while cursor < value.len() {
        let ch = value[cursor..].chars().next().expect("char boundary");
        if !ch.is_ascii_whitespace() {
            break;
        }
        let next = next_char(value, cursor);
        if !advances_to_char_boundary(value, cursor, next) {
            break;
        }
        cursor = next;
    }
    cursor
}

fn starts_with_css_keyword(value: &str, cursor: usize, keyword: &str) -> bool {
    let end = cursor + keyword.len();
    value
        .get(cursor..end)
        .is_some_and(|candidate| candidate.eq_ignore_ascii_case(keyword))
}

pub(super) fn find_css_string_end(value: &str, quote_index: usize) -> Option<usize> {
    find_css_string_end_with_next(value, quote_index, next_char_boundary)
}

fn find_css_string_end_with_next(
    value: &str,
    quote_index: usize,
    next_char: fn(&str, usize) -> usize,
) -> Option<usize> {
    let quote = value[quote_index..].chars().next()?;
    let mut cursor = quote_index + quote.len_utf8();
    while cursor < value.len() {
        let ch = value[cursor..].chars().next()?;
        if ch == '\\' {
            let next = next_char(value, cursor);
            if !advances_to_char_boundary(value, cursor, next) {
                return None;
            }
            cursor = next;
            if cursor < value.len() {
                let next = next_char(value, cursor);
                if !advances_to_char_boundary(value, cursor, next) {
                    return None;
                }
                cursor = next;
            }
            continue;
        }
        if ch == quote {
            return Some(cursor);
        }
        let next = next_char(value, cursor);
        if !advances_to_char_boundary(value, cursor, next) {
            return None;
        }
        cursor = next;
    }
    None
}

fn next_char_boundary(value: &str, cursor: usize) -> usize {
    cursor
        + value[cursor..]
            .chars()
            .next()
            .expect("char boundary")
            .len_utf8()
}

fn advances_to_char_boundary(value: &str, cursor: usize, next: usize) -> bool {
    next > cursor && next <= value.len() && value.is_char_boundary(next)
}

fn previous_char_boundary(value: &str, cursor: usize) -> usize {
    value[..cursor]
        .char_indices()
        .last()
        .map(|(index, _)| index)
        .unwrap_or(0)
}

pub(super) fn is_css_identifier_char(ch: char) -> bool {
    ch == '-' || ch == '_' || ch.is_alphanumeric()
}

#[cfg(test)]
pub(crate) fn rewrite_css_urls_for_tests(value: &str, base_url: Option<&str>) -> String {
    rewrite_css_urls(value, base_url)
}

#[cfg(test)]
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

#[cfg(test)]
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

#[cfg(test)]
pub(crate) fn css_progress_is_valid_for_tests(value: &str, cursor: usize, next: usize) -> bool {
    advances_to_char_boundary(value, cursor, next)
}

#[cfg(test)]
pub(crate) fn css_ignorable_rejects_nonadvancing_comment_for_tests() -> bool {
    skip_css_ignorable_with_comment("/* comment */", 0, |_, _| Some(0)) == 0
}

#[cfg(test)]
#[derive(Clone, Copy)]
pub(crate) enum CssProgressFault {
    Url,
    Whitespace,
    String,
    PlainString,
    EscapedString,
}

#[cfg(test)]
#[derive(Clone, Copy)]
pub(crate) enum CssDispatchFault {
    Comment,
    Url,
    Import,
    Character,
}
