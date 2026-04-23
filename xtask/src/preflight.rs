use std::path::Path;

use crate::command_exec::capture_command_output;
use crate::model::{CommandSpec, CoveragePreflightFailure, DynResult};
use crate::{
    FuzzSmokePreflightFailure, RepoToolchainPreflightFailure, cargo_fuzz_probe_command,
    coverage_preflight_failures, coverage_preflight_message, host_tool_preflight_failures,
    host_tool_preflight_message, host_tool_probe_command, repo_toolchain,
    repo_toolchain_component_probe_command, repo_toolchain_preflight_failures,
    repo_toolchain_preflight_message,
};

/// Validates the pinned repository toolchain before the main maintainer gate starts.
pub fn ensure_repo_toolchain_prerequisites(repo_root: &Path) -> DynResult<()> {
    let toolchain = repo_toolchain(repo_root).map_err(|error| {
        format!("toolchain preflight could not read rust-toolchain.toml: {error}")
    })?;
    let toolchains = capture_utf8(
        repo_root,
        &CommandSpec::new("rustup", ["toolchain", "list"], false, false),
        "toolchain preflight could not query rustup toolchains",
        "toolchain preflight received invalid rustup output",
    )?;
    let components = capture_utf8(
        repo_root,
        &CommandSpec::new(
            "rustup",
            [
                "component",
                "list",
                "--toolchain",
                toolchain.channel.as_str(),
                "--installed",
            ],
            false,
            false,
        ),
        format!(
            "toolchain preflight could not query `{}` components",
            toolchain.channel
        ),
        "toolchain preflight received invalid component output",
    )?;

    if let Some(message) =
        repo_toolchain_preflight_error(&toolchain, &toolchains, &components, |spec| {
            capture_command_output(repo_root, spec).is_ok()
        })
    {
        Err(message.into())
    } else {
        Ok(())
    }
}

/// Validates nightly plus LLVM prerequisites before the coverage gate starts.
pub fn ensure_coverage_prerequisites(repo_root: &Path) -> DynResult<()> {
    let toolchains = capture_utf8(
        repo_root,
        &CommandSpec::new("rustup", ["toolchain", "list"], false, false),
        "coverage preflight could not query rustup toolchains",
        "coverage preflight received invalid rustup output",
    )?;
    let components = capture_utf8(
        repo_root,
        &CommandSpec::new(
            "rustup",
            ["component", "list", "--toolchain", "nightly", "--installed"],
            false,
            false,
        ),
        "coverage preflight could not query nightly components",
        "coverage preflight received invalid component output",
    )?;

    if let Some(message) = coverage_preflight_error(&toolchains, &components, |tool| {
        capture_command_output(repo_root, &host_tool_probe_command(tool)).is_ok()
    }) {
        Err(message.into())
    } else {
        Ok(())
    }
}

/// Validates nightly, cargo-fuzz, and LLVM prerequisites before a fuzz smoke run starts.
pub fn ensure_fuzz_smoke_prerequisites(repo_root: &Path) -> DynResult<()> {
    let toolchains = capture_utf8(
        repo_root,
        &CommandSpec::new("rustup", ["toolchain", "list"], false, false),
        "fuzz-smoke preflight could not query rustup toolchains",
        "fuzz-smoke preflight received invalid rustup output",
    )?;
    let cargo_fuzz_installed =
        capture_command_output(repo_root, &cargo_fuzz_probe_command()).is_ok();

    if let Some(message) = fuzz_smoke_preflight_error(&toolchains, cargo_fuzz_installed, |tool| {
        capture_command_output(repo_root, &host_tool_probe_command(tool)).is_ok()
    }) {
        Err(message.into())
    } else {
        Ok(())
    }
}

fn capture_utf8(
    repo_root: &Path,
    spec: &CommandSpec,
    command_error: impl Into<String>,
    decode_error: &str,
) -> DynResult<String> {
    let output = capture_command_output(repo_root, spec)
        .map_err(|error| format!("{}: {error}", command_error.into()))?;
    String::from_utf8(output).map_err(|error| format!("{decode_error}: {error}").into())
}

