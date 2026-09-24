//! Focused behavioral proofs for command execution decisions.

use std::process::Command;

use super::*;

#[test]
fn successful_stdout_replay_respects_the_stream_contract() {
    assert!(should_replay_success_stdout(CommandStdout::Inherit));
    assert!(!should_replay_success_stdout(CommandStdout::Quiet));
}

#[test]
fn successful_stderr_replay_respects_the_stream_contract() {
    assert!(should_replay_success_stderr(CommandStderr::Inherit));
    assert!(!should_replay_success_stderr(CommandStderr::Quiet));
}

#[test]
fn captured_success_stderr_replay_respects_the_stream_contract() {
    assert!(should_replay_captured_success_stderr(
        CommandStderr::Inherit
    ));
    assert!(!should_replay_captured_success_stderr(CommandStderr::Quiet));
}

#[test]
fn command_completion_classifies_process_and_evidence_outcomes_exhaustively() {
    assert_eq!(
        classify_command_completion(true, true),
        CommandCompletion::Succeeded
    );
    assert_eq!(
        classify_command_completion(true, false),
        CommandCompletion::EvidenceFailed
    );
    assert_eq!(
        classify_command_completion(false, true),
        CommandCompletion::ProcessFailed
    );
    assert_eq!(
        classify_command_completion(false, false),
        CommandCompletion::ProcessFailed
    );
}

#[test]
fn clang_override_is_enabled_only_for_the_explicit_toolchain_mode() {
    assert!(should_force_clang(CommandToolchainEnv::ForceClang));
    assert!(!should_force_clang(CommandToolchainEnv::Inherit));
}

#[test]
fn clang_override_sets_both_compiler_commands_only_when_requested() {
    let mut forced = Command::new("cargo");
    let forced_spec = CommandSpec::new(
        "cargo",
        ["--version"],
        CommandStdout::Quiet,
        CommandToolchainEnv::ForceClang,
    );
    apply_clang_override(&mut forced, &forced_spec);
    let forced_environment = forced
        .get_envs()
        .filter_map(|(key, value)| {
            value.map(|value| {
                (
                    key.to_string_lossy().into_owned(),
                    value.to_string_lossy().into_owned(),
                )
            })
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    assert_eq!(forced_environment.get("CC"), Some(&"clang".to_owned()));
    assert_eq!(forced_environment.get("CXX"), Some(&"clang++".to_owned()));

    let mut inherited = Command::new("cargo");
    let inherited_spec = CommandSpec::new(
        "cargo",
        ["--version"],
        CommandStdout::Quiet,
        CommandToolchainEnv::Inherit,
    );
    apply_clang_override(&mut inherited, &inherited_spec);
    assert!(inherited.get_envs().next().is_none());
}

#[test]
fn combined_failure_tail_separates_streams_and_retains_the_exact_latest_byte_budget() {
    assert_eq!(
        combined_tail(b"stdout payload", b"stderr payload"),
        "stdout:\nstdout payload\nstderr:\nstderr payload"
    );

    let stdout = vec![b'x'; 9 * 1024];
    let tail = combined_tail(&stdout, b"");
    assert_eq!(tail.len(), 8 * 1024);
    assert_eq!(tail, "x".repeat(8 * 1024));
}
