#![allow(dead_code, unused_imports)]
// Shared integration-test support is compiled once per split test crate, so each crate naturally
// uses only a subset of these helpers and re-exports.

//! Black-box integration tests for the `htmlcut` CLI binary.

pub(crate) use std::fs;
pub(crate) use std::io::{Read, Write};
pub(crate) use std::net::TcpListener;
pub(crate) use std::num::NonZeroUsize;
pub(crate) use std::path::{Path, PathBuf};
pub(crate) use std::sync::{Arc, Mutex};
pub(crate) use std::thread;

pub(crate) use assert_cmd::Command;
pub(crate) use htmlcut_cli::{
    CATALOG_REPORT_SCHEMA_NAME, CATALOG_SCHEMA_VERSION, CatalogCommandReport, CliErrorCode,
    ERROR_COMMAND_REPORT_SCHEMA_NAME, ERROR_COMMAND_REPORT_SCHEMA_VERSION,
    EXTRACTION_COMMAND_REPORT_SCHEMA_NAME, EXTRACTION_COMMAND_REPORT_SCHEMA_VERSION,
    ErrorCommandReport, ErrorReportCode, ExtractionCommandReport,
    SCHEMA_COMMAND_REPORT_SCHEMA_NAME, SCHEMA_COMMAND_REPORT_SCHEMA_VERSION,
    SOURCE_INSPECTION_COMMAND_REPORT_SCHEMA_NAME, SOURCE_INSPECTION_COMMAND_REPORT_SCHEMA_VERSION,
    SchemaCommandReport, SourceInspectionCommandReport,
};
pub(crate) use htmlcut_core::{
    AttributeName, BoundaryRetention, DEFAULT_PREVIEW_CHARS, Diagnostic, ExtractionDefinition,
    ExtractionRequest, ExtractionSpec, FetchConnectTimeoutMs, FetchTimeoutMs, HttpUrl, MaxBytes,
    OutputOptions, PatternMode, RenderingOptions, RuntimeOptions, SelectionSpec, SelectorQuery,
    SliceBoundary, SliceSpec, SourceRequest, ValueSpec, WhitespaceMode, extract, inspect_source,
    preview_extraction,
    result::{ExtractionMatch, ExtractionMatchMetadata},
};
pub(crate) use htmlcut_tempdir::tempdir;
pub(crate) use predicates::prelude::*;

pub(crate) fn expected_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

pub(crate) fn expected_version_banner() -> String {
    format!(
        "HTMLCut {}\n{}\nschema registry: {}\nrepository: {}\n",
        expected_version(),
        env!("CARGO_PKG_DESCRIPTION"),
        htmlcut_core::HTMLCUT_JSON_SCHEMA_PROFILE,
        env!("CARGO_PKG_REPOSITORY")
    )
}

pub(crate) fn write_fixture(tempdir: &Path, name: &str, contents: &str) -> PathBuf {
    let path = tempdir.join(name);
    fs::write(&path, contents).expect("write fixture");
    path
}

pub(crate) fn source_request(path: &Path, base_url: Option<&str>) -> SourceRequest {
    let source = SourceRequest::file(path);
    base_url.map_or(source.clone(), |base_url| {
        source.with_base_url(http_url(base_url))
    })
}

pub(crate) fn runtime_options() -> RuntimeOptions {
    RuntimeOptions::default()
}

pub(crate) fn http_url(value: &str) -> HttpUrl {
    HttpUrl::parse(value).expect("http url")
}

pub(crate) fn max_bytes_limit(value: usize) -> MaxBytes {
    MaxBytes::new(value).expect("max bytes limit")
}

pub(crate) fn fetch_timeout_limit(value: u64) -> FetchTimeoutMs {
    FetchTimeoutMs::new(value).expect("fetch timeout")
}

pub(crate) fn fetch_connect_timeout_limit(value: u64) -> FetchConnectTimeoutMs {
    FetchConnectTimeoutMs::new(value).expect("fetch connect timeout")
}

