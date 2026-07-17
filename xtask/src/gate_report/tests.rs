use super::*;

use std::fs;
use std::io::ErrorKind;
use std::process::Command;

use htmlcut_tempdir::tempdir;
use serde_json::Value;

use crate::{
    CommandStdout, CommandToolchainEnv, capture_command_output, gate_report_dir, run_spec,
};

fn output_options(format: GateOutputFormat) -> GateOutputOptions {
    GateOutputOptions {
        format,
        verbose: false,
    }
}

fn command_spec(args: impl IntoIterator<Item = impl Into<String>>) -> CommandSpec {
    CommandSpec::new(
        "cargo",
        args,
        CommandStdout::Quiet,
        CommandToolchainEnv::Inherit,
    )
}

fn cargo_output(args: &[&str]) -> Output {
    Command::new("cargo")
        .args(args)
        .output()
        .expect("run cargo for retained report fixture")
}

fn with_gate_report_root<T>(operation: impl FnOnce(&Path) -> T) -> T {
    let repo = tempdir().expect("temporary repository");
    let target_dir = repo.path().join(".htmlcut-artifacts").join("target");
    let build_dir = repo.path().join(".htmlcut-artifacts").join("build");
    crate::plan::with_cargo_artifact_dir_overrides_for_tests(target_dir, build_dir, || {
        operation(repo.path())
    })
}

fn report_value(run: &GateRun) -> Value {
    serde_json::from_slice(&fs::read(&run.report.report_path).expect("read retained gate report"))
        .expect("parse retained gate report")
}

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
fn successful_command_retains_separate_streams_and_a_complete_report() {
    with_gate_report_root(|repo_root| {
        let mut options = output_options(GateOutputFormat::Human);
        options.verbose = true;
        let mut run = GateRun::start(repo_root, "test-gate", options).expect("start gate run");
        let spec = command_spec(["--version"]).with_env("SENSITIVE_VALUE", "never-recorded");
        let output = cargo_output(&["--version"]);
        let index = run.begin_command(&spec);

        assert_eq!(index, 1);
        assert_eq!(
            run.finish_command(index, &spec, &output, Duration::from_millis(7)),
            ""
        );
        run.finish(None).expect("finish successful run");

        let report = report_value(&run);
        assert_eq!(report["schema"], "htmlcut.gate_run@1");
        assert_eq!(report["outcome"], "passed");
        assert_eq!(report["steps"][0]["command"]["program"], "cargo");
        assert_eq!(
            report["steps"][0]["command"]["environment_keys"],
            serde_json::json!(["SENSITIVE_VALUE"])
        );
        assert_eq!(report["steps"][0]["stdout_bytes"], output.stdout.len());
        assert_eq!(report["steps"][0]["stderr_bytes"], output.stderr.len());
        assert!(run.run_dir.join("steps/001.stdout.log").is_file());
        assert!(run.run_dir.join("steps/001.stderr.log").is_file());
    });
}

#[test]
fn instrumented_command_helpers_record_success_failure_and_spawn_errors() {
    with_gate_report_root(|repo_root| {
        with_gate_report(
            repo_root,
            "instrumented-success",
            output_options(GateOutputFormat::Json),
            || -> DynResult<()> {
                run_spec(repo_root, &command_spec(["--version"]))?;
                let version = capture_command_output(repo_root, &command_spec(["--version"]))?;
                assert!(String::from_utf8_lossy(&version).contains("cargo"));
                Ok(())
            },
        )
        .expect("instrumented success");

        let command_failure = with_gate_report(
            repo_root,
            "instrumented-command-failure",
            output_options(GateOutputFormat::Json),
            || {
                run_spec(
                    repo_root,
                    &command_spec(["__htmlcut_gate_report_test_failure__"]),
                )
            },
        )
        .expect_err("instrumented command failure");
        assert!(command_failure.to_string().contains("retained logs"));

        let capture_failure = with_gate_report(
            repo_root,
            "instrumented-capture-failure",
            output_options(GateOutputFormat::Json),
            || {
                capture_command_output(
                    repo_root,
                    &command_spec(["__htmlcut_gate_report_test_failure__"]),
                )
                .map(|_| ())
            },
        )
        .expect_err("instrumented capture failure");
        assert!(capture_failure.to_string().contains("retained logs"));

        let spawn_failure = with_gate_report(
            repo_root,
            "instrumented-spawn-failure",
            output_options(GateOutputFormat::Json),
            || {
                run_spec(
                    repo_root,
                    &CommandSpec::new(
                        "htmlcut-no-such-maintainer-tool",
                        Vec::<String>::new(),
                        CommandStdout::Quiet,
                        CommandToolchainEnv::Inherit,
                    ),
                )
            },
        )
        .expect_err("instrumented spawn failure");
        assert!(spawn_failure.to_string().contains("could not start"));

        let capture_spawn_failure = with_gate_report(
            repo_root,
            "instrumented-capture-spawn-failure",
            output_options(GateOutputFormat::Json),
            || {
                capture_command_output(
                    repo_root,
                    &CommandSpec::new(
                        "htmlcut-no-such-maintainer-tool",
                        Vec::<String>::new(),
                        CommandStdout::Quiet,
                        CommandToolchainEnv::Inherit,
                    ),
                )
                .map(|_| ())
            },
        )
        .expect_err("instrumented capture spawn failure");
        assert!(
            capture_spawn_failure
                .to_string()
                .contains("could not start")
        );
    });
}

