use super::candidates::scoring::{same_page_url, select_elements_in_scope};
use super::*;

pub(super) fn build_heading_samples(
    document: &Html,
    sample_limit: usize,
    scope_paths: &[String],
) -> Vec<HeadingInspection> {
    if sample_limit == 0 {
        return Vec::new();
    }

    let selector = Selector::parse("h1, h2, h3, h4, h5, h6").expect("heading selector");
    let mut seen_paths = BTreeSet::new();
    for scope_path in scope_paths.iter().map(String::as_str) {
        let headings = sample_headings_from_scope(
            document,
            Some(scope_path),
            sample_limit,
            &selector,
            &mut seen_paths,
        );
        if !headings.is_empty() {
            return headings;
        }
    }

    sample_headings_from_scope(document, None, sample_limit, &selector, &mut seen_paths)
}

pub(super) fn sample_headings_from_scope(
    document: &Html,
    scope_path: Option<&str>,
    limit: usize,
    selector: &Selector,
    seen_paths: &mut BTreeSet<String>,
) -> Vec<HeadingInspection> {
    select_elements_in_scope(document, scope_path, selector)
        .filter_map(|element| {
            if element_looks_like_utility_chrome(&element)
                || element_has_utility_chrome_ancestor(&element)
            {
                return None;
            }

            let level = heading_level(element.value().name())?;
            let path = build_node_path(&element);
            if !seen_paths.insert(path.clone()) {
                return None;
            }
            let text = extract_heading_text(&element)?;

            Some(HeadingInspection { level, text, path })
        })
        .take(limit)
        .collect()
}

pub(super) fn count_meaningful_headings(element: &ElementRef<'_>, selector: &Selector) -> usize {
    element
        .select(selector)
        .filter(|heading| extract_heading_text(heading).is_some())
        .count()
}

pub(super) fn first_meaningful_heading<'a>(
    element: &'a ElementRef<'a>,
    selector: &Selector,
) -> Option<ElementRef<'a>> {
    element
        .select(selector)
        .find(|heading| extract_heading_text(heading).is_some())
}

pub(super) fn build_link_samples(
    document: &Html,
    effective_base_url: Option<&str>,
    sample_limit: usize,
    scope_paths: &[String],
) -> Vec<LinkInspection> {
    if sample_limit == 0 {
        return Vec::new();
    }

    let selector = Selector::parse("a").expect("link selector");
    let mut seen_paths = BTreeSet::new();
    let mut links = Vec::new();
    for scope_path in scope_paths.iter().map(String::as_str) {
        links.extend(sample_links_from_scope(
            document,
            effective_base_url,
            Some(scope_path),
            sample_limit.saturating_sub(links.len()),
            &selector,
            &mut seen_paths,
        ));
        if links.len() >= sample_limit {
            return links;
        }
    }

    links.extend(sample_links_from_scope(
        document,
        effective_base_url,
        None,
        sample_limit.saturating_sub(links.len()),
        &selector,
        &mut seen_paths,
    ));
    links
}

pub(super) fn sample_links_from_scope(
    document: &Html,
    effective_base_url: Option<&str>,
    scope_path: Option<&str>,
    limit: usize,
    selector: &Selector,
    seen_paths: &mut BTreeSet<String>,
) -> Vec<LinkInspection> {
    select_elements_in_scope(document, scope_path, selector)
        .filter_map(|element| {
            if element_looks_like_utility_chrome(&element)
                || element_has_utility_chrome_ancestor(&element)
            {
                return None;
            }

            let href = element
                .value()
                .attr("href")
                .map(str::trim)
                .filter(|value| href_is_meaningful_destination(value))
                .map(str::to_owned)?;
            let text =
                render_html_as_text(&serialize_children(&element), WhitespaceMode::Normalize);
            if text.is_empty() {
                return None;
            }
            if link_preview_is_low_signal(&href, &text, &path_hint_for_link(&element)) {
                return None;
            }

            let path = build_node_path(&element);
            if !seen_paths.insert(path.clone()) {
                return None;
            }

            let resolved_href = resolve_url(&href, effective_base_url);
            if effective_base_url.is_some_and(|base| same_page_url(&resolved_href, base)) {
                return None;
            }

            Some(LinkInspection {
                text,
                resolved_href: Some(resolved_href),
                href: Some(href),
                path,
            })
        })
        .take(limit)
        .collect()
}

pub(super) fn path_hint_for_link(element: &ElementRef<'_>) -> String {
    build_node_path(element)
}

pub(super) fn link_preview_is_low_signal(href: &str, text: &str, path: &str) -> bool {
    if !href.starts_with('#') {
        let normalized_text = text.to_ascii_lowercase();
        let normalized_href = href.to_ascii_lowercase();
        let normalized_path = path.to_ascii_lowercase();
        if LOW_SIGNAL_LINK_TEXT_PHRASES
            .iter()
            .any(|phrase| normalized_text.contains(phrase))
        {
            return true;
        }
        if LOW_SIGNAL_LINK_HREF_FRAGMENTS
            .iter()
            .any(|fragment| normalized_href.contains(fragment))
        {
            return true;
        }
        if LOW_SIGNAL_LINK_PATH_TOKENS
            .iter()
            .any(|token| normalized_path.contains(token))
        {
            return true;
        }
        return false;
    }

    let trimmed = text.trim();
    let has_word = trimmed.chars().any(char::is_alphabetic);
    let only_marker_chars = !trimmed.is_empty()
        && trimmed.chars().all(|character| {
            character.is_ascii_digit() || matches!(character, '*' | '#' | '[' | ']')
        });

    (!has_word && only_marker_chars) || path.contains("> sup:nth-of-type(")
}

pub(super) fn count_meaningful_links(element: &ElementRef<'_>, selector: &Selector) -> usize {
    element
        .select(selector)
        .filter(|candidate| {
            if element_looks_like_utility_chrome(candidate) {
                return false;
            }
            if element_has_utility_chrome_ancestor(candidate) {
                return false;
            }

            let Some(href) = candidate
                .value()
                .attr("href")
                .map(str::trim)
                .filter(|value| href_is_meaningful_destination(value))
            else {
                return false;
            };
            let text =
                render_html_as_text(&serialize_children(candidate), WhitespaceMode::Normalize);
            if text.is_empty() {
                return false;
            }
            if link_preview_is_low_signal(href, &text, &path_hint_for_link(candidate)) {
                return false;
            }
            true
        })
        .count()
}
