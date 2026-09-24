//! Pure summary derivation for completed maintainer-gate runs.

use std::io::{self, Write};

use super::*;

impl GateRun {
    pub(super) fn human_summary_text(&self) -> String {
        let (passed, failed) = gate_step_outcome_counts(&self.report.steps);
        format!(
            "HTMLCut gate `{}` {}: {} passed, {} failed, {} distinct warnings, {} ms.",
            self.report.gate,
            if self.report.outcome == GateOutcome::Passed {
                "passed"
            } else {
                "failed"
            },
            passed,
            failed,
            self.report.warnings.len(),
            self.report.duration_ms,
        )
    }

    pub(super) fn write_human_summary(&self) -> DynResult<()> {
        let stdout = io::stdout();
        let stderr = io::stderr();
        let mut stdout = stdout.lock();
        let mut stderr = stderr.lock();
        self.write_human_summary_to(&mut stdout, &mut stderr)
            .map_err(Into::into)
    }

    pub(super) fn write_human_summary_to(
        &self,
        stdout: &mut dyn Write,
        stderr: &mut dyn Write,
    ) -> io::Result<()> {
        let (stream, summary) = self.render_human_summary();
        match stream {
            HumanSummaryStream::Stdout => writeln!(stdout, "{summary}")?,
            HumanSummaryStream::Stderr => writeln!(stderr, "{summary}")?,
        }
        for warning in &self.report.warnings {
            writeln!(stderr, "{}", render_warning(warning))?;
        }
        if let Some(failure) = &self.report.failure {
            writeln!(stderr, "failure: {}", failure.message)?;
        }
        for step in self
            .report
            .steps
            .iter()
            .filter(|step| step.outcome == GateOutcome::Failed)
        {
            if let Some(tail) = &step.failure_tail {
                writeln!(stderr, "--- failed step {} ---\n{tail}", step.label)?;
            }
        }
        writeln!(stdout, "Gate report: {}", self.report.report_path)
    }

    pub(super) fn render_human_summary(&self) -> (HumanSummaryStream, String) {
        let stream = if self.report.outcome == GateOutcome::Passed {
            HumanSummaryStream::Stdout
        } else {
            HumanSummaryStream::Stderr
        };
        (stream, self.human_summary_text())
    }
}

pub(super) fn render_warning(warning: &GateWarning) -> String {
    format!(
        "warning [{}]: {}",
        render_stream(warning.stream),
        warning.message
    )
}

fn gate_step_outcome_counts(steps: &[GateStep]) -> (usize, usize) {
    let passed = steps
        .iter()
        .filter(|step| is_successful_outcome(step.outcome))
        .count();
    let failed = steps
        .iter()
        .filter(|step| !is_successful_outcome(step.outcome))
        .count();
    (passed, failed)
}
