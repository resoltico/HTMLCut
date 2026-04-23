mod support;
use support::*;

#[test]
fn help_prints_the_new_workflows_and_contract_language() {
    let mut command = Command::cargo_bin("htmlcut").expect("binary");
    command
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::starts_with(format!(
            "HTMLCut {}\n{}\n",
            expected_version(),
            env!("CARGO_PKG_DESCRIPTION")
        )))
        .stdout(predicate::str::contains("Start here:"))
        .stdout(predicate::str::contains("Reusable requests:"))
        .stdout(predicate::str::contains("catalog"))
        .stdout(predicate::str::contains("schema"))
        .stdout(predicate::str::contains("select"))
        .stdout(predicate::str::contains("slice"))
        .stdout(predicate::str::contains("inspect"))
        .stdout(predicate::str::contains("help"))
        .stdout(predicate::str::contains("--value"))
        .stdout(predicate::str::contains("--output"))
        .stdout(predicate::str::contains("--verbose"))
        .stdout(predicate::str::contains(
            "--emit-request-file writes the normalized extraction definition for the current run.",
        ))
        .stdout(predicate::str::contains(
            "--request-file reruns a saved definition instead of spelling the source and strategy inline.",
        ))
        .stdout(predicate::str::contains(
            "htmlcut select --request-file ./article-links.json --output-file ./links.json",
        ));
}

#[test]
fn select_help_stays_select_specific() {
    let mut command = Command::cargo_bin("htmlcut").expect("binary");
    command
        .args(["select", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Supported match modes: single, first, nth, all.",
        ))
        .stdout(predicate::str::contains(
            "Output default override: html when --value is one of inner-html, outer-html.",
        ))
        .stdout(predicate::str::contains(
            "Output default override: json when --value is structured.",
        ))
        .stdout(predicate::str::contains(
            "Attribute name to extract when `--value attribute` is used",
        ))
        .stdout(predicate::str::contains("The selected fragment excludes").not());
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
            "Regex flags for `--pattern regex`. Accepts `i`, `m`, `s`, `U`, `u`, and `x`",
        ))
        .stdout(predicate::str::contains(
            "`g` is accepted for compatibility and ignored",
        ))
        .stdout(predicate::str::contains(
            "The selected fragment excludes both matched boundaries by default;",
        ))
        .stdout(predicate::str::contains(
            "use --include-start when the opening tag lives in the start boundary.",
        ))
        .stdout(predicate::str::contains(
            "For --value inner-html, HTMLCut returns the selected fragment as HTML. For --value outer-html, HTMLCut returns the full outer matched range including both boundaries.",
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
fn help_subcommand_reuses_the_root_identity_banner() {
    let mut command = Command::cargo_bin("htmlcut").expect("binary");
    command
        .arg("help")
        .assert()
        .success()
        .stdout(predicate::str::starts_with(format!(
            "HTMLCut {}\n{}\n",
            expected_version(),
            env!("CARGO_PKG_DESCRIPTION")
        )))
        .stdout(predicate::str::contains(
            "Usage: htmlcut [OPTIONS] <COMMAND>",
        ));
}

#[test]
fn subcommand_version_is_rejected_as_usage_error() {
    let mut command = Command::cargo_bin("htmlcut").expect("binary");
    command
        .args(["select", "--version"])
        .assert()
        .failure()
        .code(2)
        .stdout("")
        .stderr(predicate::str::contains("unexpected argument '--version'"));
}

#[test]
fn parse_errors_do_not_switch_to_json_just_because_a_positional_argument_is_named_inspect() {
    let mut command = Command::cargo_bin("htmlcut").expect("binary");
    command
        .args(["select", "inspect"])
        .assert()
        .failure()
        .code(2)
        .stdout("")
        .stderr(predicate::str::contains(
            "the following required arguments were not provided:",
        ))
        .stderr(predicate::str::contains("\"tool\":").not());
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
