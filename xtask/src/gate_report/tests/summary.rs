use super::*;
use crate::gate_report::summary::render_warning;

#[test]
fn human_summary_prints_the_retained_tail_for_a_failed_step() {
    with_gate_report_root(|repo_root| {
        let mut run = GateRun::start(
            repo_root,
            "human-failure",
            output_options(GateOutputFormat::Human),
        )
        .expect("start gate run");
        run.report.outcome = GateOutcome::Failed;
        run.report.failure = Some(GateFailure {
            message: "fixture failure".to_owned(),
        });
        run.report.warnings.push(GateWarning {
            stream: GateStream::Stderr,
            message: "fixture warning".to_owned(),
        });
        for (index, label, tail) in [
            (1, "fixture", Some("retained failure tail")),
            (2, "empty fixture", None),
        ] {
            run.report.steps.push(GateStep {
                index,
                id: format!("human-failure/{index:03}"),
                kind: GateStepKind::InternalCheck,
                label: label.to_owned(),
                command: None,
                outcome: GateOutcome::Failed,
                exit_code: None,
                duration_ms: 0,
                stdout_log: None,
                stderr_log: None,
                stdout_bytes: 0,
                stderr_bytes: 0,
                warnings: Vec::new(),
                failure_tail: tail.map(str::to_owned),
            });
        }

        let (stream, summary) = run.render_human_summary();
        assert_eq!(stream, HumanSummaryStream::Stderr);
        assert_eq!(
            summary,
            "HTMLCut gate `human-failure` failed: 0 passed, 2 failed, 1 distinct warnings, 0 ms."
        );
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        run.write_human_summary_to(&mut stdout, &mut stderr)
            .expect("render human summary");
        assert_eq!(
            String::from_utf8(stdout).expect("stdout is UTF-8"),
            format!("Gate report: {}\n", run.report.report_path)
        );
        let stderr = String::from_utf8(stderr).expect("stderr is UTF-8");
        assert!(stderr.starts_with(
            "HTMLCut gate `human-failure` failed: 0 passed, 2 failed, 1 distinct warnings, 0 ms.\n"
        ));
        assert!(stderr.contains("failure: fixture failure\n"));
        assert!(stderr.contains("warning [stderr]: fixture warning\n"));
        assert!(stderr.contains("--- failed step fixture ---\nretained failure tail\n"));
        assert!(!stderr.contains("--- failed step empty fixture ---"));
    });
}

#[test]
fn warning_renderer_preserves_stream_and_message() {
    assert_eq!(
        render_warning(&GateWarning {
            stream: GateStream::Stdout,
            message: "fixture warning".to_owned(),
        }),
        "warning [stdout]: fixture warning"
    );
}