#[test]
fn instrumented_execution_fails_closed_when_it_cannot_retain_evidence() {
    with_gate_report_root(|repo_root| {
        let run_failure = with_gate_report(
            repo_root,
            "instrumented-log-write-failure",
            output_options(GateOutputFormat::Json),
            || {
                let steps_dir =
                    with_active(|run| run.run_dir.join("steps")).expect("active step directory");
                fs::remove_dir(&steps_dir).expect("remove step directory");
                fs::write(&steps_dir, "not a directory").expect("block step directory");
                run_spec(repo_root, &command_spec(["--version"]))
            },
        )
        .expect_err("command evidence write failure");
        assert!(
            run_failure
                .to_string()
                .contains("could not retain evidence")
        );

        let capture_failure = with_gate_report(
            repo_root,
            "instrumented-capture-log-write-failure",
            output_options(GateOutputFormat::Json),
            || {
                let steps_dir =
                    with_active(|run| run.run_dir.join("steps")).expect("active step directory");
                fs::remove_dir(&steps_dir).expect("remove step directory");
                fs::write(&steps_dir, "not a directory").expect("block step directory");
                capture_command_output(repo_root, &command_spec(["--version"])).map(|_| ())
            },
        )
        .expect_err("capture evidence write failure");
        assert!(
            capture_failure
                .to_string()
                .contains("could not retain evidence")
        );
    });
}

#[test]
fn failed_and_unstartable_commands_remain_queryable_without_streaming_noise() {
    with_gate_report_root(|repo_root| {
        let mut run = GateRun::start(
            repo_root,
            "test-gate",
            output_options(GateOutputFormat::Human),
        )
        .expect("start gate run");
        let failed_spec = command_spec(["__htmlcut_gate_report_test_failure__"]);
        let failed_output = cargo_output(&["__htmlcut_gate_report_test_failure__"]);
        assert!(!failed_output.status.success(), "fixture command must fail");

        let failed_index = run.begin_command(&failed_spec);
        let failure = run.finish_command(
            failed_index,
            &failed_spec,
            &failed_output,
            Duration::from_millis(11),
        );
        assert!(failure.contains("retained logs"));

        let missing_spec = CommandSpec::new(
            "htmlcut-no-such-maintainer-tool",
            Vec::<String>::new(),
            CommandStdout::Quiet,
            CommandToolchainEnv::Inherit,
        );
        let missing_index = run.begin_command(&missing_spec);
        let spawn_failure = run.finish_command_spawn_failure(
            missing_index,
            &missing_spec,
            &std::io::Error::new(ErrorKind::NotFound, "not installed"),
            Duration::from_millis(13),
        );
        assert!(spawn_failure.contains("could not start"));
        run.finish(Some(&failure)).expect("finish failed run");

        let report = report_value(&run);
        assert_eq!(report["outcome"], "failed");
        assert!(report["failure"]["message"].as_str().is_some());
        assert!(
            report["steps"][0]["failure_tail"]
                .as_str()
                .is_some_and(|tail| tail.contains("stderr:"))
        );
        assert!(
            report["steps"][1]["failure_tail"]
                .as_str()
                .is_some_and(|tail| tail.contains("not installed"))
        );
    });
}

