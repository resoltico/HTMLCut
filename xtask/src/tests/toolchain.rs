use super::*;

#[test]
fn coverage_preflight_failures_require_nightly_toolchain_first() {
    let failures = coverage_preflight_failures("stable-x86_64-apple-darwin (default)\n", "");

    assert_eq!(
        failures,
        vec![
            CoveragePreflightFailure::MissingNightlyToolchain,
            CoveragePreflightFailure::MissingNightlyLlvmTools,
        ]
    );
    assert!(coverage_preflight_message(&failures).contains("rustup toolchain install nightly"));
}

#[test]
fn coverage_preflight_failures_require_llvm_tools_when_nightly_exists() {
    let failures = coverage_preflight_failures("nightly-x86_64-apple-darwin\n", "clippy\n");

    assert_eq!(
        failures,
        vec![CoveragePreflightFailure::MissingNightlyLlvmTools]
    );
    assert!(
        coverage_preflight_message(&failures)
            .contains("rustup component add llvm-tools-preview --toolchain nightly")
    );
}

#[test]
fn coverage_preflight_passes_when_nightly_and_llvm_tools_are_installed() {
    let failures = coverage_preflight_failures(
        "stable-x86_64-apple-darwin (default)\nnightly-x86_64-apple-darwin\n",
        "llvm-tools-x86_64-apple-darwin\nrustfmt\n",
    );

    assert!(failures.is_empty());
    let message = coverage_preflight_message(&failures);
    assert!(message.contains("Rust coverage preflight failed."));
    assert!(!message.contains("Install the nightly coverage toolchain first"));
    assert!(!message.contains("llvm-tools-preview` is missing"));
}

#[test]
fn tracked_files_canonicalize_the_expected_maintained_sources() {
    let repo_root = tempdir().expect("tempdir");
    seed_tracked_files(repo_root.path());

    let tracked = tracked_files(repo_root.path()).expect("tracked files");

    assert_eq!(tracked.len(), TRACKED_RELATIVE_PATHS.len());
    for relative_path in TRACKED_RELATIVE_PATHS {
        let absolute_path =
            normalize_path(repo_root.path(), &repo_root.path().join(relative_path)).expect("path");
        assert_eq!(
            tracked.get(&absolute_path),
            Some(&relative_path.to_string())
        );
    }
}

#[test]
fn normalize_path_supports_relative_and_absolute_inputs() {
    let repo_root = tempdir().expect("tempdir");
    let file_path = repo_root.path().join("scripts").join("lint.sh");
    fs::create_dir_all(file_path.parent().expect("parent")).expect("create dir");
    fs::write(&file_path, "#!/usr/bin/env bash\n").expect("write script");

    let from_relative =
        normalize_path(repo_root.path(), Path::new("scripts/lint.sh")).expect("relative");
    let from_absolute = normalize_path(repo_root.path(), &file_path).expect("absolute");

    assert_eq!(from_relative, from_absolute);
}
