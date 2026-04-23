use super::*;

#[test]
fn repo_toolchain_from_manifest_extracts_channel_and_components() {
    let toolchain = repo_toolchain_from_manifest(
        "[toolchain]\nchannel = \"1.95.0\"\ncomponents = [\"clippy\", \"rustfmt\"]\n",
    )
    .expect("repo toolchain");

    assert_eq!(toolchain.channel, "1.95.0");
    assert_eq!(toolchain.components, vec!["clippy", "rustfmt"]);
}

#[test]
fn repo_toolchain_reads_from_repo_file_and_ignores_other_sections() {
    let repo_root = tempdir().expect("tempdir");
    fs::write(
        repo_root.path().join("rust-toolchain.toml"),
        "[workspace.package]\nversion = \"4.4.0\"\n\n[toolchain]\n# pinned here\nchannel = \"1.95.0\"\ncomponents = [\"clippy\", \"rustfmt\"]\n",
    )
    .expect("write rust-toolchain.toml");

    let toolchain = repo_toolchain(repo_root.path()).expect("repo toolchain");

    assert_eq!(toolchain.channel, "1.95.0");
    assert_eq!(toolchain.components, vec!["clippy", "rustfmt"]);
}

#[test]
fn repo_toolchain_from_manifest_requires_channel_and_components() {
    let missing_channel = repo_toolchain_from_manifest("[toolchain]\ncomponents = [\"clippy\"]\n")
        .expect_err("missing channel should fail");
    let missing_components = repo_toolchain_from_manifest("[toolchain]\nchannel = \"1.95.0\"\n")
        .expect_err("missing components should fail");

    assert_eq!(
        missing_channel.to_string(),
        "toolchain channel not found in rust-toolchain.toml"
    );
    assert_eq!(
        missing_components.to_string(),
        "toolchain components not found in rust-toolchain.toml"
    );
}

#[test]
fn repo_toolchain_from_manifest_rejects_malformed_component_arrays() {
    let error = repo_toolchain_from_manifest(
        "[toolchain]\nchannel = \"1.95.0\"\ncomponents = [clippy, \"rustfmt\"]\n",
    )
    .expect_err("malformed components should fail");

    assert_eq!(
        error.to_string(),
        "toolchain components not found in rust-toolchain.toml"
    );
}

#[test]
fn repo_toolchain_from_manifest_skips_malformed_headers_before_valid_toolchain_data() {
    let toolchain = repo_toolchain_from_manifest(
        "[toolchain\nchannel = \"0.0.0\"\ncomponents = [\"broken\"]\n[toolchain]\nchannel = \"1.95.0\"\ncomponents = [\"clippy\", \"rustfmt\"]\n",
    )
    .expect("repo toolchain");

    assert_eq!(toolchain.channel, "1.95.0");
    assert_eq!(toolchain.components, vec!["clippy", "rustfmt"]);
}

#[test]
fn repo_toolchain_preflight_requires_the_pinned_toolchain_first() {
    let toolchain = RepoToolchain {
        channel: "1.95.0".to_owned(),
        components: vec!["clippy".to_owned(), "rustfmt".to_owned()],
    };
    let failures =
        repo_toolchain_preflight_failures("stable-x86_64-apple-darwin (default)\n", "", &toolchain);

    assert_eq!(
        failures,
        vec![RepoToolchainPreflightFailure::MissingToolchain]
    );
    assert!(
        repo_toolchain_preflight_message(&failures, &toolchain)
            .contains("rustup toolchain install 1.95.0 --profile minimal")
    );
}

