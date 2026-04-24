use super::*;
use std::io;

fn write_markdown_contract_repo(repo_root: &Path, readme_body: &str) {
    write_empty_release_targets_script(repo_root);
    fs::write(
        repo_root.join("Cargo.toml"),
        "[workspace.package]\nversion = \"4.1.0\"\n",
    )
    .expect("write manifest");
    fs::write(
        repo_root.join("README.md"),
        format!(
            "<!--\nAFAD:\n  afad: \"4.0\"\n  version: \"4.1.0\"\n  domain: PRODUCT\n  updated: \"2026-04-20\"\nRETRIEVAL_HINTS:\n  keywords: [htmlcut]\n  questions: [\"q\"]\n-->\n{readme_body}\n"
        ),
    )
    .expect("write readme");
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
        "---\nafad: \"4.0\"\nversion: \"4.1.0\"\ndomain: DOCS\nupdated: \"2026-04-20\"\nroute:\n  keywords: [guide]\n  questions: [\"q\"]\n---\nUse `htmlcut.extraction_result` and `select.extract`.\n",
    )
    .expect("write guide");
    write_schema_inventory_doc(repo_root);
    write_operations_inventory_doc(repo_root);
}

fn write_schema_inventory_doc(repo_root: &Path) {
    let schema_names = htmlcut_core::schema_catalog()
        .iter()
        .map(|descriptor| descriptor.schema_ref.schema_name.to_owned())
        .chain([
            htmlcut_cli::CATALOG_REPORT_SCHEMA_NAME.to_owned(),
            htmlcut_cli::SCHEMA_COMMAND_REPORT_SCHEMA_NAME.to_owned(),
            htmlcut_cli::EXTRACTION_COMMAND_REPORT_SCHEMA_NAME.to_owned(),
            htmlcut_cli::SOURCE_INSPECTION_COMMAND_REPORT_SCHEMA_NAME.to_owned(),
        ])
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
    let operations = htmlcut_core::operation_catalog()
        .iter()
        .map(|descriptor| format!("| `{}` |", descriptor.id.as_str()))
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
        "```text\nhtmlcut select [INPUT] --css <SELECTOR> [options]\nhtmlcut select page.html --css\n```\n",
    );

    let errors = markdown_contract_errors(repo_root.path()).expect("markdown contract errors");

    assert!(errors.iter().any(|error| error.contains(
        "README.md contains a non-parsing htmlcut example: htmlcut select page.html --css"
    )));
    assert!(
        errors
            .iter()
            .all(|error| !error.contains("htmlcut select [INPUT] --css <SELECTOR> [options]"))
    );
}

#[test]
fn markdown_contract_errors_report_non_runnable_htmlcut_examples() {
    let repo_root = tempdir().expect("tempdir");
    write_markdown_contract_repo(
        repo_root.path(),
        "```bash\nhtmlcut select missing.html --css article\n```\n",
    );

    let errors = markdown_contract_errors(repo_root.path()).expect("markdown contract errors");

    assert!(errors.iter().any(|error| error.contains(
        "README.md contains a non-runnable htmlcut example: htmlcut select missing.html --css article"
    )));
}

#[test]
fn markdown_contract_errors_execute_examples_and_verify_emitted_artifacts() {
    let repo_root = tempdir().expect("tempdir");
    write_markdown_contract_repo(
        repo_root.path(),
        "```bash\nhtmlcut select ./page.html \\\n  --css 'article a.more' \\\n  --value attribute \\\n  --attribute href \\\n  --emit-request-file ./article-links.json\nhtmlcut select --request-file ./article-links.json\nhtmlcut select ./page.html --css article --output-file ./article.txt\nhtmlcut select ./page.html --css article --bundle ./bundle\n```\n",
    );

    let errors = markdown_contract_errors(repo_root.path()).expect("markdown contract errors");

    assert!(errors.is_empty(), "unexpected errors: {errors:#?}");
}

