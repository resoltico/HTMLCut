//! Black-box integration tests for the `htmlcut` CLI binary.

use std::fs;
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use htmlcut_cli::{
    CATALOG_REPORT_SCHEMA_NAME, CATALOG_SCHEMA_VERSION, CatalogCommandReport,
    EXTRACTION_COMMAND_REPORT_SCHEMA_NAME, EXTRACTION_COMMAND_REPORT_SCHEMA_VERSION,
    ExtractionCommandReport, SCHEMA_COMMAND_REPORT_SCHEMA_NAME,
    SCHEMA_COMMAND_REPORT_SCHEMA_VERSION, SOURCE_INSPECTION_COMMAND_REPORT_SCHEMA_NAME,
    SOURCE_INSPECTION_COMMAND_REPORT_SCHEMA_VERSION, SchemaCommandReport,
    SourceInspectionCommandReport,
};
use htmlcut_core::{
    AttributeName, DEFAULT_PREVIEW_CHARS, ExtractionDefinition, ExtractionRequest, ExtractionSpec,
    NormalizationOptions, OutputOptions, PatternMode, RuntimeOptions, SelectionSpec, SelectorQuery,
    SliceBoundary, SliceSpec, SourceRequest, ValueSpec, WhitespaceMode, extract, inspect_source,
    preview_extraction, result::ExtractionMatchMetadata,
};
use predicates::prelude::*;
use tempfile::tempdir;
use url::Url;

fn expected_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

fn expected_version_banner() -> String {
    format!(
        "htmlcut {}\n{}\nengine: htmlcut-core\nschema-profile: {}\nrepository: {}\n",
        expected_version(),
        env!("CARGO_PKG_DESCRIPTION"),
        htmlcut_core::HTMLCUT_JSON_SCHEMA_PROFILE,
        env!("CARGO_PKG_REPOSITORY")
    )
}

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

fn extraction_output() -> OutputOptions {
    OutputOptions {
        include_source_text: false,
        include_html: true,
        include_text: true,
        preview_chars: NonZeroUsize::new(DEFAULT_PREVIEW_CHARS).expect("preview chars"),
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

fn parse_extraction_report(assert: assert_cmd::assert::Assert) -> ExtractionCommandReport {
    let stdout = assert.get_output().stdout.clone();
    serde_json::from_slice(&stdout).expect("parse extraction report")
}

fn parse_source_inspection_report(
    assert: assert_cmd::assert::Assert,
) -> SourceInspectionCommandReport {
    let stdout = assert.get_output().stdout.clone();
    serde_json::from_slice(&stdout).expect("parse source inspection report")
}

fn parse_catalog_report(assert: assert_cmd::assert::Assert) -> CatalogCommandReport {
    let stdout = assert.get_output().stdout.clone();
    serde_json::from_slice(&stdout).expect("parse catalog report")
}

fn parse_schema_report(assert: assert_cmd::assert::Assert) -> SchemaCommandReport {
    let stdout = assert.get_output().stdout.clone();
    serde_json::from_slice(&stdout).expect("parse schema report")
}

#[test]
fn help_prints_the_new_workflows_and_contract_language() {
    let mut command = Command::cargo_bin("htmlcut").expect("binary");
    command
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "HTMLCut has five operator-facing entry points",
        ))
        .stdout(predicate::str::contains("catalog"))
        .stdout(predicate::str::contains("schema"))
        .stdout(predicate::str::contains("select"))
        .stdout(predicate::str::contains("slice"))
        .stdout(predicate::str::contains("inspect"))
        .stdout(predicate::str::contains("--value"))
        .stdout(predicate::str::contains("--output"))
        .stdout(predicate::str::contains("--verbose"))
        .stdout(predicate::str::contains(
            "request/result contract refs, usage, typed defaults, command constraints, modes, parameters, notes, and examples",
        ))
        .stdout(predicate::str::contains("`<a` also matches `<article>`"));
}

#[test]
fn select_help_stays_select_specific() {
    let mut command = Command::cargo_bin("htmlcut").expect("binary");
    command
        .args(["select", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Use --match single|first|nth|all to decide how many matches survive.",
        ))
        .stdout(predicate::str::contains(
            "Attribute name to extract when `--value attribute` is used",
        ))
        .stdout(predicate::str::contains("In slice mode").not());
}

