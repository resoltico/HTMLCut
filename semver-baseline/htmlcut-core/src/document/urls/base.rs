//! Base-URL resolution and destination classification.

use scraper::Html;
use url::Url;

use super::super::parse::select_first;

pub(crate) fn document_base_href(document: &Html) -> Option<String> {
    select_first(document, "base[href]")
        .and_then(|node| node.value().attr("href"))
        .map(str::trim)
        .filter(|href| !href.is_empty())
        .map(str::to_owned)
}

pub(crate) fn resolve_document_base_url(
    document: &Html,
    input_base_url: Option<&str>,
) -> Option<String> {
    let Some(document_base_href) = document_base_href(document) else {
        return input_base_url.map(ToOwned::to_owned);
    };

    if document_base_href.starts_with('#') {
        return input_base_url.map(ToOwned::to_owned);
    }

    if let Ok(parsed) = Url::parse(&document_base_href)
        && matches!(parsed.scheme(), "http" | "https")
    {
        return Some(parsed.to_string());
    }

    input_base_url
        .and_then(|base| Url::parse(base).ok())
        .and_then(|base| base.join(&document_base_href).ok())
        .filter(|resolved| matches!(resolved.scheme(), "http" | "https"))
        .map(|resolved| resolved.to_string())
        .or_else(|| input_base_url.map(ToOwned::to_owned))
}

pub(crate) fn resolve_url(value: &str, base_url: Option<&str>) -> String {
    let Some(base) = base_url else {
        return value.to_owned();
    };

    if value.trim().is_empty() {
        return value.to_owned();
    }

    if value.starts_with('#') {
        return value.to_owned();
    }

    if Url::parse(value).is_ok() {
        return value.to_owned();
    }

    match Url::parse(base).and_then(|base_url| base_url.join(value)) {
        Ok(url) => url.to_string(),
        Err(_) => value.to_owned(),
    }
}

pub(crate) fn href_is_meaningful_destination(value: &str) -> bool {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed == "#" {
        return false;
    }

    !starts_with_ignore_ascii_case(trimmed, "javascript:")
}

pub(super) fn starts_with_ignore_ascii_case(value: &str, prefix: &str) -> bool {
    value
        .get(..prefix.len())
        .is_some_and(|head| head.eq_ignore_ascii_case(prefix))
}
