use super::*;

#[test]
fn markdown_contract_errors_report_unknown_schema_names_and_operation_ids() {
    let repo_root = tempdir().expect("tempdir");
    write_empty_release_targets_script(repo_root.path());
    fs::write(
        repo_root.path().join("Cargo.toml"),
        "[workspace.package]\nversion = \"4.1.0\"\n",
    )
    .expect("write manifest");
    fs::write(
        repo_root.path().join("README.md"),
        "<!--\nAFAD:\n  afad: \"3.5\"\n  version: \"4.1.0\"\n  domain: PRODUCT\n  updated: \"2026-04-20\"\nRETRIEVAL_HINTS:\n  keywords: [htmlcut]\n  questions: [\"q\"]\n-->\nUse `htmlcut.unknown_report` and `select.inspect`.\n",
    )
    .expect("write readme");
    fs::write(
        repo_root.path().join("CONTRIBUTING.md"),
        "<!--\nAFAD:\n  afad: \"3.5\"\n  version: \"4.1.0\"\n  domain: MAINTAINER\n  updated: \"2026-04-20\"\nRETRIEVAL_HINTS:\n  keywords: [contrib]\n  questions: [\"q\"]\n-->\n",
    )
    .expect("write contributing");
    fs::write(
        repo_root.path().join("PATENTS.md"),
        "<!--\nAFAD:\n  afad: \"3.5\"\n  version: \"4.1.0\"\n  domain: LEGAL\n  updated: \"2026-04-20\"\nRETRIEVAL_HINTS:\n  keywords: [patents]\n  questions: [\"q\"]\n-->\n",
    )
    .expect("write patents");
    fs::create_dir_all(repo_root.path().join("fuzz")).expect("create fuzz dir");
    fs::write(
        repo_root.path().join("fuzz").join("README.md"),
        "<!--\nAFAD:\n  afad: \"3.5\"\n  version: \"4.1.0\"\n  domain: QUALITY\n  updated: \"2026-04-20\"\nRETRIEVAL_HINTS:\n  keywords: [fuzz]\n  questions: [\"q\"]\n-->\n",
    )
    .expect("write fuzz readme");
    fs::create_dir_all(repo_root.path().join("docs")).expect("create docs dir");
    fs::write(
        repo_root.path().join("docs").join("guide.md"),
        "---\nafad: \"3.5\"\nversion: \"4.1.0\"\ndomain: DOCS\nupdated: \"2026-04-20\"\nroute:\n  keywords: [guide]\n  questions: [\"q\"]\n---\nUse `htmlcut.extraction_result` and `select.extract`.\n",
    )
    .expect("write guide");

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
    write_empty_release_targets_script(repo_root.path());
    fs::write(
        repo_root.path().join("Cargo.toml"),
        "[workspace.package]\nversion = \"4.1.0\"\n",
    )
    .expect("write manifest");
    fs::write(
        repo_root.path().join("README.md"),
        "<!--\nAFAD:\n  afad: \"3.5\"\n  version: \"4.1.0\"\n  domain: PRODUCT\n  updated: \"2026-04-20\"\nRETRIEVAL_HINTS:\n  keywords: [htmlcut]\n  questions: [\"q\"]\n-->\n```text\nhtmlcut select [INPUT] --css <SELECTOR> [options]\nhtmlcut select page.html --css\n```\n",
    )
    .expect("write readme");
    fs::write(
        repo_root.path().join("CONTRIBUTING.md"),
        "<!--\nAFAD:\n  afad: \"3.5\"\n  version: \"4.1.0\"\n  domain: MAINTAINER\n  updated: \"2026-04-20\"\nRETRIEVAL_HINTS:\n  keywords: [contrib]\n  questions: [\"q\"]\n-->\n",
    )
    .expect("write contributing");
    fs::write(
        repo_root.path().join("PATENTS.md"),
        "<!--\nAFAD:\n  afad: \"3.5\"\n  version: \"4.1.0\"\n  domain: LEGAL\n  updated: \"2026-04-20\"\nRETRIEVAL_HINTS:\n  keywords: [patents]\n  questions: [\"q\"]\n-->\n",
    )
    .expect("write patents");
    fs::create_dir_all(repo_root.path().join("fuzz")).expect("create fuzz dir");
    fs::write(
        repo_root.path().join("fuzz").join("README.md"),
        "<!--\nAFAD:\n  afad: \"3.5\"\n  version: \"4.1.0\"\n  domain: QUALITY\n  updated: \"2026-04-20\"\nRETRIEVAL_HINTS:\n  keywords: [fuzz]\n  questions: [\"q\"]\n-->\n",
    )
    .expect("write fuzz readme");
    fs::create_dir_all(repo_root.path().join("docs")).expect("create docs dir");
    fs::write(
        repo_root.path().join("docs").join("schema.md"),
        "---\nafad: \"3.5\"\nversion: \"4.1.0\"\ndomain: SCHEMA\nupdated: \"2026-04-20\"\nroute:\n  keywords: [schema]\n  questions: [\"q\"]\n---\n- `htmlcut.source_request`\n- `htmlcut.runtime_options`\n- `htmlcut.inspection_options`\n- `htmlcut.extraction_request`\n- `htmlcut.extraction_definition`\n- `htmlcut.extraction_result`\n- `htmlcut.source_inspection_result`\n- `htmlcut.catalog_report`\n- `htmlcut.schema_report`\n- `htmlcut.extraction_report`\n- `htmlcut.source_inspection_report`\n- `htmlcut.plan`\n- `htmlcut.result`\n- `htmlcut.error`\n",
    )
    .expect("write schema doc");
    fs::write(
        repo_root.path().join("docs").join("operations.md"),
        "---\nafad: \"3.5\"\nversion: \"4.1.0\"\ndomain: OPERATIONS\nupdated: \"2026-04-20\"\nroute:\n  keywords: [operations]\n  questions: [\"q\"]\n---\n| Operation ID |\n| --- |\n| `document.parse` |\n| `source.inspect` |\n| `select.preview` |\n| `slice.preview` |\n| `select.extract` |\n| `slice.extract` |\n",
    )
    .expect("write operations doc");

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
fn markdown_contract_errors_report_invalid_catalog_schema_and_command_examples() {
    let repo_root = tempdir().expect("tempdir");
    write_empty_release_targets_script(repo_root.path());
    fs::write(
        repo_root.path().join("Cargo.toml"),
        "[workspace.package]\nversion = \"4.1.0\"\n",
    )
    .expect("write manifest");
    fs::write(
        repo_root.path().join("README.md"),
        "<!--\nAFAD:\n  afad: \"3.5\"\n  version: \"4.1.0\"\n  domain: PRODUCT\n  updated: \"2026-04-20\"\nRETRIEVAL_HINTS:\n  keywords: [htmlcut]\n  questions: [\"q\"]\n-->\n```bash\nhtmlcut catalog --operation unknown.operation\nhtmlcut schema --name htmlcut.unknown_schema\nhtmlcut inspect --help\nhtmlcut select \"page name.html\" \\\n  --css 'article.hero'\n```\n",
    )
    .expect("write readme");
    fs::write(
        repo_root.path().join("CONTRIBUTING.md"),
        "<!--\nAFAD:\n  afad: \"3.5\"\n  version: \"4.1.0\"\n  domain: MAINTAINER\n  updated: \"2026-04-20\"\nRETRIEVAL_HINTS:\n  keywords: [contrib]\n  questions: [\"q\"]\n-->\n",
    )
    .expect("write contributing");
    fs::write(
        repo_root.path().join("PATENTS.md"),
        "<!--\nAFAD:\n  afad: \"3.5\"\n  version: \"4.1.0\"\n  domain: LEGAL\n  updated: \"2026-04-20\"\nRETRIEVAL_HINTS:\n  keywords: [patents]\n  questions: [\"q\"]\n-->\n",
    )
    .expect("write patents");
    fs::create_dir_all(repo_root.path().join("fuzz")).expect("create fuzz dir");
    fs::write(
        repo_root.path().join("fuzz").join("README.md"),
        "<!--\nAFAD:\n  afad: \"3.5\"\n  version: \"4.1.0\"\n  domain: QUALITY\n  updated: \"2026-04-20\"\nRETRIEVAL_HINTS:\n  keywords: [fuzz]\n  questions: [\"q\"]\n-->\n",
    )
    .expect("write fuzz readme");
    fs::create_dir_all(repo_root.path().join("docs")).expect("create docs dir");
    fs::write(
        repo_root.path().join("docs").join("schema.md"),
        "---\nafad: \"3.5\"\nversion: \"4.1.0\"\ndomain: SCHEMA\nupdated: \"2026-04-20\"\nroute:\n  keywords: [schema]\n  questions: [\"q\"]\n---\n- `htmlcut.source_request`\n- `htmlcut.runtime_options`\n- `htmlcut.inspection_options`\n- `htmlcut.extraction_request`\n- `htmlcut.extraction_definition`\n- `htmlcut.extraction_result`\n- `htmlcut.source_inspection_result`\n- `htmlcut.catalog_report`\n- `htmlcut.schema_report`\n- `htmlcut.extraction_report`\n- `htmlcut.source_inspection_report`\n- `htmlcut.plan`\n- `htmlcut.result`\n- `htmlcut.error`\n",
    )
    .expect("write schema doc");
    fs::write(
        repo_root.path().join("docs").join("operations.md"),
        "---\nafad: \"3.5\"\nversion: \"4.1.0\"\ndomain: OPERATIONS\nupdated: \"2026-04-20\"\nroute:\n  keywords: [operations]\n  questions: [\"q\"]\n---\n| Operation ID |\n| --- |\n| `document.parse` |\n| `source.inspect` |\n| `select.preview` |\n| `slice.preview` |\n| `select.extract` |\n| `slice.extract` |\n",
    )
    .expect("write operations doc");

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
        crate::docs::command_reference_error_for_tests(
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
        crate::docs::extract_htmlcut_examples_for_tests(
            "```bash\nhtmlcut select \"page name.html\" \\\n  --css 'article.hero'\n```\n"
        ),
        vec!["htmlcut select \"page name.html\" --css 'article.hero'".to_owned()]
    );
    assert_eq!(
        crate::docs::shell_words_for_tests(
            "htmlcut select \"page name.html\" --css 'article.hero'"
        ),
        vec![
            "htmlcut".to_owned(),
            "select".to_owned(),
            "page name.html".to_owned(),
            "--css".to_owned(),
            "article.hero".to_owned(),
        ]
    );
    assert_eq!(
        crate::docs::shell_words_for_tests("htmlcut catalog --output json"),
        vec![
            "htmlcut".to_owned(),
            "catalog".to_owned(),
            "--output".to_owned(),
            "json".to_owned(),
        ]
    );
    assert!(
        crate::docs::command_path_for_tests(&[]).is_empty(),
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
        crate::docs::command_reference_error_for_tests(
            "README.md",
            &["htmlcut".to_owned(), "mystery".to_owned()],
            &known_schemas,
            &known_operations,
        ),
        None
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

    let errors = crate::docs::command_example_errors_for_tests(
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
