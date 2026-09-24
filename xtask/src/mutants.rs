//! cargo-mutants command construction and prerequisite checks.

use std::path::{Path, PathBuf};
#[cfg(test)]
use std::{cell::RefCell, rc::Rc};

use crate::{
    CommandArtifactLayout, CommandSpec, CommandStderr, CommandStdout, CommandToolchainEnv,
    DynResult,
};

mod local;

pub(crate) use local::{LocalMutationSummary, run_local_mutation_campaign};

#[cfg(test)]
type LocalCampaignOverride = Rc<dyn Fn() -> DynResult<LocalMutationSummary>>;

#[cfg(test)]
thread_local! {
    static LOCAL_CAMPAIGN_OVERRIDE: RefCell<Option<LocalCampaignOverride>> = const { RefCell::new(None) };
}

/// Runs the local mutation campaign through the production boundary.
pub(crate) fn run_local_mutation_campaign_for_gate(
    repo_root: &Path,
    output_dir: &Path,
    shard: Option<&str>,
    in_diff: Option<&Path>,
) -> DynResult<LocalMutationSummary> {
    #[cfg(test)]
    if let Some(override_fn) = LOCAL_CAMPAIGN_OVERRIDE.with_borrow(|slot| slot.clone()) {
        return override_fn();
    }

    run_local_mutation_campaign(repo_root, output_dir, shard, in_diff)
}

#[cfg(test)]
pub(crate) fn with_local_mutation_campaign_override<T>(
    override_fn: impl Fn() -> DynResult<LocalMutationSummary> + 'static,
    operation: impl FnOnce() -> T,
) -> T {
    LOCAL_CAMPAIGN_OVERRIDE.with_borrow_mut(|slot| {
        assert!(
            slot.is_none(),
            "local mutation campaign override is already installed"
        );
        *slot = Some(Rc::new(override_fn));
    });
    let outcome = operation();
    LOCAL_CAMPAIGN_OVERRIDE.with_borrow_mut(|slot| *slot = None);
    outcome
}

const SAFE_COPY_CARGO_TARGET_DIR: &str = ".htmlcut-mutant-cargo/target";
const SAFE_COPY_CARGO_BUILD_DIR: &str = ".htmlcut-mutant-cargo/build";
/// Each isolated mutation workspace retains one incremental graph so source edits recompile only
/// their affected crate instead of rebuilding the package from scratch for every mutant.
const MUTATION_CARGO_INCREMENTAL: &str = "1";

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
        "Mutation-testing preflight failed. HTMLCut runs pinned cargo-mutants with nightly libtest through `cargo xtask mutants`.\n",
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
    let (target_dir, build_dir) = if in_place {
        (
            output_dir.join("cargo-target"),
            output_dir.join("cargo-build"),
        )
    } else {
        (
            PathBuf::from(SAFE_COPY_CARGO_TARGET_DIR),
            PathBuf::from(SAFE_COPY_CARGO_BUILD_DIR),
        )
    };
    let mut args = vec![
        "mutants".to_owned(),
        "--workspace".to_owned(),
        "--output".to_owned(),
        output_dir.to_string_lossy().into_owned(),
        "--timeout".to_owned(),
        "600".to_owned(),
        "--jobserver".to_owned(),
        "false".to_owned(),
    ];
    // Local runs stage one complete disposable workspace before this command runs. CI passes
    // `in_place` only from an already-disposable checkout. Either way, cargo-mutants mutates in
    // place so every mutant reuses one isolated, incrementally rebuilt Cargo target. Cargo owns
    // invalidation for each source edit; the disposable worker boundary prevents cache leakage.
    args.push("--in-place".to_owned());
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
    .with_live_output()
    .with_env("RUSTUP_TOOLCHAIN", "nightly")
    .with_env("CARGO_TARGET_DIR", target_dir.to_string_lossy())
    .with_env("CARGO_BUILD_BUILD_DIR", build_dir.to_string_lossy())
    // Each local worker and CI shard owns this target root, so incremental state remains isolated
    // and bounded by that worker's lifetime while avoiding a full rebuild for every mutant.
    .with_env("CARGO_INCREMENTAL", MUTATION_CARGO_INCREMENTAL);
    if in_place {
        command.with_artifact_layout(CommandArtifactLayout::ManagedWorkspace)
    } else {
        command
    }
}

