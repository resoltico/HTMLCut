use std::fs;
use std::path::Path;

use crate::model::{CommandSpec, DynResult};
use crate::release_target_triples;

/// Builds the strict dependency-policy command used by the maintainer gate.
pub fn deny_check_command(repo_root: &Path) -> DynResult<CommandSpec> {
    let mut args = vec!["deny".to_owned()];

    for target in release_target_triples(repo_root)? {
        args.push("--target".to_owned());
        args.push(target);
    }

    args.extend(
        [
            "check",
            "-D",
            "warnings",
            "advisories",
            "bans",
            "licenses",
            "sources",
        ]
        .into_iter()
        .map(str::to_owned),
    );

    Ok(CommandSpec::new("cargo", args, false, false))
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

fn parse_deny_graph_targets(policy: &str) -> Option<Vec<String>> {
    let mut in_graph_section = false;
    let mut collecting_targets = false;
    let mut targets = Vec::new();

    for raw_line in policy.lines() {
        let line = raw_line.trim();

        if line.starts_with('[') && line.ends_with(']') {
            in_graph_section = line == "[graph]";
            collecting_targets = false;
            continue;
        }

        if !in_graph_section {
            continue;
        }

        if !collecting_targets {
            if let Some(rest) = line.strip_prefix("targets = [") {
                collecting_targets = true;
                collect_quoted_values(rest, &mut targets);
                if rest.contains(']') {
                    return Some(targets);
                }
            }
            continue;
        }

        collect_quoted_values(line, &mut targets);
        if line.contains(']') {
            return Some(targets);
        }
    }

    None
}

fn collect_quoted_values(line: &str, values: &mut Vec<String>) {
    let mut cursor = line;
    while let Some(start) = cursor.find('"') {
        let after_start = &cursor[start + 1..];
        let Some(end) = after_start.find('"') else {
            break;
        };
        values.push(after_start[..end].to_owned());
        cursor = &after_start[end + 1..];
    }
}

#[cfg(test)]
pub(crate) fn parse_deny_graph_targets_for_tests(policy: &str) -> Option<Vec<String>> {
    parse_deny_graph_targets(policy)
}
