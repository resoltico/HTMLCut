use super::*;
#[cfg(unix)]
use std::ffi::OsString;
#[cfg(unix)]
use std::os::unix::ffi::OsStringExt;

#[test]
fn markdown_doc_paths_walk_repo_recursively_but_skip_internal_and_generated_dirs() {
    let repo_root = tempdir().expect("tempdir");
    fs::write(
        repo_root.path().join("Cargo.toml"),
        "[workspace.package]\nversion = \"4.1.0\"\n",
    )
    .expect("write manifest");
    fs::write(
        repo_root.path().join("README.md"),
        "<!-- version: \"4.1.0\" -->\n",
    )
    .expect("write readme");
    fs::write(
        repo_root.path().join("CONTRIBUTING.md"),
        "<!-- version: \"4.1.0\" -->\n",
    )
    .expect("write contributing");
    fs::write(
        repo_root.path().join("PATENTS.md"),
        "<!-- version: \"4.1.0\" -->\n",
    )
    .expect("write patents");
    fs::create_dir_all(repo_root.path().join("docs").join("nested")).expect("create docs dir");
    fs::write(
        repo_root
            .path()
            .join("docs")
            .join("nested")
            .join("guide.md"),
        "---\nversion: \"4.1.0\"\n---\n",
    )
    .expect("write nested guide");
    fs::create_dir_all(repo_root.path().join("fuzz")).expect("create fuzz dir");
    fs::write(
        repo_root.path().join("fuzz").join("README.md"),
        "<!-- version: \"4.1.0\" -->\n",
    )
    .expect("write fuzz readme");
    fs::create_dir_all(repo_root.path().join("examples")).expect("create examples dir");
    fs::write(
        repo_root.path().join("examples").join("guide.md"),
        "# example guide\n",
    )
    .expect("write example guide");
    fs::create_dir_all(repo_root.path().join("tmp")).expect("create tmp dir");
    fs::write(repo_root.path().join("tmp").join("notes.md"), "# ignore\n").expect("write tmp");
    fs::create_dir_all(repo_root.path().join(".codex")).expect("create .codex dir");
    fs::write(
        repo_root.path().join(".codex").join("AGENTS.md"),
        "# ignore\n",
    )
    .expect("write agents");
    fs::create_dir_all(repo_root.path().join("target")).expect("create target dir");
    fs::write(
        repo_root.path().join("target").join("report.md"),
        "# ignore\n",
    )
    .expect("write target");
    fs::create_dir_all(repo_root.path().join("semver-baseline")).expect("create baseline dir");
    fs::write(
        repo_root.path().join("semver-baseline").join("README.md"),
        "# ignore\n",
    )
    .expect("write baseline");
    fs::write(repo_root.path().join("CHANGELOG.md"), "# ignored\n").expect("write changelog");

    let docs = markdown_doc_paths(repo_root.path()).expect("markdown doc paths");

    assert_eq!(
        docs,
        vec![
            repo_root.path().join("CONTRIBUTING.md"),
            repo_root.path().join("PATENTS.md"),
            repo_root.path().join("README.md"),
            repo_root
                .path()
                .join("docs")
                .join("nested")
                .join("guide.md"),
            repo_root.path().join("examples").join("guide.md"),
            repo_root.path().join("fuzz").join("README.md"),
        ]
    );
}

#[test]
fn markdown_contract_errors_report_absolute_and_missing_links() {
    let repo_root = tempdir().expect("tempdir");
    write_empty_release_targets_script(repo_root.path());
    fs::write(
        repo_root.path().join("Cargo.toml"),
        "[workspace.package]\nversion = \"4.1.0\"\n",
    )
    .expect("write manifest");
    fs::write(
        repo_root.path().join("README.md"),
        "<!--\nAFAD:\n  afad: \"3.5\"\n  version: \"4.1.0\"\n  domain: PRODUCT\n  updated: \"2026-04-20\"\nRETRIEVAL_HINTS:\n  keywords: [htmlcut]\n  questions: [\"q\"]\n-->\n[bad](/tmp/nope)\n",
    )
    .expect("write readme");
    fs::write(
        repo_root.path().join("CONTRIBUTING.md"),
        "<!--\nAFAD:\n  afad: \"3.5\"\n  version: \"4.1.0\"\n  domain: MAINTAINER\n  updated: \"2026-04-20\"\nRETRIEVAL_HINTS:\n  keywords: [contrib]\n  questions: [\"q\"]\n-->\n[missing](./missing.md)\n",
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
        "---\nafad: \"3.5\"\nversion: \"4.1.0\"\ndomain: DOCS\nupdated: \"2026-04-20\"\nroute:\n  keywords: [guide]\n  questions: [\"q\"]\n---\n",
    )
    .expect("write guide");

    let errors = markdown_contract_errors(repo_root.path()).expect("markdown contract errors");

    assert!(
        errors
            .iter()
            .any(|error| error.contains("absolute local link target"))
    );
    assert!(
        errors
            .iter()
            .any(|error| error.contains("links to missing local path"))
    );
}