/// Builds a package-local cargo-mutants command for one disposable local worker.
pub(crate) fn mutants_package_command(
    output_dir: &Path,
    packages: &[String],
    shard: &str,
    in_diff: Option<&Path>,
) -> CommandSpec {
    let mut args = vec!["mutants".to_owned()];
    for package in packages {
        args.extend(["--package".to_owned(), package.clone()]);
    }
    args.extend([
        "--output".to_owned(),
        output_dir.to_string_lossy().into_owned(),
        "--timeout".to_owned(),
        "600".to_owned(),
        "--jobserver".to_owned(),
        "false".to_owned(),
        "--in-place".to_owned(),
        "--shard".to_owned(),
        shard.to_owned(),
    ]);
    if let Some(diff) = in_diff {
        args.extend(["--in-diff".to_owned(), diff.to_string_lossy().into_owned()]);
    }

    CommandSpec::new(
        "cargo",
        args,
        CommandStdout::Inherit,
        CommandToolchainEnv::Inherit,
    )
    .with_live_output()
    .with_env("RUSTUP_TOOLCHAIN", "nightly")
    .with_env("CARGO_TARGET_DIR", SAFE_COPY_CARGO_TARGET_DIR)
    .with_env("CARGO_BUILD_BUILD_DIR", SAFE_COPY_CARGO_BUILD_DIR)
    .with_env("CARGO_INCREMENTAL", MUTATION_CARGO_INCREMENTAL)
}

/// Builds the non-mutating cargo-mutants inventory command used to reconcile local shards.
pub(crate) fn mutants_list_command(shard: Option<&str>, in_diff: Option<&Path>) -> CommandSpec {
    let mut args = vec![
        "mutants".to_owned(),
        "--workspace".to_owned(),
        "--list".to_owned(),
        "--json".to_owned(),
    ];
    if let Some(shard) = shard {
        args.extend(["--shard".to_owned(), shard.to_owned()]);
    }
    if let Some(diff) = in_diff {
        args.extend(["--in-diff".to_owned(), diff.to_string_lossy().into_owned()]);
    }

    CommandSpec::new(
        "cargo",
        args,
        CommandStdout::Quiet,
        CommandToolchainEnv::Inherit,
    )
    .with_stderr(CommandStderr::Quiet)
    .with_env("RUSTUP_TOOLCHAIN", "nightly")
}

