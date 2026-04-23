pub(super) use std::fs;
pub(super) use std::num::NonZeroUsize;
pub(super) use std::path::{Path, PathBuf};

pub(super) use htmlcut_cli::{
    EXTRACTION_COMMAND_REPORT_SCHEMA_NAME, EXTRACTION_COMMAND_REPORT_SCHEMA_VERSION,
    ExtractionCommandReport, SOURCE_INSPECTION_COMMAND_REPORT_SCHEMA_NAME,
    SOURCE_INSPECTION_COMMAND_REPORT_SCHEMA_VERSION, SourceInspectionCommandReport, run,
};
pub(super) use htmlcut_core::{
    AttributeName, DEFAULT_PREVIEW_CHARS, ExtractionRequest, ExtractionSpec, NormalizationOptions,
    OutputOptions, PatternMode, RuntimeOptions, SelectionSpec, SelectorQuery, SliceBoundary,
    SliceSpec, SourceRequest, ValueSpec, WhitespaceMode, extract, inspect_source,
    preview_extraction,
};
pub(super) use htmlcut_tempdir::tempdir;
pub(super) use url::Url;

pub(super) fn write_fixture(tempdir: &Path, name: &str, contents: &str) -> PathBuf {
    let path = tempdir.join(name);
    fs::write(&path, contents).expect("write fixture");
    path
}

pub(super) fn source_request(path: &Path, base_url: Option<&str>) -> SourceRequest {
    let source = SourceRequest::file(path);
    base_url.map_or(source.clone(), |base_url| {
        source.with_base_url(Url::parse(base_url).expect("base url"))
    })
}

pub(super) fn runtime_options() -> RuntimeOptions {
    RuntimeOptions::default()
}

pub(super) fn extraction_output(include_source_text: bool, preview_chars: usize) -> OutputOptions {
    OutputOptions {
        include_source_text,
        include_html: true,
        include_text: true,
        preview_chars: NonZeroUsize::new(preview_chars).expect("preview chars"),
    }
}

pub(super) fn selector_extraction(selector: &str) -> ExtractionSpec {
    ExtractionSpec::selector(SelectorQuery::new(selector).expect("selector"))
}

pub(super) fn slice_extraction(
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
        PatternMode::Regex => SliceSpec::regex(from, to, htmlcut_core::DEFAULT_REGEX_FLAGS),
    }
    .with_boundary_inclusion(include_start, include_end);
    ExtractionSpec::slice(slice)
}

pub(super) fn run_cli_json(args: &[String]) -> String {
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let exit_code = run(
        std::iter::once("htmlcut".to_owned()).chain(args.iter().cloned()),
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(
        exit_code,
        0,
        "CLI failed with stderr: {}",
        String::from_utf8_lossy(&stderr)
    );
    assert!(
        stderr.is_empty(),
        "Expected no stderr output, got: {}",
        String::from_utf8_lossy(&stderr)
    );

    String::from_utf8(stdout).expect("utf8 stdout")
}

pub(super) fn parse_extraction_report(args: &[String]) -> ExtractionCommandReport {
    serde_json::from_str(&run_cli_json(args)).expect("parse extraction report")
}

pub(super) fn parse_source_inspection_report(args: &[String]) -> SourceInspectionCommandReport {
    serde_json::from_str(&run_cli_json(args)).expect("parse source inspection report")
}

pub(super) enum ExtractionExecution {
    Extract,
    Preview,
}

pub(super) struct ExtractionParityCase {
    pub(super) name: &'static str,
    pub(super) args: Vec<String>,
    pub(super) command: &'static str,
    pub(super) request: ExtractionRequest,
    pub(super) runtime: RuntimeOptions,
    pub(super) execution: ExtractionExecution,
}

pub(super) struct SourceInspectionParityCase {
    pub(super) name: &'static str,
    pub(super) args: Vec<String>,
    pub(super) command: &'static str,
    pub(super) source: SourceRequest,
    pub(super) runtime: RuntimeOptions,
    pub(super) sample_limit: usize,
    pub(super) include_source_text: bool,
}

pub(super) fn assert_extraction_parity(case: &ExtractionParityCase) {
    let expected = match case.execution {
        ExtractionExecution::Extract => extract(&case.request, &case.runtime),
        ExtractionExecution::Preview => preview_extraction(&case.request, &case.runtime),
    };
    assert!(expected.ok, "core execution failed for {}", case.name);

    let report = parse_extraction_report(&case.args);
    assert_eq!(report.tool, "htmlcut", "{}", case.name);
    assert_eq!(report.engine, "htmlcut-core", "{}", case.name);
    assert_eq!(report.version, env!("CARGO_PKG_VERSION"), "{}", case.name);
    assert_eq!(
        report.schema_name, EXTRACTION_COMMAND_REPORT_SCHEMA_NAME,
        "{}",
        case.name
    );
    assert_eq!(report.command, case.command, "{}", case.name);
    assert_eq!(report.operation_id, expected.operation_id, "{}", case.name);
    assert_eq!(report.ok, expected.ok, "{}", case.name);
    assert_eq!(
        report.schema_version, EXTRACTION_COMMAND_REPORT_SCHEMA_VERSION,
        "{}",
        case.name
    );
    assert_eq!(report.source, expected.source, "{}", case.name);
    assert_eq!(report.extraction, expected.extraction, "{}", case.name);
    assert_eq!(
        report.stats.candidate_count, expected.stats.candidate_count,
        "{}",
        case.name
    );
    assert_eq!(
        report.stats.match_count, expected.stats.match_count,
        "{}",
        case.name
    );
    assert_eq!(
        report.document_title, expected.document_title,
        "{}",
        case.name
    );
    assert_eq!(report.matches, expected.matches, "{}", case.name);
    assert_eq!(report.diagnostics, expected.diagnostics, "{}", case.name);
    assert!(report.bundle.is_none(), "{}", case.name);
}

pub(super) fn assert_source_inspection_parity(case: &SourceInspectionParityCase) {
    let expected = inspect_source(
        &case.source,
        &case.runtime,
        &htmlcut_core::InspectionOptions {
            include_source_text: case.include_source_text,
            sample_limit: case.sample_limit,
        },
    );
    assert!(expected.ok, "core inspection failed for {}", case.name);

    let report = parse_source_inspection_report(&case.args);
    assert_eq!(report.tool, "htmlcut", "{}", case.name);
    assert_eq!(report.engine, "htmlcut-core", "{}", case.name);
    assert_eq!(report.version, env!("CARGO_PKG_VERSION"), "{}", case.name);
    assert_eq!(
        report.schema_name, SOURCE_INSPECTION_COMMAND_REPORT_SCHEMA_NAME,
        "{}",
        case.name
    );
    assert_eq!(report.command, case.command, "{}", case.name);
    assert_eq!(report.operation_id, expected.operation_id, "{}", case.name);
    assert_eq!(report.ok, expected.ok, "{}", case.name);
    assert_eq!(
        report.schema_version, SOURCE_INSPECTION_COMMAND_REPORT_SCHEMA_VERSION,
        "{}",
        case.name
    );
    assert_eq!(report.source, expected.source, "{}", case.name);
    assert_eq!(report.document, expected.document, "{}", case.name);
    assert_eq!(report.diagnostics, expected.diagnostics, "{}", case.name);
}