#[test]
fn markdown_contract_errors_ignore_external_anchor_and_mail_links() {
    let repo_root = tempdir().expect("tempdir");
    write_empty_release_targets_script(repo_root.path());
    fs::write(
        repo_root.path().join("Cargo.toml"),
        "[workspace.package]\nversion = \"4.1.0\"\n",
    )
    .expect("write manifest");
    fs::write(
        repo_root.path().join("README.md"),
        "<!--\nAFAD:\n  afad: \"3.5\"\n  version: \"4.1.0\"\n  domain: PRODUCT\n  updated: \"2026-04-20\"\nRETRIEVAL_HINTS:\n  keywords: [htmlcut]\n  questions: [\"q\"]\n-->\n[site](https://example.com)\n[mail](mailto:test@example.com)\n[section](#intro)\n",
    )
    .expect("write readme");
    fs::write(
        repo_root.path().join("CONTRIBUTING.md"),
        "<!--\nAFAD:\n  afad: \"3.5\"\n  version: \"4.1.0\"\n  domain: MAINTAINER\n  updated: \"2026-04-20\"\nRETRIEVAL_HINTS:\n  keywords: [contrib]\n  questions: [\"q\"]\n-->\n[repo](http://example.com)\n",
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
        "<!--\nAFAD:\n  afad: \"3.5\"\n  version: \"4.1.0\"\n  domain: QUALITY\n  updated: \"2026-04-20\"\nRETRIEVAL_HINTS:\n  keywords: [fuzz]\n  questions: [\"q\"]\n-->\n[wrapped](<https://example.com/docs>)\n",
    )
    .expect("write fuzz readme");
    fs::create_dir_all(repo_root.path().join("docs")).expect("create docs dir");
    fs::write(
        repo_root.path().join("docs").join("guide.md"),
        "---\nafad: \"3.5\"\nversion: \"4.1.0\"\ndomain: DOCS\nupdated: \"2026-04-20\"\nroute:\n  keywords: [guide]\n  questions: [\"q\"]\n---\n[top](../README.md#intro)\n",
    )
    .expect("write guide");

    let errors = markdown_contract_errors(repo_root.path()).expect("markdown contract errors");

    assert!(errors.is_empty(), "unexpected link errors: {errors:#?}");
}

#[test]
fn should_skip_dir_rejects_hidden_dirs_and_internal_generated_roots() {
    let repo_root = tempdir().expect("tempdir");
    let hidden = repo_root.path().join(".codex");
    let target = repo_root.path().join("target");
    let tmp = repo_root.path().join("tmp");

    assert!(crate::docs::should_skip_dir_for_tests(
        repo_root.path(),
        &hidden
    ));
    assert!(crate::docs::should_skip_dir_for_tests(
        repo_root.path(),
        &target
    ));
    assert!(crate::docs::should_skip_dir_for_tests(
        repo_root.path(),
        &tmp
    ));
}

#[cfg(unix)]
#[test]
fn should_skip_dir_rejects_non_utf8_directory_names() {
    let repo_root = tempdir().expect("tempdir");
    let non_utf8 = repo_root
        .path()
        .join(std::path::PathBuf::from(OsString::from_vec(vec![
            0x66, 0x80,
        ])));

    assert!(crate::docs::should_skip_dir_for_tests(
        repo_root.path(),
        &non_utf8
    ));
}