#[test]
fn repo_toolchain_preflight_requires_missing_components() {
    let toolchain = RepoToolchain {
        channel: "1.95.0".to_owned(),
        components: vec!["clippy".to_owned(), "rustfmt".to_owned()],
    };
    let failures = repo_toolchain_preflight_failures(
        "1.95.0-aarch64-apple-darwin (default)\n",
        "clippy-aarch64-apple-darwin\n",
        &toolchain,
    );

    assert_eq!(
        failures,
        vec![RepoToolchainPreflightFailure::MissingComponent(
            "rustfmt".to_owned()
        )]
    );
    assert!(
        repo_toolchain_preflight_message(&failures, &toolchain)
            .contains("rustup component add rustfmt --toolchain 1.95.0")
    );
}

#[test]
fn repo_toolchain_component_probe_commands_stay_in_sync_with_known_tools() {
    let toolchain = RepoToolchain {
        channel: "1.95.0".to_owned(),
        components: vec!["clippy".to_owned(), "rustfmt".to_owned()],
    };
    let clippy_probe =
        repo_toolchain_component_probe_command(&toolchain, "clippy").expect("clippy probe");
    let rustfmt_probe =
        repo_toolchain_component_probe_command(&toolchain, "rustfmt").expect("rustfmt probe");

    assert_eq!(clippy_probe.program, PathBuf::from("rustup"));
    assert_eq!(
        clippy_probe.args,
        vec!["run", "1.95.0", "cargo-clippy", "-V"]
    );
    assert_eq!(rustfmt_probe.program, PathBuf::from("rustup"));
    assert_eq!(
        rustfmt_probe.args,
        vec!["run", "1.95.0", "rustfmt", "--version"]
    );
    assert!(repo_toolchain_component_probe_command(&toolchain, "rust-docs").is_none());
}

#[test]
fn repo_toolchain_preflight_message_reports_broken_component_binaries() {
    let toolchain = RepoToolchain {
        channel: "1.95.0".to_owned(),
        components: vec!["clippy".to_owned(), "rustfmt".to_owned()],
    };
    let failures = vec![RepoToolchainPreflightFailure::BrokenComponentBinary(
        "clippy".to_owned(),
    )];

    let message = repo_toolchain_preflight_message(&failures, &toolchain);

    assert!(message.contains("component binaries are present in rustup metadata"));
    assert!(message.contains("rustup toolchain uninstall 1.95.0"));
    assert!(message.contains("rustup component add clippy rustfmt --toolchain 1.95.0"));
}

#[test]
fn repo_toolchain_preflight_passes_when_the_pinned_toolchain_is_ready() {
    let toolchain = RepoToolchain {
        channel: "1.95.0".to_owned(),
        components: vec!["clippy".to_owned(), "rustfmt".to_owned()],
    };
    let failures = repo_toolchain_preflight_failures(
        "stable-x86_64-apple-darwin\n1.95.0-aarch64-apple-darwin (default)\n",
        "clippy-aarch64-apple-darwin\nrustfmt-aarch64-apple-darwin\n",
        &toolchain,
    );

    assert!(failures.is_empty());
    let message = repo_toolchain_preflight_message(&failures, &toolchain);
    assert!(message.contains("Rust toolchain preflight failed."));
    assert!(!message.contains("rustup toolchain install"));
    assert!(!message.contains("rustup component add"));
}

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

    let expected_tracked_paths = [
        "crates/htmlcut-core/src/catalog.rs",
        "crates/htmlcut-core/src/contracts/mod.rs",
        "crates/htmlcut-cli/src/execute.rs",
        "crates/htmlcut-cli/src/execute/commands.rs",
        "xtask/src/plan.rs",
    ];

    assert_eq!(tracked.len(), expected_tracked_paths.len());
    for relative_path in expected_tracked_paths {
        let absolute_path =
            normalize_path(repo_root.path(), &repo_root.path().join(relative_path)).expect("path");
        assert_eq!(
            tracked.get(&absolute_path),
            Some(&relative_path.to_string())
        );
    }

    for relative_path in COVERAGE_EXCLUDED_RELATIVE_PATHS {
        let absolute_path =
            normalize_path(repo_root.path(), &repo_root.path().join(relative_path)).expect("path");
        assert!(!tracked.contains_key(&absolute_path));
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
