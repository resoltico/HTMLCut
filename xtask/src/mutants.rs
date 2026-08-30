//! cargo-mutants command construction and prerequisite checks.

use std::path::{Path, PathBuf};

use crate::{
    CommandArtifactLayout, CommandSpec, CommandStderr, CommandStdout, CommandToolchainEnv,
};

/// One actionable prerequisite for the maintained mutation-testing workflow.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MutantsPreflightFailure {
    /// The `cargo-mutants` Cargo subcommand is unavailable or cannot run.
    MissingCargoMutants,
}

/// Returns missing prerequisites for a mutation-testing run.
pub fn mutants_preflight_failures(cargo_mutants_installed: bool) -> Vec<MutantsPreflightFailure> {
    (!cargo_mutants_installed)
        .then_some(MutantsPreflightFailure::MissingCargoMutants)
        .into_iter()
        .collect()
}

/// Formats the actionable preflight error shown before mutation testing starts.
pub fn mutants_preflight_message(failures: &[MutantsPreflightFailure]) -> String {
    let mut message = String::from(
        "Mutation-testing preflight failed. HTMLCut runs the pinned cargo-mutants tool through `cargo xtask mutants`.\n",
    );

    if failures.contains(&MutantsPreflightFailure::MissingCargoMutants) {
        message.push_str(
            "\nInstall the pinned mutation-testing tool with:\n  ./scripts/install-contributor-cargo-tools.sh cargo-mutants\n",
        );
    }

    message
}

/// Builds the direct Cargo-subcommand probe used by mutation-testing preflight.
pub fn cargo_mutants_probe_command() -> CommandSpec {
    CommandSpec::new(
        "cargo",
        ["mutants", "--version"],
        CommandStdout::Quiet,
        CommandToolchainEnv::Inherit,
    )
    .with_artifact_layout(CommandArtifactLayout::ManagedWorkspace)
    .with_stderr(CommandStderr::Quiet)
}

/// Returns the managed parent directory that contains one cargo-mutants result tree.
pub fn mutants_output_dir(repo_root: &Path) -> PathBuf {
    crate::mutation_report_dir(repo_root)
}

/// Builds the cargo-mutants command for the maintained HTMLCut configuration.
pub fn mutants_command(
    output_dir: &Path,
    in_place: bool,
    shard: Option<&str>,
    in_diff: Option<&Path>,
) -> CommandSpec {
    let mut args = vec![
        "mutants".to_owned(),
        "--output".to_owned(),
        output_dir.to_string_lossy().into_owned(),
    ];
    if in_place {
        args.push("--in-place".to_owned());
    }
    if let Some(shard) = shard {
        args.extend(["--shard".to_owned(), shard.to_owned()]);
    }
    if let Some(diff) = in_diff {
        args.extend(["--in-diff".to_owned(), diff.to_string_lossy().into_owned()]);
    }

    let command = CommandSpec::new(
        "cargo",
        args,
        CommandStdout::Inherit,
        CommandToolchainEnv::Inherit,
    )
    .with_live_output();
    if in_place {
        command.with_artifact_layout(CommandArtifactLayout::ManagedWorkspace)
    } else {
        command
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mutants_preflight_reports_the_missing_cargo_subcommand() {
        assert_eq!(
            mutants_preflight_failures(false),
            vec![MutantsPreflightFailure::MissingCargoMutants]
        );
        assert!(mutants_preflight_failures(true).is_empty());
        assert!(
            mutants_preflight_message(&[MutantsPreflightFailure::MissingCargoMutants])
                .contains("install-contributor-cargo-tools.sh cargo-mutants")
        );
        assert!(!mutants_preflight_message(&[]).contains("cargo-mutants\n"));
    }

    #[test]
    fn cargo_mutants_probe_uses_the_managed_workspace_environment() {
        let command = cargo_mutants_probe_command();

        assert_eq!(command.program, PathBuf::from("cargo"));
        assert_eq!(command.args, ["mutants", "--version"]);
        assert!(matches!(command.stdout, CommandStdout::Quiet));
        assert!(matches!(command.stderr, CommandStderr::Quiet));
        assert!(matches!(
            command.artifact_layout,
            CommandArtifactLayout::ManagedWorkspace
        ));
    }

    #[test]
    fn mutation_command_preserves_safe_and_ci_execution_modes() {
        let output_dir = Path::new("/tmp/htmlcut-mutants");
        let safe_command = mutants_command(output_dir, false, None, None);
        let ci_command = mutants_command(
            output_dir,
            true,
            Some("2/16"),
            Some(Path::new("/tmp/htmlcut.diff")),
        );

        assert_eq!(
            safe_command.args,
            ["mutants", "--output", "/tmp/htmlcut-mutants"]
        );
        assert!(matches!(
            safe_command.artifact_layout,
            CommandArtifactLayout::Inherit
        ));
        assert!(safe_command.live_output);
        assert_eq!(
            ci_command.args,
            [
                "mutants",
                "--output",
                "/tmp/htmlcut-mutants",
                "--in-place",
                "--shard",
                "2/16",
                "--in-diff",
                "/tmp/htmlcut.diff",
            ]
        );
        assert!(matches!(
            ci_command.artifact_layout,
            CommandArtifactLayout::ManagedWorkspace
        ));
        assert!(ci_command.live_output);
    }
}
