use super::*;

#[test]
fn markdown_contract_errors_report_patents_license_family_drift() {
    let repo_root = tempdir().expect("tempdir");
    fs::write(
        repo_root.path().join("Cargo.toml"),
        "[workspace.package]\nversion = \"4.4.0\"\n",
    )
    .expect("write Cargo.toml");
    fs::write(
        repo_root.path().join("deny.toml"),
        r#"[licenses]
allow = [
    "MIT",
    "Apache-2.0",
    "NCSA",
]
"#,
    )
    .expect("write deny.toml");
    fs::write(
        repo_root.path().join("PATENTS.md"),
        "<!--\nAFAD:\n  afad: \"3.5\"\n  version: \"4.4.0\"\n  domain: LEGAL\n  updated: \"2026-04-23\"\nRETRIEVAL_HINTS:\n  keywords: [patents, legal, licenses]\n  questions: [what is the patent posture?]\n  related: [README.md, NOTICE, deny.toml]\n-->\n\n# Patent Notes\n\n| License family | Explicit patent grant | Notes |\n|:---------------|:----------------------|:------|\n| MIT | No explicit grant | HTMLCut itself is MIT-licensed. |\n| Apache-2.0 | Yes | Section 3 grants patent rights from contributors to their contributions. |\n",
    )
    .expect("write patents");
    fs::create_dir_all(repo_root.path().join("scripts")).expect("create scripts dir");
    fs::write(
        repo_root.path().join("scripts").join("release-targets.sh"),
        r#"#!/usr/bin/env bash
release_target_triples() {
    :
}

release_matrix_json() {
    printf '{"include":[]}\n'
}

release_asset_names_for_version() {
    :
}

macos_deployment_target_for_target() {
    :
}
"#,
    )
    .expect("write release-targets.sh");

    let errors = markdown_contract_errors(repo_root.path()).expect("markdown contract errors");

    assert!(errors.iter().any(|error| {
        error.contains("PATENTS.md is missing allowed license family from deny.toml: NCSA")
    }));
}
