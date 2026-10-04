// SPDX-License-Identifier: MPL-2.0
use super::*;
use std::io;

fn write_markdown_contract_repo(repo_root: &Path, readme_body: &str) {
    write_empty_release_targets_script(repo_root);
    fs::write(
        repo_root.join("Cargo.toml"),
        "[workspace.package]\nversion = \"4.1.0\"\n",
    )
    .expect("write manifest");
    fs::write(repo_root.join("README.md"), readme_body).expect("write readme");
    fs::write(
        repo_root.join("CONTRIBUTING.md"),
        "<!--\nAFAD:\n  afad: \"4.0\"\n  version: \"4.1.0\"\n  domain: MAINTAINER\n  updated: \"2026-04-20\"\nRETRIEVAL_HINTS:\n  keywords: [contrib]\n  questions: [\"q\"]\n-->\n",
    )
    .expect("write contributing");
    write_minimal_docs_legal_scaffold(repo_root, "4.1.0", "2026-04-20");
    fs::create_dir_all(repo_root.join("fuzz")).expect("create fuzz dir");
    fs::write(
        repo_root.join("fuzz").join("README.md"),
        "<!--\nAFAD:\n  afad: \"4.0\"\n  version: \"4.1.0\"\n  domain: QUALITY\n  updated: \"2026-04-20\"\nRETRIEVAL_HINTS:\n  keywords: [fuzz]\n  questions: [\"q\"]\n-->\n",
    )
    .expect("write fuzz readme");
    fs::create_dir_all(repo_root.join("docs")).expect("create docs dir");
    fs::write(
        repo_root.join("docs").join("guide.md"),
        "---\nafad: \"4.0\"\nversion: \"4.1.0\"\ndomain: DOCS\nupdated: \"2026-04-20\"\nroute:\n  keywords: [guide]\n  questions: [\"q\"]\n---\nUse `htmlcut.extraction.receipt` and `extract`.\n",
    )
    .expect("write guide");
    write_schema_inventory_doc(repo_root);
    write_operations_inventory_doc(repo_root);
}

fn write_schema_inventory_doc(repo_root: &Path) {
    let schema_names = crate::docs::known_schema_names_for_tests()
        .into_iter()
        .map(str::to_owned)
        .collect::<std::collections::BTreeSet<_>>();
    let schemas = schema_names
        .into_iter()
        .map(|schema_name| format!("- `{schema_name}`"))
        .collect::<Vec<_>>()
        .join("\n");

    fs::write(
        repo_root.join("docs").join("schema.md"),
        format!(
            "---\nafad: \"4.0\"\nversion: \"4.1.0\"\ndomain: SCHEMA\nupdated: \"2026-04-20\"\nroute:\n  keywords: [schema]\n  questions: [\"q\"]\n---\n{schemas}\n"
        ),
    )
    .expect("write schema doc");
}

fn write_operations_inventory_doc(repo_root: &Path) {
    let operations = crate::docs::known_operation_ids_for_tests()
        .into_iter()
        .map(|name| format!("| `{name}` |"))
        .collect::<Vec<_>>()
        .join("\n");

    fs::write(
        repo_root.join("docs").join("operations.md"),
        format!(
            "---\nafad: \"4.0\"\nversion: \"4.1.0\"\ndomain: OPERATIONS\nupdated: \"2026-04-20\"\nroute:\n  keywords: [operations]\n  questions: [\"q\"]\n---\n| Operation ID |\n| --- |\n{operations}\n"
        ),
    )
    .expect("write operations doc");
}

#[test]
fn markdown_contract_errors_report_unknown_schema_names_and_operation_ids() {
    let repo_root = tempdir().expect("tempdir");
    write_markdown_contract_repo(
        repo_root.path(),
        "Use `htmlcut.unknown_report` and `select.inspect`.",
    );

    let errors = markdown_contract_errors(repo_root.path()).expect("markdown contract errors");

    assert!(
        errors.iter().any(
            |error| error == "README.md references unknown schema name: htmlcut.unknown_report"
        )
    );
    assert!(
        errors
            .iter()
            .any(|error| error == "README.md references unknown operation ID: select.inspect")
    );
}

#[test]
fn markdown_contract_errors_report_non_parsing_htmlcut_examples_but_ignore_synopsis() {
    let repo_root = tempdir().expect("tempdir");
    write_markdown_contract_repo(
        repo_root.path(),
        "```text\nhtmlcut extract --file [INPUT] --css <SELECTOR> [options]\nhtmlcut extract --file page.html --css\n```\n",
    );

    let errors = markdown_contract_errors(repo_root.path()).expect("markdown contract errors");

    assert!(errors.iter().any(|error| error.contains(
        "README.md contains a non-parsing htmlcut example: htmlcut extract --file page.html --css"
    )));
    assert!(
        errors
            .iter()
            .all(|error| !error
                .contains("htmlcut extract --file [INPUT] --css <SELECTOR> [options]"))
    );
}

