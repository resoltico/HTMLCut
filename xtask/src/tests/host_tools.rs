use super::*;

#[test]
fn host_tool_probe_command_uses_version_and_stays_quiet() {
    let command = host_tool_probe_command("clang");

    assert_eq!(command.program, PathBuf::from("clang"));
    assert_eq!(command.args, vec!["--version"]);
    assert!(command.quiet_stdout);
    assert!(!command.force_clang);
}

#[test]
fn host_tool_preflight_reports_missing_tools_only() {
    let failures = host_tool_preflight_failures(&["clang", "clang++"], |tool| tool == "clang");

    assert_eq!(
        failures,
        vec![HostToolPreflightFailure::MissingTool("clang++".to_owned())]
    );
}

#[test]
fn host_tool_preflight_message_lists_the_context_and_missing_tools() {
    let message = host_tool_preflight_message(
        "coverage",
        &[
            HostToolPreflightFailure::MissingTool("clang".to_owned()),
            HostToolPreflightFailure::MissingTool("clang++".to_owned()),
        ],
    );

    assert!(message.contains("coverage preflight failed"));
    assert!(message.contains("CC=clang CXX=clang++"));
    assert!(message.contains("clang, clang++"));
}