fn repo_toolchain_preflight_error<F>(
    toolchain: &crate::RepoToolchain,
    toolchains_output: &str,
    installed_components_output: &str,
    mut command_succeeds: F,
) -> Option<String>
where
    F: FnMut(&CommandSpec) -> bool,
{
    let toolchain_failures = repo_toolchain_preflight_failures(toolchains_output, "", toolchain);
    if toolchain_failures.contains(&RepoToolchainPreflightFailure::MissingToolchain) {
        return Some(repo_toolchain_preflight_message(
            &toolchain_failures,
            toolchain,
        ));
    }

    let failures = repo_toolchain_preflight_failures(
        toolchains_output,
        installed_components_output,
        toolchain,
    );
    if !failures.is_empty() {
        return Some(repo_toolchain_preflight_message(&failures, toolchain));
    }

    let broken_binaries = toolchain
        .components
        .iter()
        .filter_map(|component| {
            let spec = repo_toolchain_component_probe_command(toolchain, component)?;
            (!command_succeeds(&spec))
                .then(|| RepoToolchainPreflightFailure::BrokenComponentBinary(component.clone()))
        })
        .collect::<Vec<_>>();

    (!broken_binaries.is_empty())
        .then(|| repo_toolchain_preflight_message(&broken_binaries, toolchain))
}

fn coverage_preflight_error<F>(
    toolchains_output: &str,
    installed_components_output: &str,
    has_host_tool: F,
) -> Option<String>
where
    F: FnMut(&str) -> bool,
{
    let toolchain_failures = coverage_preflight_failures(toolchains_output, "");
    if toolchain_failures.contains(&CoveragePreflightFailure::MissingNightlyToolchain) {
        return Some(coverage_preflight_message(&toolchain_failures));
    }

    let failures = coverage_preflight_failures(toolchains_output, installed_components_output);
    if !failures.is_empty() {
        return Some(coverage_preflight_message(&failures));
    }

    clang_toolchain_preflight_error("coverage", has_host_tool)
}

fn fuzz_smoke_preflight_error<F>(
    toolchains_output: &str,
    cargo_fuzz_installed: bool,
    has_host_tool: F,
) -> Option<String>
where
    F: FnMut(&str) -> bool,
{
    let failures = crate::fuzz_smoke_preflight_failures(toolchains_output, cargo_fuzz_installed);
    if failures.contains(&FuzzSmokePreflightFailure::MissingNightlyToolchain)
        || failures.contains(&FuzzSmokePreflightFailure::MissingCargoFuzz)
    {
        return Some(crate::fuzz_smoke_preflight_message(&failures));
    }

    clang_toolchain_preflight_error("fuzz-smoke", has_host_tool)
}

fn clang_toolchain_preflight_error<F>(context: &str, has_host_tool: F) -> Option<String>
where
    F: FnMut(&str) -> bool,
{
    let failures = host_tool_preflight_failures(&["clang", "clang++"], has_host_tool);
    (!failures.is_empty()).then(|| host_tool_preflight_message(context, &failures))
}

#[cfg(test)]
pub(crate) fn repo_toolchain_preflight_error_for_tests<F>(
    toolchain: &crate::RepoToolchain,
    toolchains_output: &str,
    installed_components_output: &str,
    command_succeeds: F,
) -> Option<String>
where
    F: FnMut(&CommandSpec) -> bool,
{
    repo_toolchain_preflight_error(
        toolchain,
        toolchains_output,
        installed_components_output,
        command_succeeds,
    )
}

#[cfg(test)]
pub(crate) fn coverage_preflight_error_for_tests<F>(
    toolchains_output: &str,
    installed_components_output: &str,
    has_host_tool: F,
) -> Option<String>
where
    F: FnMut(&str) -> bool,
{
    coverage_preflight_error(
        toolchains_output,
        installed_components_output,
        has_host_tool,
    )
}

#[cfg(test)]
pub(crate) fn fuzz_smoke_preflight_error_for_tests<F>(
    toolchains_output: &str,
    cargo_fuzz_installed: bool,
    has_host_tool: F,
) -> Option<String>
where
    F: FnMut(&str) -> bool,
{
    fuzz_smoke_preflight_error(toolchains_output, cargo_fuzz_installed, has_host_tool)
}

#[cfg(test)]
pub(crate) fn clang_toolchain_preflight_error_for_tests<F>(
    context: &str,
    has_host_tool: F,
) -> Option<String>
where
    F: FnMut(&str) -> bool,
{
    clang_toolchain_preflight_error(context, has_host_tool)
}
