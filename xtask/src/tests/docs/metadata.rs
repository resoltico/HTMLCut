use super::*;
use regex::Regex;

#[test]
fn metadata_version_reads_yaml_frontmatter() {
    let text = "---\nafad: \"3.5\"\nversion: \"4.1.0\"\n---\n# Title\n";

    assert_eq!(
        crate::docs::metadata_version(text, crate::docs::MetadataStyle::Frontmatter),
        Some("4.1.0".to_owned())
    );
}

#[test]
fn metadata_version_reads_html_comment_metadata() {
    let text = "<!--\nAFAD:\n  afad: \"3.5\"\n  version: \"4.1.0\"\n-->\n# Title\n";

    assert_eq!(
        crate::docs::metadata_version(text, crate::docs::MetadataStyle::HtmlComment),
        Some("4.1.0".to_owned())
    );
}

#[test]
fn metadata_version_returns_none_for_missing_or_versionless_frontmatter() {
    let missing_frontmatter = "version: \"4.1.0\"\n# Title\n";
    let versionless_frontmatter = "---\nafad: \"3.5\"\n---\n# Title\n";

    assert_eq!(
        crate::docs::metadata_version(missing_frontmatter, crate::docs::MetadataStyle::Frontmatter),
        None
    );
    assert_eq!(
        crate::docs::metadata_version(
            versionless_frontmatter,
            crate::docs::MetadataStyle::Frontmatter,
        ),
        None
    );
}

#[test]
fn markdown_contract_errors_report_missing_and_mismatched_versions() {
    let repo_root = tempdir().expect("tempdir");
    write_empty_release_targets_script(repo_root.path());
    fs::write(
        repo_root.path().join("Cargo.toml"),
        "[workspace.package]\nversion = \"4.1.0\"\n",
    )
    .expect("write manifest");
    fs::write(
        repo_root.path().join("README.md"),
        "<!--\nAFAD:\n  afad: \"3.5\"\n  domain: PRODUCT\n  updated: \"2026-04-20\"\nRETRIEVAL_HINTS:\n  keywords: [htmlcut]\n  questions: [\"q\"]\n-->\n",
    )
    .expect("write readme");
    fs::write(
        repo_root.path().join("CONTRIBUTING.md"),
        "<!--\nAFAD:\n  afad: \"3.5\"\n  version: \"9.9.9\"\n  domain: MAINTAINER\n  updated: \"2026-04-20\"\nRETRIEVAL_HINTS:\n  keywords: [contrib]\n  questions: [\"q\"]\n-->\n",
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
        "---\nafad: \"3.5\"\ndomain: DOCS\nupdated: \"2026-04-20\"\nroute:\n  keywords: [guide]\n  questions: [\"q\"]\n---\n",
    )
    .expect("write guide");
    fs::write(
        repo_root.path().join("docs").join("reference.md"),
        "---\nafad: \"3.5\"\nversion: \"9.9.9\"\ndomain: DOCS\nupdated: \"2026-04-20\"\nroute:\n  keywords: [reference]\n  questions: [\"q\"]\n---\n",
    )
    .expect("write reference");

    let errors = markdown_contract_errors(repo_root.path()).expect("markdown contract errors");

    assert!(
        errors.iter().any(|error| error
            == "README.md is missing the expected HTML comment metadata version entry")
    );
    assert!(
        errors
            .iter()
            .any(|error| error == "CONTRIBUTING.md metadata version is 9.9.9, expected 4.1.0")
    );
    assert!(
        errors.iter().any(|error| error
            == "docs/guide.md is missing the expected frontmatter metadata version entry")
    );
    assert!(
        errors
            .iter()
            .any(|error| error == "docs/reference.md metadata version is 9.9.9, expected 4.1.0")
    );
}

#[test]
fn markdown_contract_errors_report_missing_metadata_fields_and_inventory_drift() {
    let repo_root = tempdir().expect("tempdir");
    write_empty_release_targets_script(repo_root.path());
    fs::write(
        repo_root.path().join("Cargo.toml"),
        "[workspace.package]\nversion = \"4.1.0\"\n",
    )
    .expect("write manifest");
    fs::write(
        repo_root.path().join("README.md"),
        "<!--\nAFAD:\n  afad: \"3.4\"\n  version: \"4.1.0\"\n  domain: PRODUCT\n  updated: \"2026/04/20\"\nRETRIEVAL_HINTS:\n  questions: [\"q\"]\n-->\n",
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
        "---\nafad: \"3.5\"\nversion: \"4.1.0\"\ndomain: SCHEMA\nupdated: \"2026-04-20\"\nroute:\n  keywords: [schema]\n  questions: [\"q\"]\n---\n- `htmlcut.source_request`\n",
    )
    .expect("write schema doc");
    fs::write(
        repo_root.path().join("docs").join("operations.md"),
        "---\nafad: \"3.5\"\nversion: \"4.1.0\"\ndomain: OPERATIONS\nupdated: \"2026-04-20\"\nroute:\n  keywords: [operations]\n---\n| Operation ID | CLI surface |\n| --- | --- |\n| `source.inspect` | `inspect source` |\n",
    )
    .expect("write operations doc");

    let errors = markdown_contract_errors(repo_root.path()).expect("markdown contract errors");

    assert!(
        errors
            .iter()
            .any(|error| error == "README.md metadata afad is 3.4, expected 3.5")
    );
    assert!(
        errors.iter().any(|error| error
            == "README.md metadata updated value is not ISO-8601 YYYY-MM-DD: 2026/04/20")
    );
    assert!(errors.iter().any(|error| error
        == "README.md is missing the expected HTML comment RETRIEVAL_HINTS keywords entry"));
    assert!(errors.iter().any(|error| error
        == "docs/operations.md is missing the expected frontmatter route questions entry"));
    assert!(errors.iter().any(|error| {
        error.starts_with("docs/schema.md is missing schema names from the registry:")
    }));
    assert!(errors.iter().any(|error| {
        error.starts_with("docs/operations.md is missing operation IDs from the catalog:")
    }));
}

