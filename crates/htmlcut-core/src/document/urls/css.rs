//! CSS URL token rewriting for style attributes and style elements.

use super::base::resolve_url;

pub(super) fn rewrite_css_urls(value: &str, base_url: Option<&str>) -> String {
    let Some(base_url) = base_url else {
        return value.to_owned();
    };

    let mut rewritten = String::with_capacity(value.len());
    let mut cursor = 0usize;
    while cursor < value.len() {
        if let Some(end) = css_comment_end(value, cursor) {
            rewritten.push_str(&value[cursor..end]);
            cursor = end;
            continue;
        }

        if let Some((replacement, next)) = rewrite_css_url_function_at(value, cursor, base_url) {
            rewritten.push_str(&replacement);
            cursor = next;
            continue;
        }

        if let Some((replacement, next)) = rewrite_css_import_string_at(value, cursor, base_url) {
            rewritten.push_str(&replacement);
            cursor = next;
            continue;
        }

        let next = next_char_boundary(value, cursor);
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
    index = skip_ascii_whitespace(value, index);
    if !value[index..].starts_with('(') {
        return None;
    }

    let mut content_start = skip_ascii_whitespace(value, index + 1);
    let quote = value[content_start..].chars().next()?;
    if quote == '"' || quote == '\'' {
        let raw_start = content_start + quote.len_utf8();
        let raw_end = find_css_string_end(value, content_start)?;
        let after_quote = skip_ascii_whitespace(value, raw_end + quote.len_utf8());
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
        content_start = next_char_boundary(value, content_start);
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

fn skip_css_ignorable(value: &str, mut cursor: usize) -> usize {
    loop {
        let next = skip_ascii_whitespace(value, cursor);
        if let Some(end) = css_comment_end(value, next) {
            cursor = end;
            continue;
        }
        return next;
    }
}

pub(super) fn skip_ascii_whitespace(value: &str, mut cursor: usize) -> usize {
    while cursor < value.len() {
        let ch = value[cursor..].chars().next().expect("char boundary");
        if !ch.is_ascii_whitespace() {
            break;
        }
        cursor += ch.len_utf8();
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
    let quote = value[quote_index..].chars().next()?;
    let mut cursor = quote_index + quote.len_utf8();
    while cursor < value.len() {
        let ch = value[cursor..].chars().next()?;
        if ch == '\\' {
            cursor = next_char_boundary(value, cursor);
            if cursor < value.len() {
                cursor = next_char_boundary(value, cursor);
            }
            continue;
        }
        if ch == quote {
            return Some(cursor);
        }
        cursor = next_char_boundary(value, cursor);
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