#[test]
fn markdown_contract_errors_report_non_runnable_htmlcut_examples() {
    let repo_root = tempdir().expect("tempdir");
    write_markdown_contract_repo(
        repo_root.path(),
        "```bash\nhtmlcut extract --file missing.html --css article\n```\n",
    );

    let errors = markdown_contract_errors(repo_root.path()).expect("markdown contract errors");

    assert!(errors.iter().any(|error| error.contains(
        "README.md contains a non-runnable htmlcut example: htmlcut extract --file missing.html --css article"
    )));
}

#[test]
fn markdown_contract_errors_execute_examples_and_verify_emitted_artifacts() {
    let repo_root = tempdir().expect("tempdir");
    write_markdown_contract_repo(
        repo_root.path(),
        "```bash\nhtmlcut extract --file ./page.html --css 'article a.more' --read attribute:href --bundle ./article-links.htmlcut.tar\nhtmlcut run ./article-links.htmlcut.tar\nhtmlcut extract --file ./page.html --css article --output ./article.txt\nhtmlcut extract --file ./page.html --css article --receipt ./receipt.json\n```\n",
    );

    let errors = markdown_contract_errors(repo_root.path()).expect("markdown contract errors");

    assert!(errors.is_empty(), "unexpected errors: {errors:#?}");
}

#[test]
fn markdown_contract_errors_report_invalid_catalog_schema_and_command_examples() {
    let repo_root = tempdir().expect("tempdir");
    write_markdown_contract_repo(
        repo_root.path(),
        "```bash\nhtmlcut describe unknown.operation\nhtmlcut schema htmlcut.unknown_schema\nhtmlcut inspect --help\nhtmlcut extract --file \"page name.html\" \\\n  --css 'article.hero'\n```\n",
    );

    let errors = markdown_contract_errors(repo_root.path()).expect("markdown contract errors");
    let known_schemas = crate::docs::known_schema_names_for_tests();
    let known_operations = crate::docs::known_operation_ids_for_tests();

    assert!(errors.iter().any(
        |error| error == "README.md example references unknown operation ID: unknown.operation"
    ));
    assert!(
        errors.iter().any(|error| error
            == "README.md example references unknown schema name: htmlcut.unknown_schema")
    );
    assert_eq!(
        crate::docs::commands::command_reference_error(
            "README.md",
            &[
                "htmlcut".to_owned(),
                "inspect".to_owned(),
                "mystery".to_owned(),
            ],
            &known_schemas,
            &known_operations,
        ),
        None
    );
}

#[test]
fn docs_helper_parsers_cover_quotes_multiline_examples_and_empty_command_paths() {
    assert_eq!(
        crate::docs::commands::extract_htmlcut_examples(
            "```bash\nhtmlcut extract --file \"page name.html\" \\\n  --css 'article.hero'\n```\n"
        ),
        vec!["htmlcut extract --file \"page name.html\" --css 'article.hero'".to_owned()]
    );
    assert_eq!(
        crate::docs::commands::extract_htmlcut_examples(
            "htmlcut extract --file outside.md\n```bash\necho htmlcut extract --file ignored.md\nhtmlcut catalog --output json\n```\n"
        ),
        vec!["htmlcut catalog --output json".to_owned()]
    );
    assert_eq!(
        crate::docs::commands::shell_words(
            "htmlcut extract --file \"page name.html\" --css 'article.hero'"
        )
        .expect("shell words"),
        vec![
            "htmlcut".to_owned(),
            "extract".to_owned(),
            "--file".to_owned(),
            "page name.html".to_owned(),
            "--css".to_owned(),
            "article.hero".to_owned(),
        ]
    );
    assert_eq!(
        crate::docs::commands::shell_words("htmlcut catalog --output json").expect("shell words"),
        vec![
            "htmlcut".to_owned(),
            "catalog".to_owned(),
            "--output".to_owned(),
            "json".to_owned(),
        ]
    );
    assert_eq!(
        crate::docs::commands::command_reference_error(
            "README.md",
            &[],
            &crate::docs::known_schema_names_for_tests(),
            &crate::docs::known_operation_ids_for_tests(),
        ),
        None
    );
    let known_schemas = crate::docs::known_schema_names_for_tests();
    let known_operations = crate::docs::known_operation_ids_for_tests();
    assert_eq!(
        crate::docs::commands::command_reference_error(
            "README.md",
            &["htmlcut".to_owned(), "mystery".to_owned()],
            &known_schemas,
            &known_operations,
        ),
        None
    );

    assert_eq!(
        crate::docs::commands::command_reference_error(
            "README.md",
            &["htmlcut".to_owned(), "inspect".to_owned()],
            &known_schemas,
            &known_operations,
        ),
        None
    );
}

#[test]
fn binary_argument_errors_remain_actionable_without_exposing_input_values() {
    let errors = crate::docs::commands::command_example_errors(
        "README.md",
        "```bash\nhtmlcut unknown-command\n```\n",
        &crate::docs::known_schema_names_for_tests(),
        &crate::docs::known_operation_ids_for_tests(),
    );
    assert!(
        errors
            .iter()
            .any(|error| error.contains("non-parsing htmlcut example") && error.contains("--help")),
        "actual documentation adapter diagnostics: {errors:?}"
    );
}

