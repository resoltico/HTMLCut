// SPDX-License-Identifier: MPL-2.0
use std::path::PathBuf;

use super::parsing::option_value;

fn expected_artifacts(tokens: &[String]) -> Vec<PathBuf> {
    ["--output", "--bundle", "--receipt"]
        .into_iter()
        .filter_map(|flag| option_value(tokens, flag).map(PathBuf::from))
        .collect()
}

pub(super) fn documented_artifact_error(
    display_path: &str,
    example: &str,
    tokens: &[String],
) -> Option<String> {
    expected_artifacts(tokens)
        .into_iter()
        .find_map(|path| (!path.is_file()).then(|| format!("expected file {} to exist", path.display())))
        .map(|message| {
            format!(
                "{display_path} example did not produce the documented artifact for `{example}` ({message})"
            )
        })
}

pub(super) fn render_execution_failure(exit_code: i32, stdout: &[u8], stderr: &[u8]) -> String {
    let stderr_excerpt = render_stream_excerpt(stderr);
    if !stderr_excerpt.is_empty() {
        return format!("exit code {exit_code}; stderr: {stderr_excerpt}");
    }

    let stdout_excerpt = render_stream_excerpt(stdout);
    if !stdout_excerpt.is_empty() {
        return format!("exit code {exit_code}; stdout: {stdout_excerpt}");
    }

    format!("exit code {exit_code}")
}

fn render_stream_excerpt(stream: &[u8]) -> String {
    String::from_utf8_lossy(stream)
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or_default()
        .to_owned()
}
