use std::fs;
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};

use htmlcut_cli::{
    EXTRACTION_COMMAND_REPORT_SCHEMA_NAME, EXTRACTION_COMMAND_REPORT_SCHEMA_VERSION,
    ExtractionCommandReport, SOURCE_INSPECTION_COMMAND_REPORT_SCHEMA_NAME,
    SOURCE_INSPECTION_COMMAND_REPORT_SCHEMA_VERSION, SourceInspectionCommandReport, run,
};
use htmlcut_core::{
    AttributeName, DEFAULT_PREVIEW_CHARS, ExtractionRequest, ExtractionSpec, NormalizationOptions,
    OutputOptions, PatternMode, RuntimeOptions, SelectionSpec, SelectorQuery, SliceBoundary,
    SliceSpec, SourceRequest, ValueSpec, WhitespaceMode, extract, inspect_source,
    preview_extraction,
};
use htmlcut_tempdir::tempdir;
use url::Url;

fn write_fixture(tempdir: &Path, name: &str, contents: &str) -> PathBuf {
    let path = tempdir.join(name);
    fs::write(&path, contents).expect("write fixture");
    path
}

fn source_request(path: &Path, base_url: Option<&str>) -> SourceRequest {
    let source = SourceRequest::file(path);
    base_url.map_or(source.clone(), |base_url| {
        source.with_base_url(Url::parse(base_url).expect("base url"))
    })
}

fn runtime_options() -> RuntimeOptions {
    RuntimeOptions::default()
}

fn extraction_output(include_source_text: bool, preview_chars: usize) -> OutputOptions {
    OutputOptions {
        include_source_text,
        include_html: true,
        include_text: true,
        preview_chars: NonZeroUsize::new(preview_chars).expect("preview chars"),
    }
}

fn selector_extraction(selector: &str) -> ExtractionSpec {
    ExtractionSpec::selector(SelectorQuery::new(selector).expect("selector"))
}

