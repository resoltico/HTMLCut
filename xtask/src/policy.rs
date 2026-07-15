use std::fs;
use std::path::Path;

use crate::model::{
    CommandArtifactLayout, CommandSpec, CommandStdout, CommandToolchainEnv, DynResult,
};
use crate::release_target_triples;

#[derive(serde::Deserialize)]
struct DenyPolicy {
    graph: DenyGraph,
}

#[derive(serde::Deserialize)]
struct DenyGraph {
    targets: Vec<String>,
}

/// Builds the strict dependency-policy command used by the maintainer gate.
pub fn deny_check_command(_repo_root: &Path) -> DynResult<CommandSpec> {
    let args: Vec<_> = [
        "deny",
        "check",
        "-D",
        "warnings",
        "advisories",
        "bans",
        "licenses",
        "sources",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect();

    Ok(CommandSpec::new(
        "cargo",
        args,
        CommandStdout::Inherit,
        CommandToolchainEnv::Inherit,
    )
    .with_artifact_layout(CommandArtifactLayout::ManagedWorkspace))
}

/// Reads the configured `cargo deny` graph targets from `deny.toml`.
pub fn deny_graph_targets(repo_root: &Path) -> DynResult<Vec<String>> {
    let policy_path = repo_root.join("deny.toml");
    let policy = fs::read_to_string(&policy_path)?;
    parse_deny_graph_targets(&policy).ok_or_else(|| {
        format!(
            "deny.toml is missing [graph] targets = [...] configuration: {}",
            policy_path.display()
        )
        .into()
    })
}

/// Ensures `cargo deny` evaluates exactly the release target registry.
pub fn ensure_deny_targets_match_release_targets(repo_root: &Path) -> DynResult<()> {
    let release_targets = release_target_triples(repo_root)?;
    let deny_targets = deny_graph_targets(repo_root)?;
    if deny_targets == release_targets {
        return Ok(());
    }

    Err(format!(
        "deny.toml graph targets do not match the canonical release target registry: deny={deny_targets:?}, release={release_targets:?}"
    )
    .into())
}

fn parse_deny_graph_targets(policy: &str) -> Option<Vec<String>> {
    toml::from_str::<DenyPolicy>(policy)
        .ok()
        .map(|policy| policy.graph.targets)
}

#[cfg(test)]
pub(crate) fn parse_deny_graph_targets_for_tests(policy: &str) -> Option<Vec<String>> {
    parse_deny_graph_targets(policy)
}
