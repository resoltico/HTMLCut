use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Deserialize;

use crate::model::DynResult;

/// One maintained release-matrix entry declared in `scripts/release-targets.sh`.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct ReleaseMatrixEntry {
    /// Stable entry identifier used by workflow jobs and artifacts.
    pub id: String,
    /// GitHub Actions runner label.
    pub runs_on: String,
    /// Target triple for the built standalone package.
    pub target_triple: String,
    /// Artifact bundle name used for workflow handoff.
    pub artifact_bundle_name: String,
    /// Whether the job requires MUSL tooling bootstrap.
    pub needs_musl_tools: bool,
}

#[derive(Debug, Deserialize)]
struct ReleaseMatrix {
    include: Vec<ReleaseMatrixEntry>,
}

/// Reads the maintained release target triples from the canonical shell registry.
pub fn release_target_triples(repo_root: &Path) -> DynResult<Vec<String>> {
    script_lines(repo_root, "release_target_triples", &[])
}

/// Reads the maintained release asset names for one version from the canonical shell registry.
pub fn release_asset_names(repo_root: &Path, version: &str) -> DynResult<Vec<String>> {
    script_lines(repo_root, "release_asset_names_for_version", &[version])
}

/// Reads the maintained release matrix from the canonical shell registry.
pub fn release_matrix(repo_root: &Path) -> DynResult<Vec<ReleaseMatrixEntry>> {
    let output = script_output(repo_root, "release_matrix_json", &[])?;
    let matrix: ReleaseMatrix = serde_json::from_str(output.trim())
        .map_err(|error| format!("could not parse release_matrix_json output: {error}"))?;
    Ok(matrix.include)
}

/// Reads the maintained macOS deployment target for one target triple, when one exists.
pub fn macos_deployment_target(repo_root: &Path, target: &str) -> DynResult<Option<String>> {
    let output = script_output(repo_root, "macos_deployment_target_for_target", &[target])?;
    let trimmed = output.trim();
    if trimmed.is_empty() {
        Ok(None)
    } else {
        Ok(Some(trimmed.to_owned()))
    }
}

fn script_lines(repo_root: &Path, function_name: &str, args: &[&str]) -> DynResult<Vec<String>> {
    Ok(script_output(repo_root, function_name, args)?
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect())
}

fn script_output(repo_root: &Path, function_name: &str, args: &[&str]) -> DynResult<String> {
    script_output_with_program("bash", repo_root, function_name, args)
}

fn script_output_with_program(
    program: &str,
    repo_root: &Path,
    function_name: &str,
    args: &[&str],
) -> DynResult<String> {
    let script_path = release_targets_script_path(repo_root);
    if !script_path.is_file() {
        return Err(format!(
            "missing canonical release target script: {}",
            script_path.display()
        )
        .into());
    }

    let mut command = Command::new(program);
    command.current_dir(repo_root);
    command.arg(bash_source_argument(&script_path));
    command.args(script_command_args(function_name, args)?);

    let output = command.output().map_err(|error| {
        format!(
            "could not execute {} from {}: {error}",
            function_name,
            script_path.display()
        )
    })?;
    if !output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let mut stream_suffix = String::new();
        if !stdout.trim().is_empty() {
            stream_suffix.push_str("\nstdout:\n");
            stream_suffix.push_str(stdout.trim_end());
        }
        if !stderr.trim().is_empty() {
            stream_suffix.push_str("\nstderr:\n");
            stream_suffix.push_str(stderr.trim_end());
        }
        return Err(format!(
            "{} failed for {} with status {}{}",
            function_name,
            script_path.display(),
            output.status,
            stream_suffix
        )
        .into());
    }

    String::from_utf8(output.stdout)
        .map_err(|error| format!("{} returned non-UTF-8 output: {error}", function_name).into())
}

fn script_command_args(function_name: &str, args: &[&str]) -> DynResult<Vec<String>> {
    match (function_name, args) {
        ("release_target_triples", []) => Ok(vec!["triples".to_owned()]),
        ("release_asset_names_for_version", [version]) => Ok(vec![
            "assets".to_owned(),
            "--version".to_owned(),
            (*version).to_owned(),
        ]),
        ("release_matrix_json", []) => Ok(vec!["matrix-json".to_owned()]),
        ("macos_deployment_target_for_target", [target]) => Ok(vec![
            "macos-deployment-target".to_owned(),
            "--target".to_owned(),
            (*target).to_owned(),
        ]),
        _ => Err(format!(
            "unsupported canonical release-target helper call: {function_name}({args:?})"
        )
        .into()),
    }
}

fn release_targets_script_path(repo_root: &Path) -> PathBuf {
    repo_root.join("scripts").join("release-targets.sh")
}