#[test]
fn command_example_lint_reports_shell_parse_failures() {
    let schema_names = crate::docs::known_schema_names_for_tests();
    let operation_ids = crate::docs::known_operation_ids_for_tests();

    let errors = crate::docs::commands::command_example_errors(
        "README.md",
        "```bash\nhtmlcut extract --file \"unterminated\n```\n",
        &schema_names,
        &operation_ids,
    );

    assert!(
        errors
            .iter()
            .any(|error| error.contains("contains a non-parsing htmlcut example"))
    );
}

#[test]
fn docs_runtime_helpers_report_missing_artifacts_and_execution_failure_fallbacks() {
    let repo_root = tempdir().expect("tempdir");
    let missing_file = repo_root.path().join("missing.txt");
    let missing_bundle = repo_root.path().join("bundle");

    let file_error = crate::docs::commands::testing::documented_artifact_error_for_tests(
        "README.md",
        "htmlcut extract --file page.html --css article --output /tmp/missing.txt",
        &[
            "htmlcut".to_owned(),
            "extract".to_owned(),
            "--file".to_owned(),
            "page.html".to_owned(),
            "--css".to_owned(),
            "article".to_owned(),
            "--output".to_owned(),
            missing_file.to_string_lossy().into_owned(),
        ],
    )
    .expect("missing file error");
    assert!(
        file_error.contains(format!("expected file {} to exist", missing_file.display()).as_str())
    );

    let bundle_error = crate::docs::commands::testing::documented_artifact_error_for_tests(
        "README.md",
        "htmlcut extract --file page.html --css article --bundle /tmp/bundle",
        &[
            "htmlcut".to_owned(),
            "extract".to_owned(),
            "--file".to_owned(),
            "page.html".to_owned(),
            "--css".to_owned(),
            "article".to_owned(),
            "--bundle".to_owned(),
            missing_bundle.to_string_lossy().into_owned(),
        ],
    )
    .expect("missing bundle error");
    assert!(
        bundle_error
            .contains(format!("expected file {} to exist", missing_bundle.display()).as_str())
    );

    assert_eq!(
        crate::docs::commands::testing::render_execution_failure_for_tests(3, b"", b"broken\n"),
        "exit code 3; stderr: broken"
    );
    assert_eq!(
        crate::docs::commands::testing::render_execution_failure_for_tests(3, b"printed\n", b" \n",),
        "exit code 3; stdout: printed"
    );
    assert_eq!(
        crate::docs::commands::testing::render_execution_failure_for_tests(3, b" \n", b"\n"),
        "exit code 3"
    );
}

#[test]
fn docs_runtime_helpers_report_injected_sandbox_failures() {
    assert!(
        crate::docs::commands::testing::prepare_sandbox_errors_for_tests("README.md", None, None)
            .is_empty(),
        "sandbox preparation without injected failures should succeed"
    );
    assert_eq!(
        crate::docs::commands::testing::prepare_sandbox_errors_for_tests(
            "README.md",
            Some("disk full"),
            None,
        ),
        vec![
            "README.md could not initialize the htmlcut docs-example sandbox: disk full".to_owned()
        ]
    );
    assert_eq!(
        crate::docs::commands::testing::prepare_sandbox_errors_for_tests(
            "README.md",
            None,
            Some("permission denied"),
        ),
        vec![
            "README.md could not enter the htmlcut docs-example sandbox: permission denied"
                .to_owned()
        ]
    );
    assert_eq!(
        crate::docs::commands::testing::injected_sandbox_error_for_tests("injected failure"),
        vec!["injected failure".to_owned()]
    );
}

#[test]
fn docs_runtime_helpers_report_cli_output_capture_failures() {
    let error = crate::docs::commands::testing::command_runtime_error_message_for_tests(
        "README.md",
        "htmlcut extract --file page.html --css article",
        Err(io::Error::other("broken pipe")),
        &[],
        &[],
    )
    .expect("runtime error");

    assert!(error.contains("failed to capture CLI output: broken pipe"));
}

#[test]
fn help_options_are_executed_without_being_mistaken_for_schema_or_operation_names() {
    for operation in ["describe", "schema"] {
        let errors = crate::docs::commands::command_example_errors(
            "README.md",
            &format!("```bash\nhtmlcut {operation} --help\n```\n"),
            &crate::docs::known_schema_names_for_tests(),
            &crate::docs::known_operation_ids_for_tests(),
        );
        assert!(errors.is_empty(), "{errors:?}");
    }
}

#[test]
fn digest_domain_suffixes_do_not_exempt_bare_unknown_schema_names() {
    let repo_root = tempdir().expect("tempdir");
    write_markdown_contract_repo(
        repo_root.path(),
        "Identities use htmlcut.plan/3, htmlcut.prepared/3 and htmlcut.extraction/3. Bare htmlcut.plan is not a schema. Bundles end in .htmlcut.tar.",
    );
    let errors = markdown_contract_errors(repo_root.path()).unwrap();
    assert!(
        errors
            .iter()
            .any(|error| error == "README.md references unknown schema name: htmlcut.plan")
    );
    assert!(!errors.iter().any(|error| error.contains("htmlcut.tar")));
}
