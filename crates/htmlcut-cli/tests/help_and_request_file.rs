//! Black-box regression tests for derived help text and request-definition recovery guidance.

use std::fs;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use htmlcut_core::{
    ExtractionDefinition, ExtractionRequest, ExtractionSpec, SelectorQuery, SliceBoundary,
    SliceSpec, SourceRequest, wire::v1::ExtractionDefinitionDocument,
};
use htmlcut_tempdir::tempdir;
use predicates::prelude::*;
use serde_json::json;

fn write_json_file(tempdir: &Path, name: &str, contents: &str) -> PathBuf {
    let path = tempdir.join(name);
    fs::write(&path, contents).expect("write json fixture");
    path
}

fn write_definition_file(tempdir: &Path, name: &str, definition: &ExtractionDefinition) -> PathBuf {
    let path = tempdir.join(name);
    let document =
        ExtractionDefinitionDocument::try_from(definition.clone()).expect("definition document");
    fs::write(
        &path,
        serde_json::to_string_pretty(&document).expect("serialize definition"),
    )
    .expect("write definition fixture");
    path
}

fn normalize_whitespace(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn help_output(args: &[&str]) -> String {
    let assert = Command::cargo_bin("htmlcut")
        .expect("binary")
        .args(args)
        .assert()
        .success();
    String::from_utf8(assert.get_output().stdout.clone()).expect("stdout utf8")
}

fn assert_help_stays_grammar_first(args: &[&str]) {
    let help = normalize_whitespace(&help_output(args));
    assert!(help.contains("Usage:"), "{help}");
    assert!(help.contains("Examples:"), "{help}");
    for fragment in [
        "Default match mode:",
        "Supported match modes:",
        "Default value mode:",
        "Supported value modes:",
        "Default output mode:",
        "Output default override:",
        "Supported output modes:",
    ] {
        assert!(!help.contains(fragment), "{fragment}");
    }
}

#[test]
fn subcommand_help_renders_canonical_contract_modes_and_notes() {
    for args in [
        &["select", "--help"][..],
        &["slice", "--help"][..],
        &["inspect", "source", "--help"][..],
        &["inspect", "select", "--help"][..],
        &["inspect", "slice", "--help"][..],
    ] {
        assert_help_stays_grammar_first(args);
    }
}

#[test]
fn source_help_exposes_inline_html_as_a_first_class_contract() {
    let select_help = normalize_whitespace(&help_output(&["select", "--help"]));
    let inspect_help = normalize_whitespace(&help_output(&["inspect", "source", "--help"]));

    assert!(select_help.contains("--input-html <HTML>"), "{select_help}");
    assert!(
        inspect_help.contains("--input-html <HTML>"),
        "{inspect_help}"
    );
}

#[test]
fn missing_request_file_points_back_to_schema_and_catalog_contracts() {
    let tempdir = tempdir().expect("tempdir");
    let missing_path = tempdir.path().join("missing request [draft].json");

    Command::cargo_bin("htmlcut")
        .expect("binary")
        .args(["select", "--request-file"])
        .arg(&missing_path)
        .assert()
        .failure()
        .code(2)
        .stdout("")
        .stderr(predicate::str::contains(
            "Could not read extraction definition",
        ))
        .stderr(predicate::str::contains(
            "htmlcut schema --name htmlcut.extraction_definition --output json",
        ))
        .stderr(predicate::str::contains(
            "htmlcut catalog --operation select.extract --output json",
        ));
}

#[test]
fn unsupported_request_file_schema_carries_recovery_guidance() {
    let tempdir = tempdir().expect("tempdir");
    let definition = ExtractionDefinition::new(ExtractionRequest::new(
        SourceRequest::stdin(),
        ExtractionSpec::selector(SelectorQuery::new("article").expect("selector")),
    ));
    let mut document =
        serde_json::to_value(ExtractionDefinitionDocument::try_from(definition).expect("document"))
            .expect("serialize definition");
    document["schema_version"] = json!(999_u32);
    let request_path = write_json_file(
        tempdir.path(),
        "unsupported schema.json",
        &serde_json::to_string_pretty(&document).expect("serialize unsupported schema fixture"),
    );

    Command::cargo_bin("htmlcut")
        .expect("binary")
        .args(["select", "--request-file"])
        .arg(&request_path)
        .assert()
        .failure()
        .code(2)
        .stdout("")
        .stderr(predicate::str::contains(
            "Unsupported extraction definition schema",
        ))
        .stderr(predicate::str::contains(
            "htmlcut schema --name htmlcut.extraction_definition --output json",
        ))
        .stderr(predicate::str::contains(
            "htmlcut catalog --operation select.extract --output json",
        ))
        .stderr(predicate::str::contains("--emit-request-file <PATH>"));
}

#[test]
fn strategy_mismatch_request_file_points_back_to_the_matching_contract() {
    let tempdir = tempdir().expect("tempdir");
    let slice_request = ExtractionRequest::new(
        SourceRequest::stdin(),
        ExtractionSpec::slice(SliceSpec::new(
            SliceBoundary::new("<article>").expect("start boundary"),
            SliceBoundary::new("</article>").expect("end boundary"),
        )),
    );
    let request_path = write_definition_file(
        tempdir.path(),
        "slice-request.json",
        &ExtractionDefinition::new(slice_request),
    );

    Command::cargo_bin("htmlcut")
        .expect("binary")
        .args(["select", "--request-file"])
        .arg(&request_path)
        .assert()
        .failure()
        .code(2)
        .stdout("")
        .stderr(predicate::str::contains(
            "select cannot execute a slice extraction definition",
        ))
        .stderr(predicate::str::contains(
            "only accepts selector extraction definitions",
        ))
        .stderr(predicate::str::contains(
            "htmlcut schema --name htmlcut.extraction_definition --output json",
        ))
        .stderr(predicate::str::contains(
            "htmlcut catalog --operation select.extract --output json",
        ));
}

#[test]
fn request_file_schema_fixtures_stay_parseable_for_regression_tests() {
    let definition = ExtractionDefinition::new(ExtractionRequest::new(
        SourceRequest::stdin(),
        ExtractionSpec::selector(SelectorQuery::new("article").expect("selector")),
    ));
    let document =
        ExtractionDefinitionDocument::try_from(definition.clone()).expect("definition document");
    let serialized = serde_json::to_string_pretty(&document).expect("serialize definition");
    let parsed_document: ExtractionDefinitionDocument =
        serde_json::from_str(&serialized).expect("parse serialized definition");
    let parsed = ExtractionDefinition::from(parsed_document);
    assert_eq!(
        parsed.request.extraction.strategy(),
        definition.request.extraction.strategy()
    );
    assert_eq!(
        parsed.schema_name,
        htmlcut_core::EXTRACTION_DEFINITION_SCHEMA_NAME
    );
}