fn slice_extraction(
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

fn run_cli_json(args: &[String]) -> String {
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

fn parse_extraction_report(args: &[String]) -> ExtractionCommandReport {
    serde_json::from_str(&run_cli_json(args)).expect("parse extraction report")
}

fn parse_source_inspection_report(args: &[String]) -> SourceInspectionCommandReport {
    serde_json::from_str(&run_cli_json(args)).expect("parse source inspection report")
}

enum ExtractionExecution {
    Extract,
    Preview,
}

struct ExtractionParityCase {
    name: &'static str,
    args: Vec<String>,
    command: &'static str,
    request: ExtractionRequest,
    runtime: RuntimeOptions,
    execution: ExtractionExecution,
}

struct SourceInspectionParityCase {
    name: &'static str,
    args: Vec<String>,
    command: &'static str,
    source: SourceRequest,
    runtime: RuntimeOptions,
    sample_limit: usize,
    include_source_text: bool,
}

fn assert_extraction_parity(case: &ExtractionParityCase) {
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

fn assert_source_inspection_parity(case: &SourceInspectionParityCase) {
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

#[test]
fn extraction_and_preview_commands_stay_in_lockstep_with_core() {
    let tempdir = tempdir().expect("tempdir");
    let card_path = write_fixture(
        tempdir.path(),
        "matrix cards.html",
        "<html><head><title>Matrix Cards</title></head><body><article class=\"card primary\"><p>Alpha   Beta</p></article><article class=\"card secondary\"><p>Gamma</p></article><div>START::First::END</div><div>START::Second::END</div></body></html>",
    );
    let base_path = write_fixture(
        tempdir.path(),
        "matrix base.html",
        "<html><head><title>Matrix Base</title><base href=\"../content/\"></head><body><article><a class=\"cta\" href=\"guide/start.html\">Guide</a></article></body></html>",
    );

    let cases = vec![
        ExtractionParityCase {
            name: "select nth normalized text",
            args: vec![
                "select".to_owned(),
                card_path.to_string_lossy().into_owned(),
                "--css".to_owned(),
                "article.card p".to_owned(),
                "--match".to_owned(),
                "nth".to_owned(),
                "--index".to_owned(),
                "1".to_owned(),
                "--whitespace".to_owned(),
                "normalize".to_owned(),
                "--output".to_owned(),
                "json".to_owned(),
            ],
            command: "select",
            request: {
                let mut request = ExtractionRequest::new(
                    source_request(&card_path, None),
                    selector_extraction("article.card p")
                        .with_selection(SelectionSpec::nth(
                            NonZeroUsize::new(1).expect("match index"),
                        ))
                        .with_value(ValueSpec::Text),
                );
                request.normalization = NormalizationOptions {
                    whitespace: WhitespaceMode::Normalize,
                    rewrite_urls: false,
                };
                request.output = extraction_output(false, DEFAULT_PREVIEW_CHARS);
                request
            },
            runtime: runtime_options(),
            execution: ExtractionExecution::Extract,
        },
        ExtractionParityCase {
            name: "select attribute rewrite honors effective document base",
            args: vec![
                "select".to_owned(),
                base_path.to_string_lossy().into_owned(),
                "--css".to_owned(),
                "article a.cta".to_owned(),
                "--value".to_owned(),
                "attribute".to_owned(),
                "--attribute".to_owned(),
                "href".to_owned(),
                "--rewrite-urls".to_owned(),
                "--base-url".to_owned(),
                "https://example.com/docs/start.html".to_owned(),
                "--output".to_owned(),
                "json".to_owned(),
            ],
            command: "select",
            request: {
                let mut request = ExtractionRequest::new(
                    source_request(&base_path, Some("https://example.com/docs/start.html")),
                    selector_extraction("article a.cta")
                        .with_selection(SelectionSpec::First)
                        .with_value(ValueSpec::Attribute {
                            name: AttributeName::new("href").expect("attribute name"),
                        }),
                );
                request.normalization = NormalizationOptions {
                    whitespace: WhitespaceMode::Preserve,
                    rewrite_urls: true,
                };
                request.output = extraction_output(false, DEFAULT_PREVIEW_CHARS);
                request
            },
            runtime: runtime_options(),
            execution: ExtractionExecution::Extract,
        },
        ExtractionParityCase {
            name: "select single exact one keeps core parity",
            args: vec![
                "select".to_owned(),
                base_path.to_string_lossy().into_owned(),
                "--css".to_owned(),
                "article".to_owned(),
                "--match".to_owned(),
                "single".to_owned(),
                "--output".to_owned(),
                "json".to_owned(),
            ],
            command: "select",
            request: {
                let mut request = ExtractionRequest::new(
                    source_request(&base_path, None),
                    selector_extraction("article")
                        .with_selection(SelectionSpec::single())
                        .with_value(ValueSpec::Text),
                );
                request.normalization = NormalizationOptions {
                    whitespace: WhitespaceMode::Preserve,
                    rewrite_urls: false,
                };
                request.output = extraction_output(false, DEFAULT_PREVIEW_CHARS);
                request
            },
            runtime: runtime_options(),
            execution: ExtractionExecution::Extract,
        },
        ExtractionParityCase {
            name: "slice regex outer all",
            args: vec![
                "slice".to_owned(),
                card_path.to_string_lossy().into_owned(),
                "--from".to_owned(),
                "START::[A-Za-z]+".to_owned(),
                "--to".to_owned(),
                "::END".to_owned(),
                "--pattern".to_owned(),
                "regex".to_owned(),
                "--include-start".to_owned(),
                "--include-end".to_owned(),
                "--match".to_owned(),
                "all".to_owned(),
                "--output".to_owned(),
                "json".to_owned(),
            ],
            command: "slice",
            request: {
                let mut request = ExtractionRequest::new(
                    source_request(&card_path, None),
                    slice_extraction("START::[A-Za-z]+", "::END", PatternMode::Regex, true, true)
                        .with_selection(SelectionSpec::All)
                        .with_value(ValueSpec::Text),
                );
                request.normalization = NormalizationOptions {
                    whitespace: WhitespaceMode::Preserve,
                    rewrite_urls: false,
                };
                request.output = extraction_output(false, DEFAULT_PREVIEW_CHARS);
                request
            },
            runtime: runtime_options(),
            execution: ExtractionExecution::Extract,
        },
        ExtractionParityCase {
            name: "slice literal start-inclusive parity",
            args: vec![
                "slice".to_owned(),
                card_path.to_string_lossy().into_owned(),
                "--from".to_owned(),
                "START::".to_owned(),
                "--to".to_owned(),
                "::END".to_owned(),
                "--include-start".to_owned(),
                "--match".to_owned(),
                "nth".to_owned(),
                "--index".to_owned(),
                "2".to_owned(),
                "--output".to_owned(),
                "json".to_owned(),
            ],
            command: "slice",
            request: {
                let mut request = ExtractionRequest::new(
                    source_request(&card_path, None),
                    slice_extraction("START::", "::END", PatternMode::Literal, true, false)
                        .with_selection(SelectionSpec::nth(
                            NonZeroUsize::new(2).expect("match index"),
                        ))
                        .with_value(ValueSpec::Text),
                );
                request.normalization = NormalizationOptions {
                    whitespace: WhitespaceMode::Preserve,
                    rewrite_urls: false,
                };
                request.output = extraction_output(false, DEFAULT_PREVIEW_CHARS);
                request
            },
            runtime: runtime_options(),
            execution: ExtractionExecution::Extract,
        },
        ExtractionParityCase {
            name: "inspect select preview keeps core metadata",
            args: vec![
                "inspect".to_owned(),
                "select".to_owned(),
                base_path.to_string_lossy().into_owned(),
                "--css".to_owned(),
                "article a.cta".to_owned(),
                "--match".to_owned(),
                "all".to_owned(),
                "--rewrite-urls".to_owned(),
                "--base-url".to_owned(),
                "https://example.com/docs/start.html".to_owned(),
                "--preview-chars".to_owned(),
                "48".to_owned(),
                "--include-source-text".to_owned(),
            ],
            command: "inspect-select",
            request: {
                let mut request = ExtractionRequest::new(
                    source_request(&base_path, Some("https://example.com/docs/start.html")),
                    selector_extraction("article a.cta")
                        .with_selection(SelectionSpec::All)
                        .with_value(ValueSpec::Structured),
                );
                request.normalization = NormalizationOptions {
                    whitespace: WhitespaceMode::Preserve,
                    rewrite_urls: true,
                };
                request.output = extraction_output(true, 48);
                request
            },
            runtime: runtime_options(),
            execution: ExtractionExecution::Preview,
        },
        ExtractionParityCase {
            name: "inspect slice preview keeps start-only inclusion parity",
            args: vec![
                "inspect".to_owned(),
                "slice".to_owned(),
                card_path.to_string_lossy().into_owned(),
                "--from".to_owned(),
                "START::".to_owned(),
                "--to".to_owned(),
                "::END".to_owned(),
                "--include-start".to_owned(),
                "--match".to_owned(),
                "nth".to_owned(),
                "--index".to_owned(),
                "1".to_owned(),
                "--preview-chars".to_owned(),
                "48".to_owned(),
                "--include-source-text".to_owned(),
            ],
            command: "inspect-slice",
            request: {
                let mut request = ExtractionRequest::new(
                    source_request(&card_path, None),
                    slice_extraction("START::", "::END", PatternMode::Literal, true, false)
                        .with_selection(SelectionSpec::nth(
                            NonZeroUsize::new(1).expect("match index"),
                        ))
                        .with_value(ValueSpec::Structured),
                );
                request.normalization = NormalizationOptions {
                    whitespace: WhitespaceMode::Preserve,
                    rewrite_urls: false,
                };
                request.output = extraction_output(true, 48);
                request
            },
            runtime: runtime_options(),
            execution: ExtractionExecution::Preview,
        },
        ExtractionParityCase {
            name: "inspect slice preview keeps core metadata",
            args: vec![
                "inspect".to_owned(),
                "slice".to_owned(),
                card_path.to_string_lossy().into_owned(),
                "--from".to_owned(),
                "START::[A-Za-z]+".to_owned(),
                "--to".to_owned(),
                "::END".to_owned(),
                "--pattern".to_owned(),
                "regex".to_owned(),
                "--include-start".to_owned(),
                "--include-end".to_owned(),
                "--match".to_owned(),
                "all".to_owned(),
                "--preview-chars".to_owned(),
                "48".to_owned(),
                "--include-source-text".to_owned(),
            ],
            command: "inspect-slice",
            request: {
                let mut request = ExtractionRequest::new(
                    source_request(&card_path, None),
                    slice_extraction("START::[A-Za-z]+", "::END", PatternMode::Regex, true, true)
                        .with_selection(SelectionSpec::All)
                        .with_value(ValueSpec::Structured),
                );
                request.normalization = NormalizationOptions {
                    whitespace: WhitespaceMode::Preserve,
                    rewrite_urls: false,
                };
                request.output = extraction_output(true, 48);
                request
            },
            runtime: runtime_options(),
            execution: ExtractionExecution::Preview,
        },
    ];

    for case in &cases {
        assert_extraction_parity(case);
    }
}

#[test]
fn source_inspection_commands_stay_in_lockstep_with_core() {
    let tempdir = tempdir().expect("tempdir");
    let input_path = write_fixture(
        tempdir.path(),
        "inspect source matrix.html",
        "<html><head><title>Inspect Matrix</title><base href=\"../content/\"></head><body><main><h1>Heading</h1><h2>Details</h2><a href=\"guide.html\">Guide</a><a href=\"/docs\">Docs</a><p>Alpha Beta Gamma</p></main></body></html>",
    );

    let case = SourceInspectionParityCase {
        name: "inspect source with effective base and source text",
        args: vec![
            "inspect".to_owned(),
            "source".to_owned(),
            input_path.to_string_lossy().into_owned(),
            "--base-url".to_owned(),
            "https://example.com/docs/start.html".to_owned(),
            "--sample-limit".to_owned(),
            "3".to_owned(),
            "--include-source-text".to_owned(),
        ],
        command: "inspect-source",
        source: source_request(&input_path, Some("https://example.com/docs/start.html")),
        runtime: runtime_options(),
        sample_limit: 3,
        include_source_text: true,
    };

    assert_source_inspection_parity(&case);
}
