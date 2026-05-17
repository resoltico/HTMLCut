use std::fs;
use std::path::Path;

use serde::Deserialize;

use crate::model::{
    CommandSpec, CommandStderr, CommandStdout, CommandToolchainEnv, DynResult, XtaskError,
};

#[derive(Debug, Deserialize)]
struct ToolchainManifest {
    toolchain: ToolchainSection,
}

#[derive(Debug, Deserialize)]
struct ToolchainSection {
    #[serde(default)]
    channel: String,
    #[serde(default)]
    components: Vec<String>,
}

/// The repository-owned stable toolchain contract from `rust-toolchain.toml`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoToolchain {
    /// Exact channel string pinned for normal repository work.
    pub channel: String,
    /// Components that must be installed on that pinned toolchain.
    pub components: Vec<String>,
}

/// One actionable prerequisite that `cargo xtask check` validates up front.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RepoToolchainPreflightFailure {
    /// The pinned toolchain itself is not installed.
    MissingToolchain,
    /// One required component from `rust-toolchain.toml` is missing.
    MissingComponent(String),
    /// The component is listed as installed, but its binary still does not run.
    BrokenComponentBinary(String),
}

/// Reads the repository-owned toolchain contract from `rust-toolchain.toml`.
pub fn repo_toolchain(repo_root: &Path) -> DynResult<RepoToolchain> {
    repo_toolchain_from_manifest(&fs::read_to_string(repo_root.join("rust-toolchain.toml"))?)
}

/// Extracts the pinned toolchain channel and required components from `rust-toolchain.toml`.
pub fn repo_toolchain_from_manifest(manifest: &str) -> DynResult<RepoToolchain> {
    let parsed: ToolchainManifest = toml::from_str(manifest)
        .map_err(|source| XtaskError::invalid_toml("rust-toolchain.toml", source))?;
    if parsed.toolchain.channel.trim().is_empty() {
        return Err("toolchain channel not found in rust-toolchain.toml".into());
    }
    if parsed.toolchain.components.is_empty() {
        return Err("toolchain components not found in rust-toolchain.toml".into());
    }

    Ok(RepoToolchain {
        channel: parsed.toolchain.channel,
        components: parsed.toolchain.components,
    })
}

/// Returns missing prerequisites for the pinned stable toolchain gate.
pub fn repo_toolchain_preflight_failures(
    toolchain_installed: bool,
    installed_components_output: &str,
    toolchain: &RepoToolchain,
) -> Vec<RepoToolchainPreflightFailure> {
    if !toolchain_installed {
        return vec![RepoToolchainPreflightFailure::MissingToolchain];
    }

    toolchain
        .components
        .iter()
        .filter(|component| {
            !installed_component_present(installed_components_output, component.as_str())
        })
        .cloned()
        .map(RepoToolchainPreflightFailure::MissingComponent)
        .collect()
}

/// Builds the direct binary probe for one known pinned-toolchain component.
pub fn repo_toolchain_component_probe_command(
    toolchain: &RepoToolchain,
    component: &str,
) -> Option<CommandSpec> {
    let command = match component {
        "clippy" => CommandSpec::new(
            "rustup",
            ["run", toolchain.channel.as_str(), "cargo-clippy", "-V"],
            CommandStdout::Quiet,
            CommandToolchainEnv::Inherit,
        )
        .with_stderr(CommandStderr::Quiet),
        "rustfmt" => CommandSpec::new(
            "rustup",
            ["run", toolchain.channel.as_str(), "rustfmt", "--version"],
            CommandStdout::Quiet,
            CommandToolchainEnv::Inherit,
        )
        .with_stderr(CommandStderr::Quiet),
        _ => return None,
    };

    Some(command)
}

/// Builds the direct toolchain probe used to verify that rustup can run the pinned compiler.
pub fn repo_toolchain_probe_command(toolchain: &RepoToolchain) -> CommandSpec {
    CommandSpec::new(
        "rustup",
        ["run", toolchain.channel.as_str(), "rustc", "-Vv"],
        CommandStdout::Quiet,
        CommandToolchainEnv::Inherit,
    )
    .with_stderr(CommandStderr::Quiet)
}

/// Formats the actionable preflight error shown before the main Rust gate starts.
pub fn repo_toolchain_preflight_message(
    failures: &[RepoToolchainPreflightFailure],
    toolchain: &RepoToolchain,
) -> String {
    let missing_toolchain = failures.contains(&RepoToolchainPreflightFailure::MissingToolchain);
    let missing_components: Vec<&str> = failures
        .iter()
        .filter_map(|failure| match failure {
            RepoToolchainPreflightFailure::MissingComponent(component) => Some(component.as_str()),
            RepoToolchainPreflightFailure::MissingToolchain
            | RepoToolchainPreflightFailure::BrokenComponentBinary(_) => None,
        })
        .collect();
    let broken_components: Vec<&str> = failures
        .iter()
        .filter_map(|failure| match failure {
            RepoToolchainPreflightFailure::BrokenComponentBinary(component) => {
                Some(component.as_str())
            }
            RepoToolchainPreflightFailure::MissingToolchain
            | RepoToolchainPreflightFailure::MissingComponent(_) => None,
        })
        .collect();

    let mut message = format!(
        "Rust toolchain preflight failed. HTMLCut pins the day-to-day repository compiler exactly through `rust-toolchain.toml`, and `cargo xtask check` expects that pinned toolchain plus its required components before the gate starts.\n\nPinned toolchain: `{}`\n",
        toolchain.channel
    );

    if missing_toolchain {
        message.push_str(&format!(
            "\nInstall the pinned toolchain first:\n  rustup toolchain install {} --profile minimal\n",
            toolchain.channel
        ));
    }

    if !missing_components.is_empty() {
        message.push_str(&format!(
            "\nInstall the missing pinned-toolchain components:\n  rustup component add {} --toolchain {}\n",
            missing_components.join(" "),
            toolchain.channel
        ));
    }

    if !broken_components.is_empty() {
        message.push_str(&format!(
            "\nThe following pinned-toolchain component binaries are present in rustup metadata but still do not run: {}.\nRepair the pinned toolchain cleanly with:\n  rustup toolchain uninstall {}\n  rustup toolchain install {} --profile minimal\n  rustup component add {} --toolchain {}\n",
            broken_components.join(", "),
            toolchain.channel,
            toolchain.channel,
            toolchain.components.join(" "),
            toolchain.channel
        ));
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
