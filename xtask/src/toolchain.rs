use std::fs;
use std::path::Path;

use crate::model::{CommandSpec, DynResult};

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
    let channel = toolchain_string_field(manifest, "channel")
        .ok_or_else(|| "toolchain channel not found in rust-toolchain.toml".to_owned())?;
    let components = toolchain_array_field(manifest, "components")
        .ok_or_else(|| "toolchain components not found in rust-toolchain.toml".to_owned())?;

    Ok(RepoToolchain {
        channel,
        components,
    })
}

/// Returns missing prerequisites for the pinned stable toolchain gate.
pub fn repo_toolchain_preflight_failures(
    toolchains_output: &str,
    installed_components_output: &str,
    toolchain: &RepoToolchain,
) -> Vec<RepoToolchainPreflightFailure> {
    let has_toolchain = toolchains_output
        .lines()
        .map(str::trim)
        .any(|line| line.starts_with(toolchain.channel.as_str()));
    if !has_toolchain {
        return vec![RepoToolchainPreflightFailure::MissingToolchain];
    }

    toolchain
        .components
        .iter()
        .filter(|component| {
            !installed_components_output
                .lines()
                .map(str::trim)
                .any(|line| line.starts_with(component.as_str()))
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
            true,
            false,
        ),
        "rustfmt" => CommandSpec::new(
            "rustup",
            ["run", toolchain.channel.as_str(), "rustfmt", "--version"],
            true,
            false,
        ),
        _ => return None,
    };

    Some(command)
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

fn toolchain_string_field(manifest: &str, field_name: &str) -> Option<String> {
    let mut in_section = false;

    for line in manifest.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            in_section = trimmed == "[toolchain]";
            continue;
        }

        if !in_section {
            continue;
        }

        if let Some(value) = string_assignment_value(trimmed, field_name) {
            return Some(value);
        }
    }

    None
}

fn toolchain_array_field(manifest: &str, field_name: &str) -> Option<Vec<String>> {
    let mut in_section = false;

    for line in manifest.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            in_section = trimmed == "[toolchain]";
            continue;
        }

        if !in_section {
            continue;
        }

        if let Some(value) = array_assignment_values(trimmed, field_name) {
            return Some(value);
        }
    }

    None
}

fn string_assignment_value(line: &str, field_name: &str) -> Option<String> {
    let remainder = line.strip_prefix(field_name)?.trim_start();
    let remainder = remainder.strip_prefix('=')?.trim_start();
    let remainder = remainder.strip_prefix('"')?;
    let closing_quote = remainder.find('"')?;
    Some(remainder[..closing_quote].to_owned())
}

fn array_assignment_values(line: &str, field_name: &str) -> Option<Vec<String>> {
    let remainder = line.strip_prefix(field_name)?.trim_start();
    let remainder = remainder.strip_prefix('=')?.trim_start();
    let remainder = remainder.strip_prefix('[')?.strip_suffix(']')?;

    remainder
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| {
            value
                .strip_prefix('"')?
                .strip_suffix('"')
                .map(str::to_owned)
        })
        .collect::<Option<Vec<_>>>()
}
