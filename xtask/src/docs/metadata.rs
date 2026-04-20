use std::path::Path;

use regex::Regex;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MetadataStyle {
    Frontmatter,
    HtmlComment,
}

const EXPECTED_AFAD_VERSION: &str = "3.5";

#[derive(Debug, Default, PartialEq, Eq)]
struct MetadataFields {
    afad: Option<String>,
    version: Option<String>,
    domain: Option<String>,
    updated: Option<String>,
    has_route_section: bool,
    has_retrieval_hints_section: bool,
    has_keywords: bool,
    has_questions: bool,
}

pub(crate) fn metadata_version(text: &str, style: MetadataStyle) -> Option<String> {
    match style {
        MetadataStyle::Frontmatter => frontmatter_version(text),
        MetadataStyle::HtmlComment => html_comment_version(text),
    }
}

pub(super) fn expected_metadata_style(repo_root: &Path, path: &Path) -> MetadataStyle {
    let relative = path
        .strip_prefix(repo_root)
        .expect("doc path should stay inside repo root");
    if relative
        .components()
        .next()
        .is_some_and(|component| component.as_os_str() == "docs")
    {
        MetadataStyle::Frontmatter
    } else {
        MetadataStyle::HtmlComment
    }
}

pub(super) fn metadata_contract_errors(
    display_path: &str,
    text: &str,
    style: MetadataStyle,
    updated_pattern: &Regex,
) -> Vec<String> {
    let Some(fields) = metadata_fields(text, style) else {
        return vec![format!(
            "{display_path} is missing the expected {} metadata block",
            style.label()
        )];
    };

    let mut errors = Vec::new();
    match fields.afad.as_deref() {
        Some(EXPECTED_AFAD_VERSION) => {}
        Some(value) => errors.push(format!(
            "{display_path} metadata afad is {value}, expected {EXPECTED_AFAD_VERSION}"
        )),
        None => errors.push(format!(
            "{display_path} is missing the expected {} metadata afad entry",
            style.label()
        )),
    }
    if fields.domain.is_none() {
        errors.push(format!(
            "{display_path} is missing the expected {} metadata domain entry",
            style.label()
        ));
    }
    match fields.updated.as_deref() {
        Some(value) if updated_pattern.is_match(value) => {}
        Some(value) => errors.push(format!(
            "{display_path} metadata updated value is not ISO-8601 YYYY-MM-DD: {value}"
        )),
        None => errors.push(format!(
            "{display_path} is missing the expected {} metadata updated entry",
            style.label()
        )),
    }
    match style {
        MetadataStyle::Frontmatter => {
            if !fields.has_route_section {
                errors.push(format!(
                    "{display_path} is missing the expected frontmatter route section"
                ));
            }
            if !fields.has_keywords {
                errors.push(format!(
                    "{display_path} is missing the expected frontmatter route keywords entry"
                ));
            }
            if !fields.has_questions {
                errors.push(format!(
                    "{display_path} is missing the expected frontmatter route questions entry"
                ));
            }
        }
        MetadataStyle::HtmlComment => {
            if !fields.has_retrieval_hints_section {
                errors.push(format!(
                    "{display_path} is missing the expected HTML comment RETRIEVAL_HINTS section"
                ));
            }
            if !fields.has_keywords {
                errors.push(format!(
                    "{display_path} is missing the expected HTML comment RETRIEVAL_HINTS keywords entry"
                ));
            }
            if !fields.has_questions {
                errors.push(format!(
                    "{display_path} is missing the expected HTML comment RETRIEVAL_HINTS questions entry"
                ));
            }
        }
    }

    errors
}

fn metadata_fields(text: &str, style: MetadataStyle) -> Option<MetadataFields> {
    let block = metadata_block(text, style)?;
    let mut fields = MetadataFields::default();
    for line in block.lines() {
        let trimmed = line.trim();
        if let Some(value) = parse_metadata_field(trimmed, "afad:") {
            fields.afad = Some(value);
        } else if let Some(value) = parse_metadata_field(trimmed, "version:") {
            fields.version = Some(value);
        } else if let Some(value) = parse_metadata_field(trimmed, "domain:") {
            fields.domain = Some(value);
        } else if let Some(value) = parse_metadata_field(trimmed, "updated:") {
            fields.updated = Some(value);
        } else if trimmed == "route:" {
            fields.has_route_section = true;
        } else if trimmed == "RETRIEVAL_HINTS:" {
            fields.has_retrieval_hints_section = true;
        } else if trimmed.starts_with("keywords:") {
            fields.has_keywords = true;
        } else if trimmed.starts_with("questions:") {
            fields.has_questions = true;
        }
    }

    Some(fields)
}

fn metadata_block(text: &str, style: MetadataStyle) -> Option<&str> {
    match style {
        MetadataStyle::Frontmatter => {
            let mut lines = text.lines();
            if lines.next()?.trim() != "---" {
                return None;
            }
            let mut offset: usize = 4;
            for line in lines {
                if line.trim() == "---" {
                    let end = offset.saturating_sub(1);
                    return Some(&text[4..end]);
                }
                offset += line.len() + 1;
            }
            None
        }
        MetadataStyle::HtmlComment => {
            let start = text.find("<!--")?;
            let end = text[start..].find("-->")?;
            Some(&text[start + 4..start + end])
        }
    }
}

fn frontmatter_version(text: &str) -> Option<String> {
    metadata_fields(text, MetadataStyle::Frontmatter)?.version
}

fn html_comment_version(text: &str) -> Option<String> {
    metadata_fields(text, MetadataStyle::HtmlComment)?.version
}

fn parse_metadata_field(line: &str, key: &str) -> Option<String> {
    let value = line.strip_prefix(key)?.trim();
    if value.is_empty() {
        return None;
    }

    let value = value.split('#').next().unwrap_or_default().trim();
    if let Some(value) = value
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
    {
        return Some(value.to_owned());
    }

    Some(value.to_owned())
}

impl MetadataStyle {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Frontmatter => "frontmatter",
            Self::HtmlComment => "HTML comment",
        }
    }
}
