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
    let script_argument = shell_script_argument(program, &script_path);
    if !script_path.is_file() {
        return Err(format!(
            "missing canonical release target script: {}",
            script_path.display()
        )
        .into());
    }

    let mut command = Command::new(program);
    command.current_dir(repo_root);
    command.arg("-c");
    command.arg(script_command(function_name, args.len()));
    command.arg("bash");
    command.arg(script_argument);
    command.args(args);

    let output = command.output().map_err(|error| {
        format!(
            "could not execute {} from {}: {error}",
            function_name,
            script_path.display()
        )
    })?;
    if !output.status.success() {
        return Err(format!(
            "{} failed for {} with status {}",
            function_name,
            script_path.display(),
            output.status
        )
        .into());
    }

    String::from_utf8(output.stdout)
        .map_err(|error| format!("{} returned non-UTF-8 output: {error}", function_name).into())
}

fn script_command(function_name: &str, arg_count: usize) -> String {
    let mut command = format!("source \"$1\" && {function_name}");
    for index in 0..arg_count {
        command.push_str(&format!(" \"${}\"", index + 2));
    }
    command
}

fn shell_script_argument(program: &str, script_path: &Path) -> String {
    let argument = script_path.to_string_lossy().into_owned();
    if program.eq_ignore_ascii_case("bash") {
        argument.replace('\\', "/")
    } else {
        argument
    }
}

fn release_targets_script_path(repo_root: &Path) -> PathBuf {
    repo_root.join("scripts").join("release-targets.sh")
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
            "#!/usr/bin/env bash\nrelease_target_triples() { printf 'ok\\n'; }\n",
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
release_matrix_json() {
    return 7
}
"#,
        )
        .expect("write release-targets.sh");

        let error = release_matrix(repo_root.path()).expect_err("failing script should fail");
        assert!(error.to_string().contains("release_matrix_json failed"));
        assert!(error.to_string().contains("status"));
    }

    #[test]
    fn shell_script_argument_normalizes_backslashes_for_bash() {
        let bash_argument =
            shell_script_argument("bash", Path::new(r"D:\repo\scripts\release-targets.sh"));
        assert_eq!(bash_argument, "D:/repo/scripts/release-targets.sh");

        let native_argument =
            shell_script_argument("sh", Path::new(r"D:\repo\scripts\release-targets.sh"));
        assert_eq!(native_argument, r"D:\repo\scripts\release-targets.sh");
    }
}