#[test]
fn markdown_contract_errors_report_invalid_catalog_schema_and_command_examples() {
    let repo_root = tempdir().expect("tempdir");
    write_markdown_contract_repo(
        repo_root.path(),
        "```bash\nhtmlcut catalog --operation unknown.operation\nhtmlcut schema --name htmlcut.unknown_schema\nhtmlcut inspect --help\nhtmlcut select \"page name.html\" \\\n  --css 'article.hero'\n```\n",
    );

    let errors = markdown_contract_errors(repo_root.path()).expect("markdown contract errors");
    let known_schemas = htmlcut_core::schema_catalog()
        .iter()
        .map(|descriptor| descriptor.schema_ref.schema_name)
        .collect::<std::collections::BTreeSet<_>>();
    let known_operations = htmlcut_core::operation_catalog()
        .iter()
        .map(|descriptor| descriptor.id.as_str())
        .collect::<std::collections::BTreeSet<_>>();

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
        Some("README.md example references unknown CLI command path: inspect mystery".to_owned())
    );
}

#[test]
fn docs_helper_parsers_cover_quotes_multiline_examples_and_empty_command_paths() {
    assert_eq!(
        crate::docs::commands::extract_htmlcut_examples(
            "```bash\nhtmlcut select \"page name.html\" \\\n  --css 'article.hero'\n```\n"
        ),
        vec!["htmlcut select \"page name.html\" --css 'article.hero'".to_owned()]
    );
    assert_eq!(
        crate::docs::commands::shell_words(
            "htmlcut select \"page name.html\" --css 'article.hero'"
        )
        .expect("shell words"),
        vec![
            "htmlcut".to_owned(),
            "select".to_owned(),
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
    assert!(
        crate::docs::commands::command_path(&[]).is_empty(),
        "empty token streams should not invent a command path"
    );
    let known_schemas = htmlcut_core::schema_catalog()
        .iter()
        .map(|descriptor| descriptor.schema_ref.schema_name)
        .collect::<std::collections::BTreeSet<_>>();
    let known_operations = htmlcut_core::operation_catalog()
        .iter()
        .map(|descriptor| descriptor.id.as_str())
        .collect::<std::collections::BTreeSet<_>>();
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
        crate::docs::commands::command_path(&["htmlcut".to_owned(), "inspect".to_owned()]),
        vec!["inspect"]
    );
    assert_eq!(
        crate::docs::commands::command_reference_error(
            "README.md",
            &["htmlcut".to_owned(), "inspect".to_owned()],
            &known_schemas,
            &known_operations,
        ),
        Some("README.md example references unknown CLI command path: inspect".to_owned())
    );
}

#[test]
fn command_example_lint_reports_shell_parse_failures() {
    let schema_names = htmlcut_core::schema_catalog()
        .iter()
        .map(|descriptor| descriptor.schema_ref.schema_name)
        .collect::<std::collections::BTreeSet<_>>();
    let operation_ids = htmlcut_core::operation_catalog()
        .iter()
        .map(|descriptor| descriptor.id.as_str())
        .collect::<std::collections::BTreeSet<_>>();

    let errors = crate::docs::commands::command_example_errors(
        "README.md",
        "```bash\nhtmlcut select \"unterminated\n```\n",
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
        "htmlcut select page.html --css article --output-file /tmp/missing.txt",
        &[
            "htmlcut".to_owned(),
            "select".to_owned(),
            "page.html".to_owned(),
            "--css".to_owned(),
            "article".to_owned(),
            "--output-file".to_owned(),
            missing_file.to_string_lossy().into_owned(),
        ],
    )
    .expect("missing file error");
    assert!(
        file_error.contains(format!("expected file {} to exist", missing_file.display()).as_str())
    );

    let bundle_error = crate::docs::commands::testing::documented_artifact_error_for_tests(
        "README.md",
        "htmlcut select page.html --css article --bundle /tmp/bundle",
        &[
            "htmlcut".to_owned(),
            "select".to_owned(),
            "page.html".to_owned(),
            "--css".to_owned(),
            "article".to_owned(),
            "--bundle".to_owned(),
            missing_bundle.to_string_lossy().into_owned(),
        ],
    )
    .expect("missing bundle error");
    assert!(
        bundle_error.contains(
            format!(
                "expected bundle artifact {} to exist",
                missing_bundle.join("selection.html").display()
            )
            .as_str()
        )
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
        "htmlcut select page.html --css article",
        Err(io::Error::other("broken pipe")),
        &[],
        &[],
    )
    .expect("runtime error");

    assert!(error.contains("failed to capture CLI output: broken pipe"));
}
