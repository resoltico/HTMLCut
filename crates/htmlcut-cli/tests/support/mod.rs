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
    AttributeName, DEFAULT_PREVIEW_CHARS, ExtractionDefinition, ExtractionRequest, ExtractionSpec,
    NormalizationOptions, OutputOptions, PatternMode, RuntimeOptions, SelectionSpec, SelectorQuery,
    SliceBoundary, SliceSpec, SourceRequest, ValueSpec, WhitespaceMode, extract, inspect_source,
    preview_extraction, result::ExtractionMatchMetadata,
};
pub(crate) use htmlcut_tempdir::tempdir;
pub(crate) use predicates::prelude::*;
pub(crate) use url::Url;

pub(crate) fn expected_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

pub(crate) fn expected_version_banner() -> String {
    format!(
        "HTMLCut {}\n{}\nengine: htmlcut-core\nschema-profile: {}\nrepository: {}\n",
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
        source.with_base_url(Url::parse(base_url).expect("base url"))
    })
}

pub(crate) fn runtime_options() -> RuntimeOptions {
    RuntimeOptions::default()
}

pub(crate) fn extraction_output() -> OutputOptions {
    OutputOptions {
        include_source_text: false,
        include_html: true,
        include_text: true,
        preview_chars: NonZeroUsize::new(DEFAULT_PREVIEW_CHARS).expect("preview chars"),
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
    .with_boundary_inclusion(include_start, include_end);
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