#[test]
fn slice_help_clarifies_boundary_consumption_and_attribute_recovery() {
    let mut command = Command::cargo_bin("htmlcut").expect("binary");
    command
        .args(["slice", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Boundary matches are consumed exactly as matched."))
        .stdout(predicate::str::contains(
            "By default, the selected fragment excludes both matched boundaries.",
        ))
        .stdout(predicate::str::contains(
            "use --include-start when the opening tag lives in the start boundary.",
        ))
        .stdout(predicate::str::contains(
            "For --value outer-html, HTMLCut returns the full outer matched range including both boundaries.",
        ))
        .stdout(predicate::str::contains(
            "htmlcut slice ./page.html --from 'START::' --to '::END' --pattern regex --match all --output json",
        ))
        .stdout(predicate::str::contains(
            "htmlcut slice ./page.html --from '<a ' --to '</a>' --include-start --include-end --value attribute --attribute href",
        ));
}

#[test]
fn version_prints_workspace_version_and_description() {
    let mut command = Command::cargo_bin("htmlcut").expect("binary");
    command
        .arg("--version")
        .assert()
        .success()
        .stdout(expected_version_banner());
}

#[test]
fn subcommand_version_reuses_the_root_identity_banner() {
    let mut command = Command::cargo_bin("htmlcut").expect("binary");
    command
        .args(["select", "--version"])
        .assert()
        .success()
        .stdout(expected_version_banner());
}

#[test]
fn request_file_runs_reusable_select_definitions_and_rejects_inline_conflicts() {
    let tempdir = tempdir().expect("tempdir");
    let input_path = write_fixture(
        tempdir.path(),
        "request-file.html",
        "<article>Hello from definition</article>",
    );
    let definition_path = tempdir.path().join("select-request.json");

    let mut request = ExtractionRequest::new(
        source_request(&input_path, None),
        selector_extraction("article")
            .with_selection(SelectionSpec::single())
            .with_value(ValueSpec::Text),
    );
    request.normalization = NormalizationOptions {
        whitespace: WhitespaceMode::Preserve,
        rewrite_urls: false,
    };
    request.output = extraction_output();

    let definition = ExtractionDefinition::new(request);
    fs::write(
        &definition_path,
        serde_json::to_string_pretty(&definition).expect("serialize definition"),
    )
    .expect("write definition");

    let mut command = Command::cargo_bin("htmlcut").expect("binary");
    command
        .args(["select", "--request-file"])
        .arg(&definition_path)
        .assert()
        .success()
        .stdout("Hello from definition\n")
        .stderr("");

    let mut conflicting = Command::cargo_bin("htmlcut").expect("binary");
    conflicting
        .args(["select", "--request-file"])
        .arg(&definition_path)
        .args(["--css", "article"])
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains(
            "--request-file owns the extraction definition",
        ));
}

#[test]
fn output_file_writes_the_stdout_payload_without_emitting_stdout() {
    let tempdir = tempdir().expect("tempdir");
    let input_path = write_fixture(
        tempdir.path(),
        "output-file.html",
        "<article><p>Hello file output</p></article>",
    );
    let output_path = tempdir.path().join("selection.txt");

    let mut command = Command::cargo_bin("htmlcut").expect("binary");
    command
        .args(["select"])
        .arg(&input_path)
        .args(["--css", "article", "--output-file"])
        .arg(&output_path)
        .assert()
        .success()
        .stdout("")
        .stderr("");

    assert_eq!(
        fs::read_to_string(&output_path).expect("read output file"),
        "Hello file output\n"
    );
}

#[test]
fn quiet_suppresses_non_fatal_success_stderr() {
    let tempdir = tempdir().expect("tempdir");
    let input_path = write_fixture(
        tempdir.path(),
        "quiet.html",
        "<article>First</article><article>Second</article>",
    );

    let mut noisy = Command::cargo_bin("htmlcut").expect("binary");
    noisy
        .args(["select"])
        .arg(&input_path)
        .args(["--css", "article"])
        .assert()
        .success()
        .stdout("First\n")
        .stderr(predicate::str::contains("warning MULTIPLE_MATCHES"));

    let mut quiet = Command::cargo_bin("htmlcut").expect("binary");
    quiet
        .args(["select", "--quiet"])
        .arg(&input_path)
        .args(["--css", "article"])
        .assert()
        .success()
        .stdout("First\n")
        .stderr("");
}

#[test]
fn catalog_json_surfaces_operation_catalog() {
    let mut command = Command::cargo_bin("htmlcut").expect("binary");
    let report = parse_catalog_report(
        command
            .args(["catalog", "--output", "json"])
            .assert()
            .success(),
    );

    assert_eq!(report.tool, "htmlcut");
    assert_eq!(report.version, expected_version());
    assert_eq!(report.schema_name, CATALOG_REPORT_SCHEMA_NAME);
    assert_eq!(report.schema_version, CATALOG_SCHEMA_VERSION);
    assert_eq!(
        report.schema_profile,
        htmlcut_core::HTMLCUT_JSON_SCHEMA_PROFILE
    );
    assert_eq!(
        report.description,
        "Extract and inspect HTML from files, URLs, and stdin with CSS selectors, literal or regex slicing, and structured reports."
    );
    assert_eq!(report.command, "catalog");
    assert_eq!(
        report.operations.len(),
        htmlcut_core::operation_catalog().len()
    );
    assert_eq!(
        report.operations[0].operation_id,
        htmlcut_core::operation_catalog()[0].id
    );
    assert_eq!(
        report.operations[0].core_surface,
        htmlcut_core::operation_catalog()[0].core_surface
    );
    assert_eq!(
        report.operations[0].request_contract.rust_shape,
        htmlcut_core::operation_catalog()[0]
            .request_contract
            .rust_shape
    );
    assert_eq!(
        report.operations[0].result_contract.rust_shape,
        htmlcut_core::operation_catalog()[0]
            .result_contract
            .rust_shape
    );
    assert!(report.operations[0].command_contract.is_none());

    let select_extract = report
        .operations
        .iter()
        .find(|operation| operation.operation_id == htmlcut_core::OperationId::SelectExtract)
        .expect("select.extract should be cataloged");
    let command_contract = select_extract
        .command_contract
        .as_ref()
        .expect("cli operation should expose a command contract");
    assert_eq!(
        command_contract.invocation,
        "htmlcut select [OPTIONS] --css <CSS> [INPUT]"
    );
    assert_eq!(command_contract.default_match.as_deref(), Some("first"));
    assert_eq!(command_contract.default_value.as_deref(), Some("text"));
    assert_eq!(command_contract.default_output.as_deref(), Some("text"));
    assert_eq!(command_contract.default_output_overrides.len(), 1);
    assert_eq!(command_contract.default_output_overrides[0].value, "json");
    assert_eq!(
        command_contract.default_output_overrides[0].when.parameter,
        "--value"
    );
    assert_eq!(
        command_contract.default_output_overrides[0].when.values,
        vec!["structured".to_owned()]
    );
    assert!(
        command_contract
            .constraints
            .iter()
            .any(|constraint| matches!(
                constraint,
                htmlcut_cli::CatalogConstraint::RequiresParameter { parameter, when }
                    if parameter == "--bundle"
                        && when.parameter == "--output"
                        && when.values == vec!["none".to_owned()]
            ))
    );
    assert!(command_contract.parameters.iter().any(|parameter| {
        parameter.name == "--css"
            && parameter.requirement == htmlcut_cli::CatalogParameterRequirement::Conditional
            && parameter.requirement_note.as_deref()
                == Some("required unless --request-file is used")
    }));
    assert!(command_contract.parameters.iter().any(|parameter| {
        parameter.name == "--request-file"
            && parameter.requirement == htmlcut_cli::CatalogParameterRequirement::Optional
    }));
    assert!(command_contract.parameters.iter().any(|parameter| {
        parameter.name == "--fetch-preflight"
            && parameter.requirement == htmlcut_cli::CatalogParameterRequirement::Optional
    }));
    assert!(command_contract.parameters.iter().any(|parameter| {
        parameter.name == "--output-file"
            && parameter.requirement == htmlcut_cli::CatalogParameterRequirement::Optional
    }));
    assert!(command_contract.parameters.iter().any(|parameter| {
        parameter.name == "--attribute"
            && parameter.requirement == htmlcut_cli::CatalogParameterRequirement::Conditional
            && parameter.requirement_note.as_deref()
                == Some("required when --value attribute is used")
    }));
    assert!(command_contract.notes.iter().any(|note| {
        note.contains("Structured extraction only supports --output json or --output none.")
    }));
}

#[test]
fn schema_json_surfaces_registry_for_core_cli_and_interop() {
    let mut command = Command::cargo_bin("htmlcut").expect("binary");
    let report = parse_schema_report(
        command
            .args(["schema", "--output", "json"])
            .assert()
            .success(),
    );

    assert_eq!(report.tool, "htmlcut");
    assert_eq!(report.version, expected_version());
    assert_eq!(report.schema_name, SCHEMA_COMMAND_REPORT_SCHEMA_NAME);
    assert_eq!(report.schema_version, SCHEMA_COMMAND_REPORT_SCHEMA_VERSION);
    assert_eq!(
        report.schema_profile,
        htmlcut_core::HTMLCUT_JSON_SCHEMA_PROFILE
    );
    assert!(report.schemas.iter().any(|schema| {
        schema.schema_name == htmlcut_core::EXTRACTION_REQUEST_SCHEMA_NAME
            && schema.schema_version == htmlcut_core::CORE_REQUEST_SCHEMA_VERSION
            && schema.owner_surface == "htmlcut-core"
    }));
    assert!(report.schemas.iter().any(|schema| {
        schema.schema_name == htmlcut_core::interop::v1::RESULT_SCHEMA_NAME
            && schema.owner_surface == "htmlcut_core::interop::v1"
    }));
    assert!(report.schemas.iter().any(|schema| {
        schema.schema_name == CATALOG_REPORT_SCHEMA_NAME && schema.owner_surface == "htmlcut-cli"
    }));
}

#[test]
fn inspect_source_directory_input_reports_directory_specific_failure() {
    let tempdir = tempdir().expect("tempdir");
    let input_dir = tempdir.path().join("input dir");
    fs::create_dir_all(&input_dir).expect("create dir");

    let mut command = Command::cargo_bin("htmlcut").expect("binary");
    command
        .args(["inspect", "source"])
        .arg(&input_dir)
        .args(["--output", "text"])
        .assert()
        .failure()
        .code(3)
        .stderr(predicate::str::contains(
            "Input path is a directory, not a file:",
        ));
}

#[test]
fn inspect_source_invalid_utf8_input_reports_utf8_failure() {
    let tempdir = tempdir().expect("tempdir");
    let input_path = tempdir.path().join("bad.bin");
    fs::write(&input_path, [0xff, 0xfe]).expect("write invalid utf8");

    let mut command = Command::cargo_bin("htmlcut").expect("binary");
    command
        .args(["inspect", "source"])
        .arg(&input_path)
        .args(["--output", "text"])
        .assert()
        .failure()
        .code(3)
        .stderr(predicate::str::contains("File is not valid UTF-8:"));
}

#[test]
fn select_text_output_extracts_text_for_humans() {
    let tempdir = tempdir().expect("tempdir");
    let input_path = write_fixture(
        tempdir.path(),
        "input.html",
        "<article><p>Hello <strong>world</strong></p></article>",
    );

    let mut command = Command::cargo_bin("htmlcut").expect("binary");
    command
        .args(["select"])
        .arg(&input_path)
        .args(["--css", "article"])
        .assert()
        .success()
        .stdout("Hello world\n");
}

#[test]
fn select_nth_human_output_does_not_warn_about_multiple_candidates() {
    let tempdir = tempdir().expect("tempdir");
    let input_path = write_fixture(
        tempdir.path(),
        "select-nth.html",
        "<article class=\"card\">One</article><article class=\"card\">Two</article>",
    );

    let mut command = Command::cargo_bin("htmlcut").expect("binary");
    command
        .args(["select"])
        .arg(&input_path)
        .args(["--css", "article.card", "--match", "nth", "--index", "2"])
        .assert()
        .success()
        .stdout("Two\n")
        .stderr("");
}

#[test]
fn select_single_fails_when_multiple_candidates_exist() {
    let tempdir = tempdir().expect("tempdir");
    let input_path = write_fixture(
        tempdir.path(),
        "select-single.html",
        "<article class=\"card\">One</article><article class=\"card\">Two</article>",
    );

    let mut command = Command::cargo_bin("htmlcut").expect("binary");
    command
        .args(["select"])
        .arg(&input_path)
        .args(["--css", "article.card", "--match", "single"])
        .assert()
        .failure()
        .code(4)
        .stderr(predicate::str::contains(
            "Exact-one selection requires exactly one candidate",
        ));
}

#[test]
fn select_json_report_has_core_parity_for_structured_extraction() {
    let tempdir = tempdir().expect("tempdir");
    let input_path = write_fixture(
        tempdir.path(),
        "page.html",
        "<html><head><title>Parity</title></head><body><article class=\"card\"><p>Hello</p></article></body></html>",
    );

    let mut request = ExtractionRequest::new(
        source_request(&input_path, None),
        selector_extraction("article.card")
            .with_selection(SelectionSpec::First)
            .with_value(ValueSpec::Structured),
    );
    request.normalization = NormalizationOptions {
        whitespace: WhitespaceMode::Preserve,
        rewrite_urls: false,
    };
    request.output = extraction_output();
    let expected = extract(&request, &runtime_options());
    assert!(expected.ok);

    let mut command = Command::cargo_bin("htmlcut").expect("binary");
    let report = parse_extraction_report(
        command
            .args(["select"])
            .arg(&input_path)
            .args(["--css", "article.card", "--value", "structured"])
            .assert()
            .success(),
    );

    assert_eq!(report.tool, "htmlcut");
    assert_eq!(report.engine, "htmlcut-core");
    assert_eq!(report.version, expected_version());
    assert_eq!(report.schema_name, EXTRACTION_COMMAND_REPORT_SCHEMA_NAME);
    assert_eq!(report.command, "select");
    assert_eq!(
        report.operation_id,
        htmlcut_core::OperationId::SelectExtract
    );
    assert_eq!(report.ok, expected.ok);
    assert_eq!(
        report.schema_version,
        EXTRACTION_COMMAND_REPORT_SCHEMA_VERSION
    );
    assert_eq!(report.source, expected.source);
    assert_eq!(report.extraction, expected.extraction);
    assert_eq!(report.stats.candidate_count, expected.stats.candidate_count);
    assert_eq!(report.stats.match_count, expected.stats.match_count);
    assert_eq!(report.matches, expected.matches);
    assert_eq!(report.diagnostics, expected.diagnostics);
    assert_eq!(report.document_title.as_deref(), Some("Parity"));
    assert!(report.bundle.is_none());
}

#[test]
fn structured_selector_metadata_only_rewrites_url_bearing_attributes() {
    let tempdir = tempdir().expect("tempdir");
    let input_path = write_fixture(
        tempdir.path(),
        "input.html",
        "<article><a class=\"card featured\" href=\"guide.html\" data-track=\"hero\">Guide</a></article>",
    );

    let mut command = Command::cargo_bin("htmlcut").expect("binary");
    let report = parse_extraction_report(
        command
            .args(["select"])
            .arg(&input_path)
            .args([
                "--css",
                "a",
                "--value",
                "structured",
                "--rewrite-urls",
                "--base-url",
                "https://example.com/docs/start.html",
                "--output",
                "json",
            ])
            .assert()
            .success(),
    );

    let attributes = report.matches[0].metadata.clone();
    let ExtractionMatchMetadata::Selector(attributes) = attributes else {
        panic!("expected selector metadata");
    };
    assert_eq!(
        attributes.attributes.get("href").map(String::as_str),
        Some("https://example.com/docs/guide.html")
    );
    assert_eq!(
        attributes.attributes.get("class").map(String::as_str),
        Some("card featured")
    );
    assert_eq!(
        attributes.attributes.get("data-track").map(String::as_str),
        Some("hero")
    );
}

#[test]
fn select_attribute_rewrite_has_core_parity() {
    let tempdir = tempdir().expect("tempdir");
    let input_path = write_fixture(
        tempdir.path(),
        "input.html",
        "<article><a href=\"../guide.html\">Guide</a></article>",
    );
    let base_url = "https://example.com/docs/start.html";

    let mut request = ExtractionRequest::new(
        source_request(&input_path, Some(base_url)),
        selector_extraction("article a")
            .with_selection(SelectionSpec::First)
            .with_value(ValueSpec::Attribute {
                name: AttributeName::new("href").expect("attribute name"),
            }),
    );
    request.normalization = NormalizationOptions {
        whitespace: WhitespaceMode::Preserve,
        rewrite_urls: true,
    };
    request.output = extraction_output();
    let expected = extract(&request, &runtime_options());
    assert!(expected.ok);

    let mut command = Command::cargo_bin("htmlcut").expect("binary");
    let report = parse_extraction_report(
        command
            .args(["select"])
            .arg(&input_path)
            .args([
                "--css",
                "article a",
                "--value",
                "attribute",
                "--attribute",
                "href",
                "--rewrite-urls",
                "--base-url",
                base_url,
                "--output",
                "json",
            ])
            .assert()
            .success(),
    );

    assert_eq!(report.source, expected.source);
    assert_eq!(report.extraction, expected.extraction);
    assert_eq!(report.matches, expected.matches);
    assert_eq!(
        report.matches[0].value.as_str(),
        Some("https://example.com/guide.html")
    );
}

#[test]
fn select_attribute_rewrite_honors_document_base_for_file_inputs() {
    let tempdir = tempdir().expect("tempdir");
    let input_path = write_fixture(
        tempdir.path(),
        "document-base.html",
        "<html><head><base href=\"https://fixture.example/base/\"></head><body><article><a href=\"guide/start.html\">Guide</a></article></body></html>",
    );

    let mut command = Command::cargo_bin("htmlcut").expect("binary");
    command
        .args(["select"])
        .arg(&input_path)
        .args([
            "--css",
            "article a",
            "--value",
            "attribute",
            "--attribute",
            "href",
            "--rewrite-urls",
        ])
        .assert()
        .success()
        .stdout("https://fixture.example/base/guide/start.html\n");
}

#[test]
fn slice_json_report_has_core_parity() {
    let tempdir = tempdir().expect("tempdir");
    let input_path = write_fixture(tempdir.path(), "input.html", "<p>One</p><p>Two</p>");

    let mut request = ExtractionRequest::new(
        source_request(&input_path, None),
        slice_extraction("<p>", "</p>", PatternMode::Literal, false, false)
            .with_selection(SelectionSpec::All)
            .with_value(ValueSpec::Text),
    );
    request.normalization = NormalizationOptions {
        whitespace: WhitespaceMode::Preserve,
        rewrite_urls: false,
    };
    request.output = extraction_output();
    let expected = extract(&request, &runtime_options());
    assert!(expected.ok);

    let mut command = Command::cargo_bin("htmlcut").expect("binary");
    let report = parse_extraction_report(
        command
            .args(["slice"])
            .arg(&input_path)
            .args([
                "--from", "<p>", "--to", "</p>", "--match", "all", "--output", "json",
            ])
            .assert()
            .success(),
    );

    assert_eq!(report.command, "slice");
    assert_eq!(report.source, expected.source);
    assert_eq!(report.extraction, expected.extraction);
    assert_eq!(report.stats.candidate_count, expected.stats.candidate_count);
    assert_eq!(report.stats.match_count, expected.stats.match_count);
    assert_eq!(report.matches, expected.matches);
    assert_eq!(report.diagnostics, expected.diagnostics);
}

#[test]
fn inspect_source_json_has_core_parity() {
    let tempdir = tempdir().expect("tempdir");
    let input_path = write_fixture(
        tempdir.path(),
        "inspect.html",
        "<html><head><title>Inspect Me</title></head><body><main><h1>Heading</h1><a href=\"/guide\">Guide</a></main></body></html>",
    );
    let request = source_request(&input_path, Some("https://example.com/start"));
    let expected = inspect_source(
        &request,
        &runtime_options(),
        &htmlcut_core::InspectionOptions {
            include_source_text: false,
            sample_limit: 8,
        },
    );
    assert!(expected.ok);

    let mut command = Command::cargo_bin("htmlcut").expect("binary");
    let report = parse_source_inspection_report(
        command
            .args(["inspect", "source"])
            .arg(&input_path)
            .args(["--base-url", "https://example.com/start"])
            .assert()
            .success(),
    );

    assert_eq!(report.tool, "htmlcut");
    assert_eq!(report.engine, "htmlcut-core");
    assert_eq!(report.version, expected_version());
    assert_eq!(
        report.schema_name,
        SOURCE_INSPECTION_COMMAND_REPORT_SCHEMA_NAME
    );
    assert_eq!(report.command, "inspect-source");
    assert_eq!(
        report.operation_id,
        htmlcut_core::OperationId::SourceInspect
    );
    assert_eq!(report.ok, expected.ok);
    assert_eq!(
        report.schema_version,
        SOURCE_INSPECTION_COMMAND_REPORT_SCHEMA_VERSION
    );
    assert_eq!(report.source, expected.source);
    assert_eq!(report.document, expected.document);
    assert_eq!(report.diagnostics, expected.diagnostics);
}

#[test]
fn inspect_source_text_surfaces_base_behavior_and_source_preview() {
    let tempdir = tempdir().expect("tempdir");
    let input_path = write_fixture(
        tempdir.path(),
        "inspect-text.html",
        "<html><head><base href=\"../content/\"><title>Inspect Me</title></head><body><main><h1>Heading</h1><a href=\"guide.html\">Guide</a></main></body></html>",
    );

    let mut command = Command::cargo_bin("htmlcut").expect("binary");
    command
        .args(["inspect", "source"])
        .arg(&input_path)
        .args([
            "--base-url",
            "https://example.com/docs/start.html",
            "--output",
            "text",
            "--include-source-text",
            "--preview-chars",
            "32",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Input base URL: https://example.com/docs/start.html",
        ))
        .stdout(predicate::str::contains(
            "Effective base URL: https://example.com/content/",
        ))
        .stdout(predicate::str::contains(
            "Document <base href>: ../content/",
        ))
        .stdout(predicate::str::contains("Source text preview:"));
}

#[test]
fn inspect_source_text_reports_unresolved_effective_base() {
    let tempdir = tempdir().expect("tempdir");
    let input_path = write_fixture(
        tempdir.path(),
        "inspect-unresolved.html",
        "<html><head><base href=\"../content/\"><title>Inspect Me</title></head><body><main><a href=\"guide.html\">Guide</a></main></body></html>",
    );

    let mut command = Command::cargo_bin("htmlcut").expect("binary");
    command
        .args(["inspect", "source"])
        .arg(&input_path)
        .args(["--output", "text"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Document <base href>: ../content/",
        ))
        .stdout(predicate::str::contains("Effective base URL: unresolved"))
        .stdout(predicate::str::contains(
            "warning EFFECTIVE_BASE_URL_UNRESOLVED",
        ));
}

#[test]
fn inspect_select_json_has_core_preview_parity() {
    let tempdir = tempdir().expect("tempdir");
    let input_path = write_fixture(
        tempdir.path(),
        "preview.html",
        "<section class=\"card\"><h2>One</h2></section><section class=\"card\"><h2>Two</h2></section>",
    );

    let mut request = ExtractionRequest::new(
        source_request(&input_path, None),
        selector_extraction("section.card")
            .with_selection(SelectionSpec::All)
            .with_value(ValueSpec::Structured),
    );
    request.normalization = NormalizationOptions {
        whitespace: WhitespaceMode::Preserve,
        rewrite_urls: false,
    };
    request.output = extraction_output();
    let expected = preview_extraction(&request, &runtime_options());
    assert!(expected.ok);

    let mut command = Command::cargo_bin("htmlcut").expect("binary");
    let report = parse_extraction_report(
        command
            .args(["inspect", "select"])
            .arg(&input_path)
            .args(["--css", "section.card", "--match", "all"])
            .assert()
            .success(),
    );

    assert_eq!(report.command, "inspect-select");
    assert_eq!(
        report.operation_id,
        htmlcut_core::OperationId::SelectPreview
    );
    assert_eq!(report.source, expected.source);
    assert_eq!(report.extraction, expected.extraction);
    assert_eq!(report.stats.candidate_count, expected.stats.candidate_count);
    assert_eq!(report.stats.match_count, expected.stats.match_count);
    assert_eq!(report.matches, expected.matches);
    assert_eq!(report.diagnostics, expected.diagnostics);
}

#[test]
fn inspect_select_nth_does_not_warn_about_multiple_candidates() {
    let tempdir = tempdir().expect("tempdir");
    let input_path = write_fixture(
        tempdir.path(),
        "inspect-select-nth.html",
        "<section class=\"card\">One</section><section class=\"card\">Two</section>",
    );

    let mut command = Command::cargo_bin("htmlcut").expect("binary");
    let report = parse_extraction_report(
        command
            .args(["inspect", "select"])
            .arg(&input_path)
            .args([
                "--css",
                "section.card",
                "--match",
                "nth",
                "--index",
                "2",
                "--output",
                "json",
            ])
            .assert()
            .success(),
    );

    assert!(report.ok);
    assert_eq!(report.stats.candidate_count, 2);
    assert_eq!(report.stats.match_count, 1);
    assert!(report.diagnostics.is_empty());
}

#[test]
fn inspect_slice_json_has_core_preview_parity() {
    let tempdir = tempdir().expect("tempdir");
    let input_path = write_fixture(
        tempdir.path(),
        "slice-preview.html",
        "<div>START::Alpha::END</div><div>START::Beta::END</div>",
    );

    let mut request = ExtractionRequest::new(
        source_request(&input_path, None),
        slice_extraction("START::[A-Za-z]+", "::END", PatternMode::Regex, true, true)
            .with_selection(SelectionSpec::All)
            .with_value(ValueSpec::Structured),
    );
    request.normalization = NormalizationOptions {
        whitespace: WhitespaceMode::Preserve,
        rewrite_urls: false,
    };
    request.output = extraction_output();
    let expected = preview_extraction(&request, &runtime_options());
    assert!(expected.ok);

    let mut command = Command::cargo_bin("htmlcut").expect("binary");
    let report = parse_extraction_report(
        command
            .args(["inspect", "slice"])
            .arg(&input_path)
            .args([
                "--from",
                "START::[A-Za-z]+",
                "--to",
                "::END",
                "--pattern",
                "regex",
                "--include-start",
                "--include-end",
                "--match",
                "all",
            ])
            .assert()
            .success(),
    );

    assert_eq!(report.command, "inspect-slice");
    assert_eq!(report.operation_id, htmlcut_core::OperationId::SlicePreview);
    assert_eq!(report.source, expected.source);
    assert_eq!(report.extraction, expected.extraction);
    assert_eq!(report.stats.candidate_count, expected.stats.candidate_count);
    assert_eq!(report.stats.match_count, expected.stats.match_count);
    assert_eq!(report.matches, expected.matches);
    assert_eq!(report.diagnostics, expected.diagnostics);
}

#[test]
fn inspect_slice_text_surfaces_ranges_and_boundary_context() {
    let tempdir = tempdir().expect("tempdir");
    let input_path = write_fixture(
        tempdir.path(),
        "slice-preview-text.html",
        "<div>START::Alpha::END</div><div>START::Beta::END</div>",
    );

    let mut command = Command::cargo_bin("htmlcut").expect("binary");
    command
        .args(["inspect", "slice"])
        .arg(&input_path)
        .args([
            "--from",
            "START::[A-Za-z]+",
            "--to",
            "::END",
            "--pattern",
            "regex",
            "--include-start",
            "--include-end",
            "--match",
            "all",
            "--output",
            "text",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Selected: 2"))
        .stdout(predicate::str::contains("candidate index:"))
        .stdout(predicate::str::contains("include start: true"))
        .stdout(predicate::str::contains("include end: true"))
        .stdout(predicate::str::contains("selected range:"))
        .stdout(predicate::str::contains("inner range:"))
        .stdout(predicate::str::contains("outer range:"))
        .stdout(predicate::str::contains("text: START::Alpha::END"));
}

#[test]
fn inspect_slice_text_shows_fragment_preview_when_boundary_consumption_hides_text() {
    let tempdir = tempdir().expect("tempdir");
    let input_path = write_fixture(
        tempdir.path(),
        "slice-preview-empty-text.html",
        "<div>START::Alpha::END</div><div>START::Beta::END</div>",
    );

    let mut command = Command::cargo_bin("htmlcut").expect("binary");
    command
        .args(["inspect", "slice"])
        .arg(&input_path)
        .args([
            "--from",
            "START::[A-Za-z]+",
            "--to",
            "::END",
            "--pattern",
            "regex",
            "--match",
            "all",
            "--output",
            "text",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("fragment: START::Alpha::END"))
        .stdout(predicate::str::contains("fragment: START::Beta::END"));
}

#[test]
fn inspect_slice_text_shows_fragment_preview_for_html_like_matches() {
    let tempdir = tempdir().expect("tempdir");
    let input_path = write_fixture(
        tempdir.path(),
        "inspect-slice-html.html",
        "<article><a href=\"guide.html\">Guide</a></article><section><a href=\"more.html\">More</a></section>",
    );

    let mut command = Command::cargo_bin("htmlcut").expect("binary");
    command
        .args(["inspect", "slice"])
        .arg(&input_path)
        .args([
            "--from",
            "<a",
            "--to",
            "</a>",
            "--include-start",
            "--include-end",
            "--match",
            "all",
            "--output",
            "text",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "fragment: <article><a href=\"guide.html\">Guide</a>",
        ))
        .stdout(predicate::str::contains(
            "fragment: <a href=\"more.html\">More</a>",
        ));
}

#[test]
fn stdin_bundle_flow_and_verbose_levels_work() {
    let tempdir = tempdir().expect("tempdir");
    let bundle_dir = tempdir.path().join("bundle space");

    let mut command = Command::cargo_bin("htmlcut").expect("binary");
    command
        .args(["select", "-"])
        .args([
            "--css",
            "article",
            "--output",
            "json",
            "--bundle",
            bundle_dir.to_str().expect("bundle dir"),
            "-vv",
        ])
        .write_stdin("<article><p>Hello</p></article>")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"command\": \"select\""))
        .stderr(predicate::str::contains("selected 1 match"))
        .stderr(predicate::str::contains("scanned 1 candidates"))
        .stderr(predicate::str::contains("wrote bundle"));

    assert!(bundle_dir.join("selection.html").exists());
    assert!(bundle_dir.join("selection.txt").exists());
    assert!(bundle_dir.join("report.json").exists());
}

#[test]
fn global_verbose_before_subcommand_also_works() {
    let mut command = Command::cargo_bin("htmlcut").expect("binary");
    command
        .args(["-vv", "select", "-"])
        .args(["--css", "article"])
        .write_stdin("<article><p>Hello</p></article>")
        .assert()
        .success()
        .stdout("Hello\n")
        .stderr(predicate::str::contains("selected 1 match"))
        .stderr(predicate::str::contains("scanned 1 candidates"));
}

#[test]
fn human_select_warns_when_rewrite_is_requested_without_an_effective_base() {
    let tempdir = tempdir().expect("tempdir");
    let input_path = write_fixture(
        tempdir.path(),
        "unresolved-base.html",
        "<html><head><base href=\"../content/\"></head><body><a href=\"guide.html\">Guide</a></body></html>",
    );

    let mut command = Command::cargo_bin("htmlcut").expect("binary");
    command
        .args(["select"])
        .arg(&input_path)
        .args([
            "--css",
            "a",
            "--value",
            "attribute",
            "--attribute",
            "href",
            "--rewrite-urls",
        ])
        .assert()
        .success()
        .stdout("guide.html\n")
        .stderr(predicate::str::contains(
            "warning EFFECTIVE_BASE_URL_UNRESOLVED",
        ));
}

#[test]
fn invalid_selector_exits_with_usage_code() {
    let tempdir = tempdir().expect("tempdir");
    let input_path = write_fixture(tempdir.path(), "input.html", "<div>Hello</div>");

    let mut command = Command::cargo_bin("htmlcut").expect("binary");
    command
        .args(["select"])
        .arg(input_path)
        .args(["--css", "["])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("Invalid selector"));
}

#[test]
fn slice_attribute_error_hints_when_excluded_start_boundary_drops_the_opening_tag() {
    let tempdir = tempdir().expect("tempdir");
    let input_path = write_fixture(
        tempdir.path(),
        "slice-attribute.html",
        "<article><a href=\"guide.html\">Guide</a></article>",
    );

    let mut command = Command::cargo_bin("htmlcut").expect("binary");
    command
        .args(["slice"])
        .arg(&input_path)
        .args([
            "--from",
            "<a ",
            "--to",
            "</a>",
            "--value",
            "attribute",
            "--attribute",
            "href",
        ])
        .assert()
        .failure()
        .code(4)
        .stderr(predicate::str::contains("use --include-start"));
}

#[test]
fn output_none_requires_bundle() {
    let tempdir = tempdir().expect("tempdir");
    let input_path = write_fixture(tempdir.path(), "input.html", "<div>Hello</div>");

    let mut command = Command::cargo_bin("htmlcut").expect("binary");
    command
        .args(["select"])
        .arg(input_path)
        .args(["--css", "div", "--output", "none"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("--output none requires --bundle"));
}