#[test]
fn markdown_contract_errors_report_malformed_metadata_blocks_and_empty_values() {
    let repo_root = tempdir().expect("tempdir");
    write_empty_release_targets_script(repo_root.path());
    fs::write(
        repo_root.path().join("Cargo.toml"),
        "[workspace.package]\nversion = \"4.1.0\"\n",
    )
    .expect("write manifest");
    fs::write(repo_root.path().join("README.md"), "# no metadata\n").expect("write readme");
    fs::write(
        repo_root.path().join("CONTRIBUTING.md"),
        "<!--\nAFAD:\n  afad: \"3.5\"\n  version: \"4.1.0\"\n  domain: MAINTAINER\n  updated:\nRETRIEVAL_HINTS:\n  keywords: [contrib]\n  questions: [\"q\"]\n-->\n",
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
        "---\nafad: \"3.5\"\nversion: \"4.1.0\"\ndomain: DOCS\nupdated: \"2026-04-20\"\nroute:\n  keywords: [guide]\n  questions: [\"q\"]\n",
    )
    .expect("write guide");

    let errors = markdown_contract_errors(repo_root.path()).expect("markdown contract errors");

    assert!(
        errors
            .iter()
            .any(|error| error == "README.md is missing the expected HTML comment metadata block")
    );
    assert!(errors.iter().any(|error| error
        == "CONTRIBUTING.md is missing the expected HTML comment metadata updated entry"));
    assert!(
        errors.iter().any(
            |error| error == "docs/guide.md is missing the expected frontmatter metadata block"
        )
    );
}

#[test]
fn metadata_helpers_report_missing_frontmatter_and_html_comment_fields_directly() {
    let updated_pattern = Regex::new(r"^\d{4}-\d{2}-\d{2}$").expect("updated pattern");

    let frontmatter_errors = crate::docs::metadata_contract_errors_for_tests(
        "docs/guide.md",
        "---\nversion: \"4.1.0\"\nupdated: \"2026-04-20\"\n---\n",
        crate::docs::MetadataStyle::Frontmatter,
        &updated_pattern,
    );
    for expected in [
        "missing the expected frontmatter metadata afad entry",
        "missing the expected frontmatter metadata domain entry",
        "missing the expected frontmatter route section",
        "missing the expected frontmatter route keywords entry",
        "missing the expected frontmatter route questions entry",
    ] {
        assert!(
            frontmatter_errors
                .iter()
                .any(|error| error.contains(expected)),
            "missing frontmatter error containing {expected:?}: {frontmatter_errors:#?}"
        );
    }

    let html_comment_errors = crate::docs::metadata_contract_errors_for_tests(
        "README.md",
        "<!--\nAFAD:\n  version: \"4.1.0\"\n  updated: \"2026-04-20\"\n-->\n",
        crate::docs::MetadataStyle::HtmlComment,
        &updated_pattern,
    );
    for expected in [
        "missing the expected HTML comment metadata afad entry",
        "missing the expected HTML comment metadata domain entry",
        "missing the expected HTML comment RETRIEVAL_HINTS section",
        "missing the expected HTML comment RETRIEVAL_HINTS keywords entry",
        "missing the expected HTML comment RETRIEVAL_HINTS questions entry",
    ] {
        assert!(
            html_comment_errors
                .iter()
                .any(|error| error.contains(expected)),
            "missing HTML comment error containing {expected:?}: {html_comment_errors:#?}"
        );
    }
}

#[test]
fn expected_metadata_style_maps_docs_to_frontmatter_and_root_docs_to_html_comments() {
    let repo_root = tempdir().expect("tempdir");
    let docs_path = repo_root.path().join("docs").join("guide.md");
    let readme_path = repo_root.path().join("README.md");

    assert_eq!(
        crate::docs::expected_metadata_style_for_tests(repo_root.path(), &docs_path),
        crate::docs::MetadataStyle::Frontmatter
    );
    assert_eq!(
        crate::docs::expected_metadata_style_for_tests(repo_root.path(), &readme_path),
        crate::docs::MetadataStyle::HtmlComment
    );
}