fn bash_source_argument(path: &Path) -> String {
    let mut rendered = path.to_string_lossy().replace('\\', "/");
    if let Some(stripped) = rendered.strip_prefix("//?/") {
        rendered = stripped.to_owned();
    }

    let bytes = rendered.as_bytes();
    if bytes.len() >= 3 && bytes[1] == b':' && bytes[2] == b'/' && bytes[0].is_ascii_alphabetic() {
        let drive_letter = (bytes[0] as char).to_ascii_lowercase();
        let remainder = &rendered[3..];
        rendered = if remainder.is_empty() {
            format!("/{drive_letter}")
        } else {
            format!("/{drive_letter}/{remainder}")
        };
    }

    rendered
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    use htmlcut_tempdir::tempdir;

    #[test]
    fn release_helpers_report_missing_and_non_invocable_scripts() {
        let repo_root = tempdir().expect("tempdir");

        let error =
            release_target_triples(repo_root.path()).expect_err("missing script should fail");
        assert!(
            error
                .to_string()
                .contains("missing canonical release target script")
        );

        let scripts_dir = repo_root.path().join("scripts");
        fs::create_dir_all(&scripts_dir).expect("create scripts dir");
        fs::write(
            scripts_dir.join("release-targets.sh"),
            "#!/usr/bin/env bash\nprintf 'ok\\n'\n",
        )
        .expect("write release-targets.sh");

        let error = script_output_with_program(
            "htmlcut-definitely-missing-shell",
            repo_root.path(),
            "release_target_triples",
            &[],
        )
        .expect_err("missing shell should fail");
        assert!(
            error
                .to_string()
                .contains("could not execute release_target_triples")
        );
    }

    #[test]
    fn release_helpers_report_script_failures() {
        let repo_root = tempdir().expect("tempdir");
        let scripts_dir = repo_root.path().join("scripts");
        fs::create_dir_all(&scripts_dir).expect("create scripts dir");
        fs::write(
            scripts_dir.join("release-targets.sh"),
            r#"#!/usr/bin/env bash
if [[ "${1:-}" == "matrix-json" ]]; then
    exit 7
fi
"#,
        )
        .expect("write release-targets.sh");

        let error = release_matrix(repo_root.path()).expect_err("failing script should fail");
        assert!(error.to_string().contains("release_matrix_json failed"));
        assert!(error.to_string().contains("status"));
        assert!(!error.to_string().contains("stderr:"));
    }

    #[test]
    fn release_helpers_preserve_shell_stderr_when_available() {
        let repo_root = tempdir().expect("tempdir");
        let scripts_dir = repo_root.path().join("scripts");
        fs::create_dir_all(&scripts_dir).expect("create scripts dir");
        fs::write(
            scripts_dir.join("release-targets.sh"),
            r#"#!/usr/bin/env bash
printf 'boom\n' >&2
exit 9
"#,
        )
        .expect("write release-targets.sh");

        let error = script_output(repo_root.path(), "release_target_triples", &[])
            .expect_err("failing helper script should fail");
        let rendered = error.to_string();
        assert!(rendered.contains("release_target_triples failed"));
        assert!(rendered.contains("status"));
        assert!(!rendered.contains("stdout:"));
    }

    #[test]
    fn release_helpers_preserve_shell_stdout_when_stderr_is_empty() {
        let repo_root = tempdir().expect("tempdir");
        let scripts_dir = repo_root.path().join("scripts");
        fs::create_dir_all(&scripts_dir).expect("create scripts dir");
        fs::write(
            scripts_dir.join("release-targets.sh"),
            r#"#!/usr/bin/env bash
printf 'boom\n'
exit 9
"#,
        )
        .expect("write release-targets.sh");

        let error = script_output(repo_root.path(), "release_target_triples", &[])
            .expect_err("failing helper script should fail");
        let rendered = error.to_string();
        assert!(rendered.contains("release_target_triples failed"));
        assert!(rendered.contains("status"));
        assert!(!rendered.contains("stderr:"));
    }

    #[test]
    fn release_helpers_normalize_windows_script_paths_for_bash() {
        assert_eq!(
            bash_source_argument(Path::new(r"D:\a\HTMLCut\scripts\release-targets.sh")),
            "/d/a/HTMLCut/scripts/release-targets.sh"
        );
        assert_eq!(
            bash_source_argument(Path::new(r"\\?\D:\a\HTMLCut\scripts\release-targets.sh")),
            "/d/a/HTMLCut/scripts/release-targets.sh"
        );
        assert_eq!(bash_source_argument(Path::new("D:/")), "/d");
        assert_eq!(
            bash_source_argument(Path::new("scripts/release-targets.sh")),
            "scripts/release-targets.sh"
        );
        assert_eq!(bash_source_argument(Path::new("")), "");
        assert_eq!(
            bash_source_argument(Path::new("D:relative-path")),
            "D:relative-path"
        );
        assert_eq!(
            bash_source_argument(Path::new("1:/not-a-drive")),
            "1:/not-a-drive"
        );
    }

    #[test]
    fn release_helpers_reject_unknown_canonical_script_calls() {
        let error = script_command_args("definitely_not_a_real_helper", &["arg"])
            .expect_err("unknown helper call should fail");
        assert!(
            error
                .to_string()
                .contains("unsupported canonical release-target helper call")
        );
    }
}
