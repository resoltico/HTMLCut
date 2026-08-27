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
    rewrite_css_urls_with_steps_and_budget(
        value,
        base_url,
        comment_end,
        rewrite_url,
        rewrite_import,
        next_char,
        scan_step_budget(value),
    )
}

fn rewrite_css_urls_with_steps_and_budget(
    value: &str,
    base_url: Option<&str>,
    comment_end: fn(&str, usize) -> Option<usize>,
    rewrite_url: fn(&str, usize, &str) -> Option<(String, usize)>,
    rewrite_import: fn(&str, usize, &str) -> Option<(String, usize)>,
    next_char: fn(&str, usize) -> usize,
    mut remaining_steps: usize,
) -> String {
    let Some(base_url) = base_url else {
        return value.to_owned();
    };

    let mut rewritten = String::with_capacity(value.len());
    let mut cursor = 0usize;
    while cursor < value.len() {
        if !consume_scan_step_budget(&mut remaining_steps) {
            return value.to_owned();
        }
        if let Some(end) = comment_end(value, cursor) {
            if cursor_does_not_advance(cursor, end) {
                return value.to_owned();
            }
            if !is_in_bounds_char_boundary(value, end) {
                return value.to_owned();
            }
            rewritten.push_str(&value[cursor..end]);
            cursor = end;
            continue;
        }

        if let Some((replacement, next)) = rewrite_url(value, cursor, base_url) {
            if cursor_does_not_advance(cursor, next) {
                return value.to_owned();
            }
            if !is_in_bounds_char_boundary(value, next) {
                return value.to_owned();
            }
            rewritten.push_str(&replacement);
            cursor = next;
            continue;
        }

        if let Some((replacement, next)) = rewrite_import(value, cursor, base_url) {
            if cursor_does_not_advance(cursor, next) {
                return value.to_owned();
            }
            if !is_in_bounds_char_boundary(value, next) {
                return value.to_owned();
            }
            rewritten.push_str(&replacement);
            cursor = next;
            continue;
        }

        let next = next_char(value, cursor);
        if cursor_does_not_advance(cursor, next) {
            return value.to_owned();
        }
        if !is_in_bounds_char_boundary(value, next) {
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
    rewrite_css_url_function_at_with_next_and_budget(
        value,
        cursor,
        base_url,
        next_char,
        scan_step_budget(value),
    )
}

fn rewrite_css_url_function_at_with_next_and_budget(
    value: &str,
    cursor: usize,
    base_url: &str,
    next_char: fn(&str, usize) -> usize,
    mut remaining_steps: usize,
) -> Option<(String, usize)> {
    if !starts_with_css_keyword(value, cursor, "url") {
        return None;
    }
    if value[..cursor]
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
    loop {
        if !consume_scan_step_budget(&mut remaining_steps) {
            return None;
        }
        let ch = value.get(content_start..)?.chars().next()?;
        if ch == ')' {
            break;
        }
        let next = next_char(value, content_start);
        if cursor_does_not_advance(content_start, next) {
            return None;
        }
        if !is_in_bounds_char_boundary(value, next) {
            return None;
        }
        content_start = next;
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
    cursor: usize,
    comment_end: fn(&str, usize) -> Option<usize>,
) -> usize {
    skip_css_ignorable_with_comment_and_budget(value, cursor, comment_end, scan_step_budget(value))
}

fn skip_css_ignorable_with_comment_and_budget(
    value: &str,
    mut cursor: usize,
    comment_end: fn(&str, usize) -> Option<usize>,
    mut remaining_steps: usize,
) -> usize {
    loop {
        if !consume_scan_step_budget(&mut remaining_steps) {
            return cursor;
        }
        let next = skip_ascii_whitespace(value, cursor);
        if let Some(end) = comment_end(value, next) {
            if cursor_does_not_advance(cursor, end) {
                return next;
            }
            if is_in_bounds_char_boundary(value, end) {
                cursor = end;
                continue;
            }
        }
        return next;
    }
}

pub(super) fn skip_ascii_whitespace(value: &str, cursor: usize) -> usize {
    skip_ascii_whitespace_with_next(value, cursor, next_char_boundary)
}

fn skip_ascii_whitespace_with_next(
    value: &str,
    cursor: usize,
    next_char: fn(&str, usize) -> usize,
) -> usize {
    skip_ascii_whitespace_with_next_and_budget(value, cursor, next_char, scan_step_budget(value))
}

fn skip_ascii_whitespace_with_next_and_budget(
    value: &str,
    mut cursor: usize,
    next_char: fn(&str, usize) -> usize,
    mut remaining_steps: usize,
) -> usize {
    while cursor < value.len() {
        if !consume_scan_step_budget(&mut remaining_steps) {
            return cursor;
        }
        let ch = value[cursor..].chars().next().expect("char boundary");
        if !ch.is_ascii_whitespace() {
            break;
        }
        let next = next_char(value, cursor);
        if cursor_does_not_advance(cursor, next) {
            break;
        }
        if !is_in_bounds_char_boundary(value, next) {
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
    find_css_string_end_with_next_and_budget(value, quote_index, next_char, scan_step_budget(value))
}

fn find_css_string_end_with_next_and_budget(
    value: &str,
    quote_index: usize,
    next_char: fn(&str, usize) -> usize,
    mut remaining_steps: usize,
) -> Option<usize> {
    let quote = value[quote_index..].chars().next()?;
    let mut cursor = quote_index + quote.len_utf8();
    loop {
        if !consume_scan_step_budget(&mut remaining_steps) {
            return None;
        }
        let ch = value.get(cursor..)?.chars().next()?;
        if ch == '\\' {
            let next = next_char(value, cursor);
            if cursor_does_not_advance(cursor, next) {
                return None;
            }
            if !is_in_bounds_char_boundary(value, next) {
                return None;
            }
            cursor = next;
            if cursor < value.len() {
                let next = next_char(value, cursor);
                if cursor_does_not_advance(cursor, next) {
                    return None;
                }
                if !is_in_bounds_char_boundary(value, next) {
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
        if cursor_does_not_advance(cursor, next) {
            return None;
        }
        if !is_in_bounds_char_boundary(value, next) {
            return None;
        }
        cursor = next;
    }
}

fn next_char_boundary(value: &str, cursor: usize) -> usize {
    cursor
        + value[cursor..]
            .chars()
            .next()
            .expect("char boundary")
            .len_utf8()
}

fn is_in_bounds_char_boundary(value: &str, next: usize) -> bool {
    next <= value.len() && value.is_char_boundary(next)
}

fn cursor_does_not_advance(cursor: usize, next: usize) -> bool {
    next <= cursor
}

fn scan_step_budget(value: &str) -> usize {
    value.chars().count().saturating_add(1)
}

fn consume_scan_step_budget(remaining_steps: &mut usize) -> bool {
    if *remaining_steps == 0 {
        return false;
    }
    *remaining_steps -= 1;
    true
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
mod test_support;

#[cfg(test)]
pub(crate) use test_support::*;