pub(crate) fn extraction_output() -> OutputOptions {
    OutputOptions {
        preview_chars: NonZeroUsize::new(DEFAULT_PREVIEW_CHARS).expect("preview chars"),
        ..OutputOptions::default()
    }
}

pub(crate) fn selector_extraction(selector: &str) -> ExtractionSpec {
    ExtractionSpec::selector(SelectorQuery::new(selector).expect("selector"))
}

pub(crate) fn slice_extraction(
    from: &str,
    to: &str,
    mode: PatternMode,
    include_start: bool,
    include_end: bool,
) -> ExtractionSpec {
    let from = SliceBoundary::new(from).expect("slice boundary");
    let to = SliceBoundary::new(to).expect("slice boundary");
    let slice = match mode {
        PatternMode::Literal => SliceSpec::new(from, to),
        PatternMode::Regex => SliceSpec::regex(from, to, ""),
    }
    .with_boundary_retention(BoundaryRetention::from_flags(include_start, include_end));
    ExtractionSpec::slice(slice)
}

pub(crate) fn parse_extraction_report(
    assert: assert_cmd::assert::Assert,
) -> ExtractionCommandReport {
    let stdout = assert.get_output().stdout.clone();
    serde_json::from_slice(&stdout).expect("parse extraction report")
}

pub(crate) fn parse_source_inspection_report(
    assert: assert_cmd::assert::Assert,
) -> SourceInspectionCommandReport {
    let stdout = assert.get_output().stdout.clone();
    serde_json::from_slice(&stdout).expect("parse source inspection report")
}

pub(crate) fn parse_catalog_report(assert: assert_cmd::assert::Assert) -> CatalogCommandReport {
    let stdout = assert.get_output().stdout.clone();
    serde_json::from_slice(&stdout).expect("parse catalog report")
}

pub(crate) fn parse_error_report(assert: assert_cmd::assert::Assert) -> ErrorCommandReport {
    let stdout = assert.get_output().stdout.clone();
    serde_json::from_slice(&stdout).expect("parse error report")
}

pub(crate) fn parse_schema_report(assert: assert_cmd::assert::Assert) -> SchemaCommandReport {
    let stdout = assert.get_output().stdout.clone();
    serde_json::from_slice(&stdout).expect("parse schema report")
}

pub(crate) fn normalize_public_matches(matches: Vec<ExtractionMatch>) -> Vec<ExtractionMatch> {
    matches
        .into_iter()
        .map(|mut matched| {
            matched.value = normalize_public_json_value(matched.value);
            matched
        })
        .collect()
}

pub(crate) fn normalize_public_diagnostics(diagnostics: Vec<Diagnostic>) -> Vec<Diagnostic> {
    diagnostics
        .into_iter()
        .map(|mut diagnostic| {
            diagnostic.details = diagnostic.details.map(normalize_public_json_value);
            diagnostic
        })
        .collect()
}

fn normalize_public_json_value(value: serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(object) => serde_json::Value::Object(
            object
                .into_iter()
                .map(|(key, value)| (snake_case_key(&key), normalize_public_json_value(value)))
                .collect(),
        ),
        serde_json::Value::Array(values) => serde_json::Value::Array(
            values
                .into_iter()
                .map(normalize_public_json_value)
                .collect(),
        ),
        other => other,
    }
}

fn snake_case_key(key: &str) -> String {
    let mut normalized = String::with_capacity(key.len() + 4);
    let mut previous_was_underscore = false;

    for character in key.chars() {
        if character == '-' || character == ' ' {
            if !previous_was_underscore {
                normalized.push('_');
                previous_was_underscore = true;
            }
            continue;
        }

        if character.is_uppercase() {
            if !normalized.is_empty() && !previous_was_underscore {
                normalized.push('_');
            }
            for lowercase in character.to_lowercase() {
                normalized.push(lowercase);
            }
            previous_was_underscore = false;
            continue;
        }

        normalized.push(character);
        previous_was_underscore = character == '_';
    }

    normalized
}
