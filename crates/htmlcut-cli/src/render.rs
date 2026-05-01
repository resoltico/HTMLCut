#[cfg(test)]
use std::cell::Cell;
use std::path::Path;

mod discovery;
mod inspection;

use htmlcut_core::{ValueType, result::ExtractionMatch};
use serde::Serialize;
use serde_json::Value;

use crate::args::CliOutputMode;
use crate::error::{CliError, internal_error, output_error};
use crate::file_output::{FileWriteMode, prepare_bundle_directory, write_text_file};
use crate::model::{BundlePaths, CliErrorCode, ExtractionCommandReport};

#[cfg(test)]
thread_local! {
    static JSON_RENDER_FAILURE_OVERRIDE: Cell<bool> = const { Cell::new(false) };
}

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
    write_mode: FileWriteMode,
) -> Result<(), CliError> {
    let bundle_dir = Path::new(&bundle.dir);
    let html_document = wrap_html_document(report)?;
    let text_payload = render_text_payload(report)?;
    let report_payload = to_pretty_json(report)?;

    prepare_bundle_directory(bundle_dir, write_mode).map_err(|error| {
        let code = if matches!(write_mode, FileWriteMode::CreateFresh) && bundle_dir.exists() {
            CliErrorCode::BundlePathExists
        } else {
            CliErrorCode::BundleDirectoryCreateFailed
        };
        output_error(
            code,
            format!("Could not create bundle directory {}: {error}", bundle.dir),
        )
    })?;
    write_text_file(Path::new(&bundle.html), &html_document, write_mode).map_err(|error| {
        output_error(
            CliErrorCode::BundleHtmlWriteFailed,
            format!("Could not write {}: {error}", bundle.html),
        )
    })?;
    write_text_file(Path::new(&bundle.text), &text_payload, write_mode).map_err(|error| {
        output_error(
            CliErrorCode::BundleTextWriteFailed,
            format!("Could not write {}: {error}", bundle.text),
        )
    })?;
    write_text_file(
        Path::new(&bundle.report),
        &format!("{report_payload}\n"),
        write_mode,
    )
    .map_err(|error| {
        output_error(
            CliErrorCode::BundleReportWriteFailed,
            format!("Could not write {}: {error}", bundle.report),
        )
    })?;
    Ok(())
}

pub(crate) fn render_extraction_output(
    report: &ExtractionCommandReport,
    output: CliOutputMode,
) -> Result<Option<String>, CliError> {
    match output {
        CliOutputMode::Text => render_text_payload(report).map(Some),
        CliOutputMode::Html => render_html_payload(report).map(Some),
        CliOutputMode::Json => to_pretty_json(report).map(Some),
        CliOutputMode::None => Ok(None),
    }
}

pub(crate) fn render_text_payload(report: &ExtractionCommandReport) -> Result<String, CliError> {
    report
        .matches
        .iter()
        .map(render_match_as_text)
        .collect::<Result<Vec<_>, _>>()
        .map(|parts| parts.join("\n\n"))
}

pub(crate) fn render_html_payload(report: &ExtractionCommandReport) -> Result<String, CliError> {
    report
        .matches
        .iter()
        .map(render_match_as_html)
        .collect::<Result<Vec<_>, _>>()
        .map(|parts| parts.join("\n\n"))
}

pub(crate) fn render_match_as_text(matched: &ExtractionMatch) -> Result<String, CliError> {
    if let Value::String(text) = &matched.value {
        return Ok(text.clone());
    }

    render_json_string(&matched.value, "extracted match payload")
}

pub(crate) fn render_match_as_html(matched: &ExtractionMatch) -> Result<String, CliError> {
    if let Value::String(html) = &matched.value
        && (matched.value_type == ValueType::InnerHtml
            || matched.value_type == ValueType::OuterHtml)
    {
        return Ok(html.clone());
    }

    match matched.html.as_ref() {
        Some(html) => Ok(html.clone()),
        None => {
            render_match_as_text(matched).map(|text| format!("<pre>{}</pre>", escape_html(&text)))
        }
    }
}

pub(crate) fn wrap_html_document(report: &ExtractionCommandReport) -> Result<String, CliError> {
    if report.matches.len() == 1
        && let Some(Value::String(html)) = report.matches.first().map(|matched| &matched.value)
        && looks_like_document(html)
    {
        return Ok(html.clone());
    }

    let body = report
        .matches
        .iter()
        .map(|matched| {
            render_match_as_html(matched).map(|html| {
                format!(
                    "<section data-match-index=\"{}\">{}</section>",
                    matched.index, html
                )
            })
        })
        .collect::<Result<Vec<_>, _>>()?
        .join("\n\n");

    Ok(format!(
        "<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n  <meta charset=\"utf-8\">\n  <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n  <title>{}</title>\n  <style>\n    body {{ font-family: ui-serif, Georgia, serif; margin: 2rem auto; max-width: 72rem; padding: 0 1.25rem 3rem; line-height: 1.6; }}\n    section + section {{ border-top: 1px solid #d6d6d6; margin-top: 2rem; padding-top: 2rem; }}\n  </style>\n</head>\n<body>\n{}\n</body>\n</html>\n",
        escape_html(&bundle_document_title(report)),
        body
    ))
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

pub(crate) fn to_pretty_json<T: Serialize>(value: &T) -> Result<String, CliError> {
    render_json_string(value, "CLI JSON payload")
}

pub(crate) fn render_json_string<T: Serialize>(
    value: &T,
    context: &str,
) -> Result<String, CliError> {
    render_pretty_json(value).map_err(|error| json_render_error(context, error))
}

fn json_render_error(context: &str, error: serde_json::Error) -> CliError {
    internal_error(
        CliErrorCode::JsonRenderFailed,
        format!("Could not render {context}: {error}"),
    )
}

fn render_pretty_json<T: Serialize>(value: &T) -> Result<String, serde_json::Error> {
    #[cfg(test)]
    if JSON_RENDER_FAILURE_OVERRIDE.with(Cell::get) {
        return Err(serde_json::Error::io(std::io::Error::other(
            "synthetic JSON render failure",
        )));
    }

    serde_json::to_string_pretty(value)
}

#[cfg(test)]
pub(crate) fn with_json_render_failure_for_tests<T>(operation: impl FnOnce() -> T) -> T {
    struct ResetJsonRenderFailure;

    impl Drop for ResetJsonRenderFailure {
        fn drop(&mut self) {
            JSON_RENDER_FAILURE_OVERRIDE.with(|enabled| enabled.set(false));
        }
    }

    JSON_RENDER_FAILURE_OVERRIDE.with(|enabled| enabled.set(true));
    let _reset = ResetJsonRenderFailure;
    operation()
}
