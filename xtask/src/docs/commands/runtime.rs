use std::path::PathBuf;

use super::parsing::option_value;

enum ExpectedArtifact {
    File(PathBuf),
    Bundle(PathBuf),
}

impl ExpectedArtifact {
    fn missing_message(self) -> Option<String> {
        match self {
            Self::File(path) => {
                (!path.is_file()).then(|| format!("expected file {} to exist", path.display()))
            }
            Self::Bundle(dir) => [
                dir.join("selection.html"),
                dir.join("selection.txt"),
                dir.join("report.json"),
            ]
            .into_iter()
            .find(|path| !path.is_file())
            .map(|path| format!("expected bundle artifact {} to exist", path.display())),
        }
    }
}

fn expected_artifacts(tokens: &[String]) -> Vec<ExpectedArtifact> {
    let mut artifacts = Vec::new();

    for flag in ["--emit-request-file", "--output-file"] {
        if let Some(path) = option_value(tokens, flag) {
            artifacts.push(ExpectedArtifact::File(PathBuf::from(path)));
        }
    }
    if let Some(path) = option_value(tokens, "--bundle") {
        artifacts.push(ExpectedArtifact::Bundle(PathBuf::from(path)));
    }

    artifacts
}

pub(super) fn documented_artifact_error(
    display_path: &str,
    example: &str,
    tokens: &[String],
) -> Option<String> {
    expected_artifacts(tokens)
        .into_iter()
        .find_map(ExpectedArtifact::missing_message)
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
