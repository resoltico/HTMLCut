use super::*;

#[test]
fn markdown_contract_reports_release_doc_drift_against_canonical_release_targets() {
    let repo_root = tempdir().expect("tempdir");
    fs::write(
        repo_root.path().join("Cargo.toml"),
        "[workspace.package]\nversion = \"4.1.0\"\n",
    )
    .expect("write manifest");
    fs::write(
        repo_root.path().join("README.md"),
        "<!--\nAFAD:\n  afad: \"3.5\"\n  version: \"4.1.0\"\n  domain: README\n  updated: \"2026-04-22\"\nRETRIEVAL_HINTS:\n  keywords: [readme, install, release assets, target matrix, source install]\n  questions: [what release assets exist?, how do I install HTMLCut?, which binaries ship?]\n-->\n\n# README\n\n- `htmlcut-source-X.Y.Z.zip`\n",
    )
    .expect("write readme");
    fs::write(
        repo_root.path().join("CONTRIBUTING.md"),
        "<!--\nAFAD:\n  afad: \"3.5\"\n  version: \"4.1.0\"\n  domain: MAINTAINER\n  updated: \"2026-04-22\"\nRETRIEVAL_HINTS:\n  keywords: [contributing, workflow, checks, release, docs]\n  questions: [how do I contribute?, which checks run?]\n-->\n\n# Contributing\n",
    )
    .expect("write contributing");
    write_minimal_docs_legal_scaffold(repo_root.path(), "4.1.0", "2026-04-22");
    fs::create_dir_all(repo_root.path().join("fuzz")).expect("create fuzz dir");
    fs::write(
        repo_root.path().join("fuzz").join("README.md"),
        "<!--\nAFAD:\n  afad: \"3.5\"\n  version: \"4.1.0\"\n  domain: QUALITY\n  updated: \"2026-04-22\"\nRETRIEVAL_HINTS:\n  keywords: [fuzz, corpus, targets, libfuzzer, seeds]\n  questions: [how do I run fuzzing?, which fuzz targets exist?]\n-->\n\n# Fuzz\n",
    )
    .expect("write fuzz readme");
    fs::create_dir_all(repo_root.path().join("docs")).expect("create docs dir");
    fs::write(
        repo_root.path().join("docs").join("platform-support.md"),
        "---\nafad: \"3.5\"\nversion: \"4.1.0\"\ndomain: PLATFORM\nupdated: \"2026-04-22\"\nroute:\n  keywords: [platform support, target matrix, release assets, runners, deployment floors]\n  questions: [which platforms ship?, which runners build the releases?]\n---\n\n# Platform Support\n\n- `aarch64-apple-darwin`\n- `x86_64-pc-windows-msvc`\n- `htmlcut-source-X.Y.Z.zip`\n- `macos-15` for `aarch64-apple-darwin`\n",
    )
    .expect("write platform support");
    fs::create_dir_all(repo_root.path().join("scripts")).expect("create scripts dir");
    fs::write(
        repo_root.path().join("scripts").join("release-targets.sh"),
        r#"#!/usr/bin/env bash
release_target_triples() {
    cat <<'EOF'
aarch64-apple-darwin
x86_64-pc-windows-msvc
EOF
}

release_matrix_json() {
    cat <<'EOF'
{"include":[{"id":"macos-arm64","runs_on":"macos-15","target_triple":"aarch64-apple-darwin","artifact_bundle_name":"standalone-macos-arm64","needs_musl_tools":false},{"id":"windows-x64","runs_on":"windows-2022","target_triple":"x86_64-pc-windows-msvc","artifact_bundle_name":"standalone-windows-x64","needs_musl_tools":false}]}
EOF
}

release_asset_names_for_version() {
    local release_version="$1"
    printf 'htmlcut-source-%s.zip\n' "${release_version}"
    printf 'htmlcut-%s-x86_64-pc-windows-msvc.zip\n' "${release_version}"
    printf 'htmlcut-%s-checksums.txt\n' "${release_version}"
}

macos_deployment_target_for_target() {
    local requested_target="$1"
    case "${requested_target}" in
        aarch64-apple-darwin) printf '12.0\n' ;;
        *) printf '\n' ;;
    esac
}
"#,
    )
    .expect("write release-targets.sh");

    let errors = markdown_contract_errors(repo_root.path()).expect("markdown contract errors");

    assert!(errors.iter().any(|error| error.contains(
        "README.md is missing release asset name: htmlcut-X.Y.Z-x86_64-pc-windows-msvc.zip"
    )));
    assert!(errors.iter().any(|error| {
        error.contains("README.md is missing release asset name: htmlcut-X.Y.Z-checksums.txt")
    }));
    assert!(errors.iter().any(|error| error.contains("docs/platform-support.md is missing release asset name: htmlcut-X.Y.Z-x86_64-pc-windows-msvc.zip")));
    assert!(errors.iter().any(|error| error.contains("docs/platform-support.md is missing the release runner mapping - `windows-2022` for `x86_64-pc-windows-msvc`")));
    assert!(errors.iter().any(|error| error.contains(
        "docs/platform-support.md is missing the canonical macOS deployment floor macOS 12.0"
    )));
}