/// Builds a non-mutating package-local inventory command for one local worker shard.
pub(crate) fn mutants_package_list_command(
    packages: &[String],
    shard: &str,
    in_diff: Option<&Path>,
) -> CommandSpec {
    let mut args = vec!["mutants".to_owned()];
    for package in packages {
        args.extend(["--package".to_owned(), package.clone()]);
    }
    args.extend([
        "--list".to_owned(),
        "--json".to_owned(),
        "--shard".to_owned(),
        shard.to_owned(),
    ]);
    if let Some(diff) = in_diff {
        args.extend(["--in-diff".to_owned(), diff.to_string_lossy().into_owned()]);
    }
    CommandSpec::new(
        "cargo",
        args,
        CommandStdout::Quiet,
        CommandToolchainEnv::Inherit,
    )
    .with_stderr(CommandStderr::Quiet)
    .with_env("RUSTUP_TOOLCHAIN", "nightly")
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
            [
                "mutants",
                "--workspace",
                "--output",
                "/tmp/htmlcut-mutants",
                "--timeout",
                "600",
                "--jobserver",
                "false",
                "--in-place",
            ]
        );
        assert!(matches!(
            safe_command.artifact_layout,
            CommandArtifactLayout::Inherit
        ));
        assert_eq!(
            safe_command.env.get("RUSTUP_TOOLCHAIN"),
            Some(&"nightly".to_owned())
        );
        assert_eq!(
            safe_command.env.get("CARGO_INCREMENTAL"),
            Some(&"1".to_owned())
        );
        assert_eq!(
            safe_command.env.get("CARGO_TARGET_DIR"),
            Some(&SAFE_COPY_CARGO_TARGET_DIR.to_owned())
        );
        assert_eq!(
            safe_command.env.get("CARGO_BUILD_BUILD_DIR"),
            Some(&SAFE_COPY_CARGO_BUILD_DIR.to_owned())
        );
        assert!(safe_command.live_output);
        assert_eq!(
            ci_command.args,
            [
                "mutants",
                "--workspace",
                "--output",
                "/tmp/htmlcut-mutants",
                "--timeout",
                "600",
                "--jobserver",
                "false",
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
        assert_eq!(
            ci_command.env.get("RUSTUP_TOOLCHAIN"),
            Some(&"nightly".to_owned())
        );
        assert_eq!(
            ci_command.env.get("CARGO_INCREMENTAL"),
            Some(&"1".to_owned())
        );
        assert_eq!(
            ci_command.env.get("CARGO_TARGET_DIR"),
            Some(&"/tmp/htmlcut-mutants/cargo-target".to_owned())
        );
        assert_eq!(
            ci_command.env.get("CARGO_BUILD_BUILD_DIR"),
            Some(&"/tmp/htmlcut-mutants/cargo-build".to_owned())
        );
        assert!(ci_command.live_output);
    }

    #[test]
    fn mutation_inventory_command_preserves_filters_without_mutating_source() {
        let command =
            mutants_list_command(Some("2/16"), Some(Path::new("/tmp/htmlcut-mutants.diff")));

        assert_eq!(
            command.args,
            [
                "mutants",
                "--workspace",
                "--list",
                "--json",
                "--shard",
                "2/16",
                "--in-diff",
                "/tmp/htmlcut-mutants.diff",
            ]
        );
        assert!(!command.args.iter().any(|argument| argument == "--in-place"));
        assert!(matches!(command.stdout, CommandStdout::Quiet));
        assert!(matches!(command.stderr, CommandStderr::Quiet));
        assert_eq!(
            command.env.get("RUSTUP_TOOLCHAIN"),
            Some(&"nightly".to_owned())
        );
    }

    #[test]
    fn package_local_mutation_commands_preserve_exact_package_and_shard_scope() {
        let packages = ["htmlcut-core".to_owned(), "htmlcut-selectors".to_owned()];
        let run = mutants_package_command(
            Path::new("/tmp/htmlcut-mutants"),
            &packages,
            "1/3",
            Some(Path::new("/tmp/htmlcut-mutants.diff")),
        );
        let inventory = mutants_package_list_command(
            &packages,
            "1/3",
            Some(Path::new("/tmp/htmlcut-mutants.diff")),
        );

        assert_eq!(
            run.args,
            [
                "mutants",
                "--package",
                "htmlcut-core",
                "--package",
                "htmlcut-selectors",
                "--output",
                "/tmp/htmlcut-mutants",
                "--timeout",
                "600",
                "--jobserver",
                "false",
                "--in-place",
                "--shard",
                "1/3",
                "--in-diff",
                "/tmp/htmlcut-mutants.diff",
            ]
        );
        assert_eq!(
            inventory.args,
            [
                "mutants",
                "--package",
                "htmlcut-core",
                "--package",
                "htmlcut-selectors",
                "--list",
                "--json",
                "--shard",
                "1/3",
                "--in-diff",
                "/tmp/htmlcut-mutants.diff",
            ]
        );
        assert!(
            !inventory
                .args
                .iter()
                .any(|argument| argument == "--in-place")
        );
        assert!(matches!(
            run.artifact_layout,
            CommandArtifactLayout::Inherit
        ));
        assert_eq!(run.env.get("CARGO_INCREMENTAL"), Some(&"1".to_owned()));
    }
}
