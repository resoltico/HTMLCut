use super::*;

#[test]
fn miri_commands_use_the_managed_workspace_layout() {
    let probe = miri_probe_command();
    let test = miri_selector_command();

    assert_eq!(probe.program, PathBuf::from("cargo"));
    assert_eq!(probe.args, vec!["+nightly", "miri", "--version"]);
    assert!(command_is_quiet(&probe));
    assert!(!command_forces_clang(&probe));
    assert!(command_uses_managed_workspace_artifacts(&probe));

    assert_eq!(test.program, PathBuf::from("cargo"));
    assert_eq!(
        test.args,
        vec![
            "+nightly",
            "miri",
            "test",
            "-p",
            "htmlcut-core",
            "--lib",
            "--no-default-features",
            "--locked",
            "tests::extract_api::selector_contract_remains_miri_sound",
            "--",
            "--exact",
        ]
    );
    assert!(!command_is_quiet(&test));
    assert!(!command_forces_clang(&test));
    assert!(command_uses_managed_workspace_artifacts(&test));
    assert_eq!(
        test.env.get("MIRIFLAGS").map(String::as_str),
        Some("-Zmiri-strict-provenance")
    );
}

#[test]
fn miri_preflight_reports_missing_toolchain_components_and_binary() {
    assert_eq!(
        miri_preflight_failures("", "", false),
        vec![
            MiriPreflightFailure::MissingNightlyToolchain,
            MiriPreflightFailure::MissingNightlyMiri,
            MiriPreflightFailure::MissingNightlyRustSrc,
        ]
    );
    assert_eq!(
        miri_preflight_failures("nightly-aarch64-apple-darwin\n", "", false),
        vec![
            MiriPreflightFailure::MissingNightlyMiri,
            MiriPreflightFailure::MissingNightlyRustSrc,
        ]
    );
    assert_eq!(
        miri_preflight_failures(
            "nightly-aarch64-apple-darwin\n",
            "miri-aarch64-apple-darwin (installed)\nrust-src (installed)\n",
            false,
        ),
        vec![MiriPreflightFailure::BrokenNightlyMiriBinary]
    );
    assert!(
        miri_preflight_failures(
            "nightly-aarch64-apple-darwin\n",
            "miri-aarch64-apple-darwin (installed)\nrust-src (installed)\n",
            true,
        )
        .is_empty()
    );
}

#[test]
fn miri_preflight_message_is_actionable() {
    let missing_nightly = miri_preflight_message(&[
        MiriPreflightFailure::MissingNightlyToolchain,
        MiriPreflightFailure::MissingNightlyMiri,
        MiriPreflightFailure::MissingNightlyRustSrc,
    ]);
    assert!(missing_nightly.contains("selector-safety proof"));
    assert!(missing_nightly.contains(
        "rustup toolchain install nightly --profile minimal --component miri --component rust-src"
    ));

    let missing_components = miri_preflight_message(&[
        MiriPreflightFailure::MissingNightlyMiri,
        MiriPreflightFailure::MissingNightlyRustSrc,
    ]);
    assert!(missing_components.contains("rustup component add miri rust-src --toolchain nightly"));
    assert!(!missing_components.contains("toolchain uninstall nightly"));

    let broken_binary = miri_preflight_message(&[MiriPreflightFailure::BrokenNightlyMiriBinary]);
    assert!(broken_binary.contains("cargo +nightly miri --version"));
    assert!(broken_binary.contains("rustup toolchain uninstall nightly"));
}