#[test]
fn evidence_write_failures_fail_the_step_instead_of_silently_losing_diagnostics() {
    with_gate_report_root(|repo_root| {
        let mut run = GateRun::start(
            repo_root,
            "test-gate",
            output_options(GateOutputFormat::Human),
        )
        .expect("start gate run");
        fs::remove_dir(run.run_dir.join("steps")).expect("remove step directory");
        fs::write(run.run_dir.join("steps"), "not a directory").expect("block evidence path");
        let spec = command_spec(["--version"]);
        let output = cargo_output(&["--version"]);
        let index = run.begin_command(&spec);
        let failure = run.finish_command(index, &spec, &output, Duration::ZERO);

        assert!(failure.contains("could not retain evidence"));
        run.finish(Some(&failure)).expect("finish failed report");
        let report = report_value(&run);
        assert_eq!(report["steps"][0]["outcome"], "failed");
        assert!(
            report["steps"][0]["failure_tail"]
                .as_str()
                .is_some_and(|tail| tail.contains("failed to retain command logs"))
        );
    });
}

#[test]
fn internal_checks_and_wrapper_lifecycle_are_retained_in_execution_order() {
    with_gate_report_root(|repo_root| {
        let report_root = gate_report_dir(repo_root);
        with_gate_report(
            repo_root,
            "lifecycle",
            output_options(GateOutputFormat::Human),
            || -> DynResult<()> {
                assert!(is_active());
                record_internal_check("first", Ok(()), Duration::from_millis(1));
                record_internal_check(
                    "second",
                    Err("broken invariant".to_owned()),
                    Duration::from_millis(2),
                );

                let nested = with_gate_report(
                    repo_root,
                    "nested",
                    output_options(GateOutputFormat::Human),
                    || Ok(()),
                );
                assert!(
                    nested.is_err(),
                    "nested reporting must fail before it creates evidence"
                );
                Err("outer failure".into())
            },
        )
        .expect_err("outer operation must fail");

        assert!(!is_active(), "the thread-local reporter must always clear");
        let reports = fs::read_dir(&report_root)
            .expect("read report root")
            .filter_map(Result::ok)
            .filter(|entry| entry.path().join("report.json").is_file())
            .collect::<Vec<_>>();
        assert_eq!(
            reports.len(),
            1,
            "nested invocation must not create an orphan run"
        );
        let report = serde_json::from_slice::<Value>(
            &fs::read(reports[0].path().join("report.json")).expect("read report"),
        )
        .expect("parse report");
        assert_eq!(report["outcome"], "failed");
        assert_eq!(report["steps"][0]["kind"], "internal_check");
        assert_eq!(report["steps"][1]["failure_tail"], "broken invariant");
    });
}

#[test]
fn report_finalization_failures_preserve_the_original_gate_failure_when_present() {
    with_gate_report_root(|repo_root| {
        let report_error = with_gate_report(
            repo_root,
            "report-write-failure",
            output_options(GateOutputFormat::Human),
            || -> DynResult<()> {
                let report_path =
                    with_active(|run| run.report.report_path.clone()).expect("active report path");
                fs::create_dir_all(report_path).expect("block report file path");
                Ok(())
            },
        )
        .expect_err("report finalization should fail");
        assert!(!report_error.to_string().is_empty());

        let combined_error = with_gate_report(
            repo_root,
            "report-and-gate-failure",
            output_options(GateOutputFormat::Human),
            || -> DynResult<()> {
                let report_path =
                    with_active(|run| run.report.report_path.clone()).expect("active report path");
                fs::create_dir_all(report_path).expect("block report file path");
                Err("gate operation failed".into())
            },
        )
        .expect_err("combined operation and report failure");
        let message = combined_error.to_string();
        assert!(message.contains("gate operation failed"));
        assert!(message.contains("could not finalize the gate report"));
    });
}

