use super::*;

#[test]
fn only_human_gate_output_emits_terminal_progress() {
    with_gate_report_root(|repo_root| {
        let human = GateRun::start(
            repo_root,
            "human-progress",
            output_options(GateOutputFormat::Human),
        )
        .expect("start human gate run");
        let json = GateRun::start(
            repo_root,
            "json-progress",
            output_options(GateOutputFormat::Json),
        )
        .expect("start JSON gate run");

        assert!(human.emits_human_progress());
        assert!(!json.emits_human_progress());
    });
}

#[test]
fn only_a_passed_outcome_renders_as_a_success() {
    assert!(is_successful_outcome(GateOutcome::Passed));
    assert!(!is_successful_outcome(GateOutcome::Failed));
    assert!(!is_successful_outcome(GateOutcome::Running));
}
