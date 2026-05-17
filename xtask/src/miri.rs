use crate::model::{
    CommandArtifactLayout, CommandSpec, CommandStderr, CommandStdout, CommandToolchainEnv,
    MAINTAINED_NIGHTLY_TOOLCHAIN, MAINTAINED_NIGHTLY_TOOLCHAIN_NAME, MiriPreflightFailure,
};

pub(crate) const MIRI_SELECTOR_TEST_NAME: &str =
    "tests::extract_api::selector_contract_remains_miri_sound";

/// Builds the direct cargo-Miri probe used by the maintained preflight.
pub fn miri_probe_command() -> CommandSpec {
    CommandSpec::new(
        "cargo",
        [MAINTAINED_NIGHTLY_TOOLCHAIN, "miri", "--version"],
        CommandStdout::Quiet,
        CommandToolchainEnv::Inherit,
    )
    .with_artifact_layout(CommandArtifactLayout::ManagedWorkspace)
    .with_stderr(CommandStderr::Quiet)
}

/// Builds the maintained strict-provenance selector-safety Miri proof command.
pub fn miri_selector_command() -> CommandSpec {
    CommandSpec::new(
        "cargo",
        [
            MAINTAINED_NIGHTLY_TOOLCHAIN,
            "miri",
            "test",
            "-p",
            "htmlcut-core",
            "--lib",
            "--no-default-features",
            "--locked",
            MIRI_SELECTOR_TEST_NAME,
            "--",
            "--exact",
        ],
        CommandStdout::Inherit,
        CommandToolchainEnv::Inherit,
    )
    .with_artifact_layout(CommandArtifactLayout::ManagedWorkspace)
    .with_env("MIRIFLAGS", "-Zmiri-strict-provenance")
}

/// Returns missing prerequisites for the maintained strict-provenance selector-safety Miri proof.
pub fn miri_preflight_failures(
    toolchains_output: &str,
    installed_components_output: &str,
    miri_binary_runs: bool,
) -> Vec<MiriPreflightFailure> {
    let has_nightly_toolchain = toolchains_output
        .lines()
        .map(str::trim)
        .any(|line| line.starts_with(MAINTAINED_NIGHTLY_TOOLCHAIN_NAME));
    if !has_nightly_toolchain {
        return vec![
            MiriPreflightFailure::MissingNightlyToolchain,
            MiriPreflightFailure::MissingNightlyMiri,
            MiriPreflightFailure::MissingNightlyRustSrc,
        ];
    }

    let mut failures = Vec::new();
    if !installed_component_present(installed_components_output, "miri") {
        failures.push(MiriPreflightFailure::MissingNightlyMiri);
    }
    if !installed_component_present(installed_components_output, "rust-src") {
        failures.push(MiriPreflightFailure::MissingNightlyRustSrc);
    }
    if failures.is_empty() && !miri_binary_runs {
        failures.push(MiriPreflightFailure::BrokenNightlyMiriBinary);
    }

    failures
}

/// Formats the actionable preflight error shown before Miri work starts.
pub fn miri_preflight_message(failures: &[MiriPreflightFailure]) -> String {
    let missing_nightly = failures.contains(&MiriPreflightFailure::MissingNightlyToolchain);
    let missing_miri = failures.contains(&MiriPreflightFailure::MissingNightlyMiri);
    let missing_rust_src = failures.contains(&MiriPreflightFailure::MissingNightlyRustSrc);
    let broken_binary = failures.contains(&MiriPreflightFailure::BrokenNightlyMiriBinary);

    let mut message = String::from(
        "Rust Miri preflight failed. HTMLCut keeps stable as the default toolchain, but the maintained strict-provenance selector-safety proof runs through `cargo +nightly miri test`.\n",
    );

    if missing_nightly {
        message.push_str(
            "\nInstall the nightly Miri toolchain first:\n  rustup toolchain install nightly --profile minimal --component miri --component rust-src\n",
        );
        return message;
    }

    let mut missing_components = Vec::new();
    if missing_miri {
        missing_components.push("miri");
    }
    if missing_rust_src {
        missing_components.push("rust-src");
    }

    if !missing_components.is_empty() {
        message.push_str(&format!(
            "\nInstall the missing nightly Miri components:\n  rustup component add {} --toolchain nightly\n",
            missing_components.join(" ")
        ));
    }

    if broken_binary {
        message.push_str(
            "\nNightly reports the Miri components as installed, but `cargo +nightly miri --version` still does not run.\nRepair the nightly toolchain cleanly with:\n  rustup toolchain uninstall nightly\n  rustup toolchain install nightly --profile minimal --component miri --component rust-src\n",
        );
    }

    message
}

fn installed_component_present(output: &str, expected_component: &str) -> bool {
    output
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .filter_map(|line| line.split_whitespace().next())
        .any(|component| {
            component == expected_component
                || component
                    .strip_prefix(expected_component)
                    .is_some_and(|suffix| suffix.starts_with('-'))
        })
}
