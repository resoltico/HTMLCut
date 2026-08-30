//! Behavioral evidence for incremental command-stream retention.

use std::fs;
use std::process::Command;
use std::time::Duration;

use super::*;

#[test]
fn warning_extraction_preserves_stream_and_ignores_progress_noise() {
    let warnings = warnings_from_output(
        b"Compiling htmlcut-core\nwarning: first\n",
        b"warning[E1234]: second\nFinished test profile\n",
    );

    assert_eq!(
        warnings,
        vec![
            GateWarning {
                stream: GateStream::Stdout,
                message: "warning: first".to_owned(),
            },
            GateWarning {
                stream: GateStream::Stderr,
                message: "warning[E1234]: second".to_owned(),
            },
        ]
    );
}

#[test]
fn streamed_command_reads_incremental_logs_into_the_gate_report() {
    with_gate_report_root(|repo_root| {
        let mut run = GateRun::start(
            repo_root,
            "streamed-command",
            output_options(GateOutputFormat::Human),
        )
        .expect("start gate run");
        let spec = command_spec(["--version"]);
        let index = run.begin_command(&spec);
        let (stdout_log, stderr_log) = run.command_log_paths(index);
        fs::write(&stdout_log, "progress\nwarning: retained stdout\n").expect("write stdout log");
        fs::write(&stderr_log, "warning[E1234]: retained stderr\n").expect("write stderr log");
        let status = cargo_output(&["--version"]).status;

        assert_eq!(
            run.finish_streamed_command(index, &spec, status, Duration::from_millis(3)),
            ""
        );
        run.finish(None).expect("finish streamed report");

        let report = report_value(&run);
        assert_eq!(report["steps"][0]["stdout_bytes"], 34);
        assert_eq!(report["steps"][0]["stderr_bytes"], 32);
        assert_eq!(report["warnings"].as_array().map(Vec::len), Some(2));
    });
}

#[test]
fn streamed_command_preserves_human_diagnostics_and_fails_closed_on_missing_evidence() {
    with_gate_report_root(|repo_root| {
        let mut options = output_options(GateOutputFormat::Human);
        options.verbose = true;
        let mut run =
            GateRun::start(repo_root, "streamed-diagnostics", options).expect("start gate run");
        let spec = command_spec(["--version"]);
        let live_spec = spec.clone().with_live_output();
        assert!(!run.should_mirror_live_output(&spec));
        assert!(run.should_mirror_live_output(&live_spec));

        let success_index = run.begin_command(&spec);
        let (success_stdout, success_stderr) = run.command_log_paths(success_index);
        fs::write(&success_stdout, "successful stdout\n").expect("write success stdout log");
        fs::write(&success_stderr, "successful stderr\n").expect("write success stderr log");
        let success_status = cargo_output(&["--version"]).status;
        assert_eq!(
            run.finish_streamed_command(
                success_index,
                &spec,
                success_status,
                Duration::from_millis(4),
            ),
            ""
        );

        let failed_index = run.begin_command(&spec);
        let (failed_stdout, failed_stderr) = run.command_log_paths(failed_index);
        fs::write(&failed_stdout, "failed stdout\n").expect("write failure stdout log");
        fs::write(&failed_stderr, "failed stderr\n").expect("write failure stderr log");
        let failed_status = cargo_output(&["__htmlcut_gate_report_test_failure__"]).status;
        let failure = run.finish_streamed_command(
            failed_index,
            &spec,
            failed_status,
            Duration::from_millis(5),
        );
        assert!(failure.contains("retained logs"));

        let missing_index = run.begin_command(&spec);
        let evidence_failure = run.finish_streamed_command(
            missing_index,
            &spec,
            success_status,
            Duration::from_millis(6),
        );
        assert!(evidence_failure.contains("could not retain evidence"));
        let explicit_evidence_failure = run.finish_command_evidence_failure(
            missing_index + 1,
            &spec,
            &std::io::Error::other("retention denied"),
            Duration::from_millis(7),
        );
        assert!(explicit_evidence_failure.contains("retention denied"));
        replay_log_stream("missing", &run.run_dir.join("missing.log"), false);

        run.finish(Some(&failure)).expect("finish failed report");
        let report = report_value(&run);
        assert_eq!(report["steps"][1]["outcome"], "failed");
        assert!(
            report["steps"][1]["failure_tail"]
                .as_str()
                .is_some_and(|tail| tail.contains("failed stderr"))
        );
    });
}

#[test]
fn streamed_runner_rejects_successful_commands_when_their_evidence_disappears() {
    with_gate_report_root(|repo_root| {
        let error = with_gate_report(
            repo_root,
            "streamed-runner-missing-evidence",
            output_options(GateOutputFormat::Json),
            || {
                crate::command_stream::with_stream_child_override(
                    || {
                        let (stdout_log, stderr_log) = with_active(|run| run.command_log_paths(1))
                            .expect("active command log paths");
                        fs::remove_file(stdout_log).expect("remove stdout log");
                        fs::remove_file(stderr_log).expect("remove stderr log");
                        let status = Command::new("cargo")
                            .arg("--version")
                            .status()
                            .expect("run successful status fixture");
                        Some(Ok(status))
                    },
                    || run_spec(repo_root, &command_spec(["--version"])),
                )
            },
        )
        .expect_err("missing stream evidence must fail a successful command");
        assert!(error.to_string().contains("could not retain evidence"));
    });
}
