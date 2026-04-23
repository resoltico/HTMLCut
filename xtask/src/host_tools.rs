use crate::model::CommandSpec;

/// One actionable prerequisite that a host-tool-dependent maintainer flow checks before launch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostToolPreflightFailure {
    /// One required host tool is not available on `PATH`.
    MissingTool(String),
}

/// Builds the direct `--version` probe used to detect one required host tool.
pub fn host_tool_probe_command(tool: &str) -> CommandSpec {
    CommandSpec::new(tool, ["--version"], true, false)
}

/// Returns missing host tools for one maintained command.
pub fn host_tool_preflight_failures<F>(
    required_tools: &[&str],
    mut is_installed: F,
) -> Vec<HostToolPreflightFailure>
where
    F: FnMut(&str) -> bool,
{
    required_tools
        .iter()
        .copied()
        .filter(|tool| !is_installed(tool))
        .map(|tool| HostToolPreflightFailure::MissingTool(tool.to_owned()))
        .collect()
}

/// Formats the actionable preflight error shown before a clang-forced flow starts.
pub fn host_tool_preflight_message(context: &str, failures: &[HostToolPreflightFailure]) -> String {
    let missing_tools = failures
        .iter()
        .map(|failure| match failure {
            HostToolPreflightFailure::MissingTool(tool) => tool.as_str(),
        })
        .collect::<Vec<_>>();

    format!(
        "{context} preflight failed. This maintained command runs Cargo with `CC=clang CXX=clang++`, so the following host tools must be available on `PATH`: {}.\n\nInstall or repair a working LLVM/clang toolchain for this host, then rerun the command.",
        missing_tools.join(", ")
    )
}