#[test]
fn warning_summaries_deduplicate_repeated_diagnostics() {
    with_gate_report_root(|repo_root| {
        let mut run = GateRun::start(
            repo_root,
            "warning-summary",
            output_options(GateOutputFormat::Human),
        )
        .expect("start gate run");
        let spec = command_spec(["--version"]);
        let mut first_output = cargo_output(&["--version"]);
        first_output.stderr = b"warning: retained once\n".to_vec();
        let mut second_output = cargo_output(&["--version"]);
        second_output.stderr = b"warning: retained once\n".to_vec();

        let first_index = run.begin_command(&spec);
        assert!(
            run.finish_command(first_index, &spec, &first_output, Duration::ZERO)
                .is_empty()
        );
        let second_index = run.begin_command(&spec);
        assert!(
            run.finish_command(second_index, &spec, &second_output, Duration::ZERO)
                .is_empty()
        );
        run.finish(None).expect("finish warning report");

        let report = report_value(&run);
        assert_eq!(report["warnings"].as_array().map(Vec::len), Some(1));
        assert_eq!(
            report["steps"][0]["warnings"].as_array().map(Vec::len),
            Some(1)
        );
        assert_eq!(
            report["steps"][1]["warnings"].as_array().map(Vec::len),
            Some(1)
        );
    });
}

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
        run.report.steps.push(GateStep {
            index: 1,
            id: "human-failure/001".to_owned(),
            kind: GateStepKind::InternalCheck,
            label: "fixture".to_owned(),
            command: None,
            outcome: GateOutcome::Failed,
            exit_code: None,
            duration_ms: 0,
            stdout_log: None,
            stderr_log: None,
            stdout_bytes: 0,
            stderr_bytes: 0,
            warnings: Vec::new(),
            failure_tail: Some("retained failure tail".to_owned()),
        });
        run.report.steps.push(GateStep {
            index: 2,
            id: "human-failure/002".to_owned(),
            kind: GateStepKind::InternalCheck,
            label: "empty fixture".to_owned(),
            command: None,
            outcome: GateOutcome::Failed,
            exit_code: None,
            duration_ms: 0,
            stdout_log: None,
            stderr_log: None,
            stdout_bytes: 0,
            stderr_bytes: 0,
            warnings: Vec::new(),
            failure_tail: None,
        });

        run.render_human_summary();
    });
}

#[test]
fn retention_prunes_the_oldest_completed_reports_and_preserves_the_current_run() {
    with_gate_report_root(|repo_root| {
        let root = prepare_gate_report_root(repo_root).expect("prepare report root");
        for index in 0..22 {
            let run = root.join(format!("run-{index:03}"));
            fs::create_dir_all(&run).expect("create completed run");
            fs::write(run.join("report.json"), "{}\n").expect("write completed report");
        }

        let mut run = GateRun::start(
            repo_root,
            "retention",
            output_options(GateOutputFormat::Human),
        )
        .expect("start retained run");
        assert!(!root.join("run-000").exists());
        assert!(!root.join("run-001").exists());
        assert!(!root.join("run-002").exists());
        run.finish(None).expect("finish retained run");

        let completed = fs::read_dir(&root)
            .expect("read report root")
            .filter_map(Result::ok)
            .filter(|entry| entry.path().join("report.json").is_file())
            .count();
        assert_eq!(completed, MAX_RETAINED_RUNS);
    });
}

#[test]
fn helpers_bound_failure_output_and_render_each_stream_kind() {
    let long_output = vec![b'x'; FAILURE_TAIL_BYTES + 4];
    assert_eq!(bounded_tail(&long_output).len(), FAILURE_TAIL_BYTES);
    assert!(bounded_tail(&long_output).bytes().all(|byte| byte == b'x'));
    assert!(combined_failure_tail(b"", b"").is_empty());
    let combined = combined_failure_tail(&long_output, &vec![b'y'; FAILURE_TAIL_BYTES + 4]);
    assert!(combined.len() <= FAILURE_TAIL_BYTES);
    assert!(combined.ends_with("yyyy"));
    assert_eq!(unix_millis(UNIX_EPOCH).expect("epoch timestamp"), 0);
    assert!(unix_millis(UNIX_EPOCH - Duration::from_millis(1)).is_err());
    assert_eq!(render_stream(GateStream::Stdout), "stdout");
    assert_eq!(render_stream(GateStream::Stderr), "stderr");
    replay_stream("empty", b"", false);
    replay_stream("stdout", b"retained output", false);
    replay_stream("stderr", b"retained error", true);
    record_internal_check("outside a gate", Ok(()), Duration::ZERO);
    record_internal_check(
        "outside a gate",
        Err("retained failure".to_owned()),
        Duration::ZERO,
    );
}
