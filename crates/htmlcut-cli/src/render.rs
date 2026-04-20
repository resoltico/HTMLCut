use std::fs;
use std::path::Path;

mod discovery;
mod inspection;

use htmlcut_core::{ValueType, result::ExtractionMatch};
use serde::Serialize;
use serde_json::Value;

use crate::args::CliOutputMode;
use crate::error::{CliError, output_error};
use crate::model::{BundlePaths, ExtractionCommandReport};

#[cfg(test)]
pub(crate) use self::discovery::render_catalog_surface;
pub(crate) use self::discovery::{render_catalog_text, render_schema_text};
pub(crate) use self::inspection::{
    build_human_diagnostic_stderr_lines, build_source_inspection_verbose_lines,
    build_source_load_error_lines, build_verbose_lines, fallback_document_title,
    render_preview_text, render_source_inspection_text,
};
#[cfg(test)]
pub(crate) use self::inspection::{
    compact_inline_preview, render_attribute_summary, render_diagnostic_level,
    render_preview_location, render_preview_match_lines, render_range_summary, render_source_kind,
    render_text_preview,
};

pub(crate) fn get_bundle_paths(dir: &Path) -> BundlePaths {
    let dir = dir.canonicalize().unwrap_or_else(|_| dir.to_path_buf());
    BundlePaths {
        dir: dir.to_string_lossy().into_owned(),
        html: dir.join("selection.html").to_string_lossy().into_owned(),
        text: dir.join("selection.txt").to_string_lossy().into_owned(),
        report: dir.join("report.json").to_string_lossy().into_owned(),
    }
}

pub(crate) fn write_bundle(
    report: &ExtractionCommandReport,
    bundle: &BundlePaths,
) -> Result<(), CliError> {
    fs::create_dir_all(&bundle.dir).map_err(|error| {
        output_error(
            "CLI_BUNDLE_DIRECTORY_CREATE_FAILED",
            format!("Could not create bundle directory {}: {error}", bundle.dir),
        )
    })?;
    fs::write(&bundle.html, wrap_html_document(report)).map_err(|error| {
        output_error(
            "CLI_BUNDLE_HTML_WRITE_FAILED",
            format!("Could not write {}: {error}", bundle.html),
        )
    })?;
    fs::write(&bundle.text, render_text_payload(report)).map_err(|error| {
        output_error(
            "CLI_BUNDLE_TEXT_WRITE_FAILED",
            format!("Could not write {}: {error}", bundle.text),
        )
    })?;
    fs::write(&bundle.report, format!("{}\n", to_pretty_json(report))).map_err(|error| {
        output_error(
            "CLI_BUNDLE_REPORT_WRITE_FAILED",
            format!("Could not write {}: {error}", bundle.report),
        )
    })?;
    Ok(())
}

pub(crate) fn render_extraction_output(
    report: &ExtractionCommandReport,
    output: CliOutputMode,
) -> Option<String> {
    match output {
        CliOutputMode::Text => Some(render_text_payload(report)),
        CliOutputMode::Html => Some(render_html_payload(report)),
        CliOutputMode::Json => Some(to_pretty_json(report)),
        CliOutputMode::None => None,
    }
}

pub(crate) fn render_text_payload(report: &ExtractionCommandReport) -> String {
    report
        .matches
        .iter()
        .map(render_match_as_text)
        .collect::<Vec<_>>()
        .join("\n\n")
}

pub(crate) fn render_html_payload(report: &ExtractionCommandReport) -> String {
    report
        .matches
        .iter()
        .map(render_match_as_html)
        .collect::<Vec<_>>()
        .join("\n\n")
}

pub(crate) fn render_match_as_text(matched: &ExtractionMatch) -> String {
    if let Value::String(text) = &matched.value {
        return text.clone();
    }

    serde_json::to_string_pretty(&matched.value)
        .expect("serde_json::Value should always serialize to pretty JSON")
}

pub(crate) fn render_match_as_html(matched: &ExtractionMatch) -> String {
    if let Value::String(html) = &matched.value
        && (matched.value_type == ValueType::InnerHtml
            || matched.value_type == ValueType::OuterHtml)
    {
        return html.clone();
    }

    matched
        .html
        .clone()
        .unwrap_or_else(|| format!("<pre>{}</pre>", escape_html(&render_match_as_text(matched))))
}

pub(crate) fn wrap_html_document(report: &ExtractionCommandReport) -> String {
    if report.matches.len() == 1
        && let Some(Value::String(html)) = report.matches.first().map(|matched| &matched.value)
        && looks_like_document(html)
    {
        return html.clone();
    }

    let body = report
        .matches
        .iter()
        .map(|matched| {
            format!(
                "<section data-match-index=\"{}\">{}</section>",
                matched.index,
                render_match_as_html(matched)
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n");

    format!(
        "<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n  <meta charset=\"utf-8\">\n  <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n  <title>{}</title>\n  <style>\n    body {{ font-family: ui-serif, Georgia, serif; margin: 2rem auto; max-width: 72rem; padding: 0 1.25rem 3rem; line-height: 1.6; }}\n    section + section {{ border-top: 1px solid #d6d6d6; margin-top: 2rem; padding-top: 2rem; }}\n  </style>\n</head>\n<body>\n{}\n</body>\n</html>\n",
        escape_html(&bundle_document_title(report)),
        body
    )
}

pub(crate) fn looks_like_document(fragment: &str) -> bool {
    let trimmed = fragment.trim_start().to_ascii_lowercase();
    trimmed.starts_with("<!doctype") || trimmed.starts_with("<html")
}

pub(crate) fn escape_html(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

pub(crate) fn bundle_document_title(report: &ExtractionCommandReport) -> String {
    report
        .document_title
        .clone()
        .filter(|title| !title.trim().is_empty())
        .unwrap_or_else(|| fallback_document_title(&report.source))
}

pub(crate) fn to_pretty_json<T: Serialize>(value: &T) -> String {
    serde_json::to_string_pretty(value)
        .expect("HTMLCut CLI reports should always serialize to pretty JSON")
}
