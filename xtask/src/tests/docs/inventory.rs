use super::*;

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
            "---\nafad: \"3.5\"\nversion: \"4.1.0\"\ndomain: SCHEMA\nupdated: \"2026-04-20\"\nroute:\n  keywords: [schema]\n  questions: [\"q\"]\n---\n{schemas}\n"
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
            "---\nafad: \"3.5\"\nversion: \"4.1.0\"\ndomain: OPERATIONS\nupdated: \"2026-04-20\"\nroute:\n  keywords: [operations]\n  questions: [\"q\"]\n---\n| Operation ID |\n| --- |\n{operations}\n"
        ),
    )
    .expect("write operations doc");
}

fn write_workspace_inventory_repo(repo_root: &Path, workspace_doc_body: &str) {
    write_empty_release_targets_script(repo_root);
    fs::write(
        repo_root.join("Cargo.toml"),
        r#"[workspace]
members = [
    "crates/htmlcut-core",
    "crates/htmlcut-cli",
    "crates/htmlcut-tempdir",
    "fuzz",
    "xtask",
]

[workspace.package]
version = "4.1.0"
"#,
    )
    .expect("write manifest");
    fs::write(
        repo_root.join("README.md"),
        "<!--\nAFAD:\n  afad: \"3.5\"\n  version: \"4.1.0\"\n  domain: PRODUCT\n  updated: \"2026-04-20\"\nRETRIEVAL_HINTS:\n  keywords: [htmlcut]\n  questions: [\"q\"]\n-->\n",
    )
    .expect("write readme");
    fs::write(
        repo_root.join("CONTRIBUTING.md"),
        "<!--\nAFAD:\n  afad: \"3.5\"\n  version: \"4.1.0\"\n  domain: MAINTAINER\n  updated: \"2026-04-20\"\nRETRIEVAL_HINTS:\n  keywords: [contrib]\n  questions: [\"q\"]\n-->\n",
    )
    .expect("write contributing");
    write_minimal_docs_legal_scaffold(repo_root, "4.1.0", "2026-04-20");
    fs::create_dir_all(repo_root.join("fuzz")).expect("create fuzz dir");
    fs::write(
        repo_root.join("fuzz").join("README.md"),
        "<!--\nAFAD:\n  afad: \"3.5\"\n  version: \"4.1.0\"\n  domain: QUALITY\n  updated: \"2026-04-20\"\nRETRIEVAL_HINTS:\n  keywords: [fuzz]\n  questions: [\"q\"]\n-->\n",
    )
    .expect("write fuzz readme");
    fs::create_dir_all(repo_root.join("docs")).expect("create docs dir");
    fs::write(
        repo_root.join("docs").join("workspace-layout.md"),
        format!(
            "---\nafad: \"3.5\"\nversion: \"4.1.0\"\ndomain: WORKSPACE\nupdated: \"2026-04-20\"\nroute:\n  keywords: [workspace]\n  questions: [\"q\"]\n---\n{workspace_doc_body}\n"
        ),
    )
    .expect("write workspace-layout doc");
    write_schema_inventory_doc(repo_root);
    write_operations_inventory_doc(repo_root);
}

#[test]
fn markdown_contract_errors_report_missing_workspace_members_in_workspace_layout_doc() {
    let repo_root = tempdir().expect("tempdir");
    write_workspace_inventory_repo(
        repo_root.path(),
        "| Path |\n| --- |\n| `crates/htmlcut-core` |\n| `crates/htmlcut-cli` |\n| `xtask` |\n",
    );

    let errors = markdown_contract_errors(repo_root.path()).expect("markdown contract errors");

    assert!(errors.iter().any(|error| {
        error == "docs/workspace-layout.md is missing workspace members from Cargo.toml: crates/htmlcut-tempdir, fuzz"
    }));
}

#[test]
fn markdown_contract_errors_report_extra_workspace_members_in_workspace_layout_doc() {
    let repo_root = tempdir().expect("tempdir");
    write_workspace_inventory_repo(
        repo_root.path(),
        "| Path |\n| --- |\n| `crates/htmlcut-core` |\n| `crates/htmlcut-cli` |\n| `crates/htmlcut-tempdir` |\n| `fuzz` |\n| `xtask` |\n| `crates/not-real` |\n",
    );

    let errors = markdown_contract_errors(repo_root.path()).expect("markdown contract errors");

    assert!(errors.iter().any(|error| {
        error == "docs/workspace-layout.md documents workspace members not present in Cargo.toml: crates/not-real"
    }));
}

#[test]
fn inventory_errors_report_workspace_manifest_load_failures_for_workspace_layout_doc() {
    let repo_root = tempdir().expect("tempdir");
    fs::write(
        repo_root.path().join("Cargo.toml"),
        "[workspace]\nresolver = \"3\"\n",
    )
    .expect("write manifest");

    let errors = crate::docs::inventory_errors_for_tests(
        repo_root.path(),
        "docs/workspace-layout.md",
        "| Path |\n| --- |\n| `crates/htmlcut-core` |\n",
    );

    assert_eq!(
        errors,
        vec!["docs/workspace-layout.md could not load workspace members from Cargo.toml: workspace members not found in Cargo.toml".to_owned()]
    );
}
