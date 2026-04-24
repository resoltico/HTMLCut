//! Black-box regression tests for derived help text and request-definition recovery guidance.

use std::fs;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use htmlcut_core::cli_contract::{
    CliValue, OperationCliContract, cli_operation_contract, render_cli_value,
};
use htmlcut_core::{
    ExtractionDefinition, ExtractionRequest, ExtractionSpec, OperationId, SelectorQuery,
    SliceBoundary, SliceSpec, SourceRequest,
};
use htmlcut_tempdir::tempdir;
use predicates::prelude::*;

fn write_json_file(tempdir: &Path, name: &str, contents: &str) -> PathBuf {
    let path = tempdir.join(name);
    fs::write(&path, contents).expect("write json fixture");
    path
}

fn write_definition_file(tempdir: &Path, name: &str, definition: &ExtractionDefinition) -> PathBuf {
    let path = tempdir.join(name);
    fs::write(
        &path,
        serde_json::to_string_pretty(definition).expect("serialize definition"),
    )
    .expect("write definition fixture");
    path
}

fn normalize_whitespace(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn join_cli_values(values: impl IntoIterator<Item = CliValue>) -> String {
    values
        .into_iter()
        .map(render_cli_value)
        .collect::<Vec<_>>()
        .join(", ")
}

fn help_output(args: &[&str]) -> String {
    let assert = Command::cargo_bin("htmlcut")
        .expect("binary")
        .args(args)
        .assert()
        .success();
    String::from_utf8(assert.get_output().stdout.clone()).expect("stdout utf8")
}

fn assert_help_contains_contract(args: &[&str], operation_id: OperationId) {
    let help = normalize_whitespace(&help_output(args));
    let contract = cli_operation_contract(operation_id).expect("cli contract");

    assert_contract_modes_and_notes(&help, contract);
}

fn assert_contract_modes_and_notes(help: &str, contract: &OperationCliContract) {
    if let Some(default_match) = contract.default_match {
        let fragment = format!(
            "Default match mode: {}.",
            render_cli_value(CliValue::SelectionMode(default_match))
        );
        assert!(
            help.contains(&normalize_whitespace(&fragment)),
            "{fragment}"
        );
    }
    if !contract.selection_modes.is_empty() {
        let fragment = format!(
            "Supported match modes: {}.",
            join_cli_values(
                contract
                    .selection_modes
                    .iter()
                    .copied()
                    .map(CliValue::SelectionMode)
            )
        );
        assert!(
            help.contains(&normalize_whitespace(&fragment)),
            "{fragment}"
        );
    }
    if let Some(default_value) = contract.default_value {
        let fragment = format!(
            "Default value mode: {}.",
            render_cli_value(CliValue::ValueType(default_value))
        );
        assert!(
            help.contains(&normalize_whitespace(&fragment)),
            "{fragment}"
        );
    }
    if !contract.value_modes.is_empty() {
        let fragment = format!(
            "Supported value modes: {}.",
            join_cli_values(
                contract
                    .value_modes
                    .iter()
                    .copied()
                    .map(CliValue::ValueType)
            )
        );
        assert!(
            help.contains(&normalize_whitespace(&fragment)),
            "{fragment}"
        );
    }
    if let Some(default_output) = contract.default_output {
        let fragment = format!(
            "Default output mode: {}.",
            render_cli_value(CliValue::OutputMode(default_output))
        );
        assert!(
            help.contains(&normalize_whitespace(&fragment)),
            "{fragment}"
        );
    }
    for override_rule in &contract.default_output_overrides {
        let verb = if override_rule.when.values.len() == 1 {
            "is"
        } else {
            "is one of"
        };
        let fragment = format!(
            "Output default override: {} when {} {} {}.",
            render_cli_value(override_rule.value),
            override_rule.when.parameter,
            verb,
            join_cli_values(override_rule.when.values.iter().copied())
        );
        assert!(
            help.contains(&normalize_whitespace(&fragment)),
            "{fragment}"
        );
    }
    if !contract.output_modes.is_empty() {
        let fragment = format!(
            "Supported output modes: {}.",
            join_cli_values(
                contract
                    .output_modes
                    .iter()
                    .copied()
                    .map(CliValue::OutputMode)
            )
        );
        assert!(
            help.contains(&normalize_whitespace(&fragment)),
            "{fragment}"
        );
    }
    for note in &contract.notes {
        assert!(help.contains(&normalize_whitespace(note)), "{note}");
    }
}

#[test]
fn subcommand_help_renders_canonical_contract_modes_and_notes() {
    for (args, operation_id) in [
        (&["select", "--help"][..], OperationId::SelectExtract),
        (&["slice", "--help"][..], OperationId::SliceExtract),
        (
            &["inspect", "source", "--help"][..],
            OperationId::SourceInspect,
        ),
        (
            &["inspect", "select", "--help"][..],
            OperationId::SelectPreview,
        ),
        (
            &["inspect", "slice", "--help"][..],
            OperationId::SlicePreview,
        ),
    ] {
        assert_help_contains_contract(args, operation_id);
    }
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
fn unsupported_request_file_schema_still_carries_recovery_guidance() {
    let tempdir = tempdir().expect("tempdir");
    let request_path = write_json_file(
        tempdir.path(),
        "unsupported schema.json",
        r#"{
  "schema_name": "htmlcut.extraction_definition",
  "schema_version": 999,
  "request": {
    "source": { "input": { "type": "stdin" } },
    "extraction": { "kind": "selector", "selector": "article" }
  }
}"#,
    );

    Command::cargo_bin("htmlcut")
        .expect("binary")
        .args(["select", "--request-file"])
        .arg(&request_path)
        .assert()
        .failure()
        .code(2)
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
    let serialized = serde_json::to_string_pretty(&definition).expect("serialize definition");
    let parsed: ExtractionDefinition =
        serde_json::from_str(&serialized).expect("parse serialized definition");
    assert_eq!(
        parsed.request.extraction.strategy(),
        definition.request.extraction.strategy()
    );
    assert_eq!(
        parsed.schema_name,
        htmlcut_core::EXTRACTION_DEFINITION_SCHEMA_NAME
    );
}
