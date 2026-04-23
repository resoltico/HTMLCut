use std::path::Path;

use crate::{
    macos_deployment_target, release_asset_names, release_matrix, release_target_triples,
    workspace_version,
};

pub(super) fn release_doc_errors(repo_root: &Path, display_path: &str, text: &str) -> Vec<String> {
    let mut errors = Vec::new();

    if matches!(
        display_path,
        "docs/release-preflight.md" | "docs/release-publishing.md"
    ) {
        errors.extend(asset_name_errors(repo_root, display_path, text));
    }

    if display_path == "docs/release-publishing.md" {
        errors.extend(version_banner_check_errors(display_path, text));
    }

    if display_path == "docs/getting-started.md" {
        errors.extend(install_version_literal_errors(
            repo_root,
            display_path,
            text,
        ));
    }

    if display_path == "docs/platform-support.md" {
        errors.extend(asset_name_errors(repo_root, display_path, text));
        errors.extend(target_triple_errors(repo_root, display_path, text));
        errors.extend(matrix_pair_errors(repo_root, display_path, text));
        errors.extend(macos_floor_errors(repo_root, display_path, text));
    }

    if matches!(
        display_path,
        "docs/release-preflight.md" | "docs/release-closeout.md"
    ) {
        errors.extend(explicit_main_sync_errors(display_path, text));
    }

    if display_path == "docs/release-closeout.md" {
        errors.extend(worktree_handoff_errors(display_path, text));
    }

    errors
}

fn asset_name_errors(repo_root: &Path, display_path: &str, text: &str) -> Vec<String> {
    release_asset_names(repo_root, "X.Y.Z")
        .map(|asset_names| missing_literals(display_path, text, &asset_names, "release asset name"))
        .unwrap_or_else(|error| {
            vec![format!(
                "{display_path} could not load canonical release assets: {error}"
            )]
        })
}

fn install_version_literal_errors(repo_root: &Path, display_path: &str, text: &str) -> Vec<String> {
    workspace_version(repo_root)
        .map(|version| {
            missing_literals(
                display_path,
                text,
                &[
                    format!("VERSION={version}"),
                    format!("$Version = \"{version}\""),
                ],
                "install version literal",
            )
        })
        .unwrap_or_else(|error| {
            vec![format!(
                "{display_path} could not load canonical workspace version: {error}"
            )]
        })
}

fn target_triple_errors(repo_root: &Path, display_path: &str, text: &str) -> Vec<String> {
    release_target_triples(repo_root)
        .map(|target_triples| {
            missing_literals(display_path, text, &target_triples, "release target triple")
        })
        .unwrap_or_else(|error| {
            vec![format!(
                "{display_path} could not load canonical release targets: {error}"
            )]
        })
}

fn matrix_pair_errors(repo_root: &Path, display_path: &str, text: &str) -> Vec<String> {
    release_matrix(repo_root)
        .map(|entries| {
            entries
                .into_iter()
                .filter_map(|entry| {
                    let expected = format!("- `{}` for `{}`", entry.runs_on, entry.target_triple);
                    (!text.contains(&expected)).then(|| {
                        format!("{display_path} is missing the release runner mapping {expected}")
                    })
                })
                .collect()
        })
        .unwrap_or_else(|error| {
            vec![format!(
                "{display_path} could not load canonical release matrix: {error}"
            )]
        })
}

fn macos_floor_errors(repo_root: &Path, display_path: &str, text: &str) -> Vec<String> {
    match macos_deployment_target(repo_root, "aarch64-apple-darwin") {
        Ok(Some(target)) => {
            let expected = format!("macOS {target}");
            (!text.contains(&expected))
                .then(|| {
                    format!(
                        "{display_path} is missing the canonical macOS deployment floor {expected}"
                    )
                })
                .into_iter()
                .collect()
        }
        Ok(None) => vec![format!(
            "{display_path} could not determine the canonical macOS deployment floor"
        )],
        Err(error) => vec![format!(
            "{display_path} could not load canonical macOS deployment floor: {error}"
        )],
    }
}

fn explicit_main_sync_errors(display_path: &str, text: &str) -> Vec<String> {
    let mut errors = Vec::new();

    if text.contains("git pull") {
        errors.push(format!(
            "{display_path} must not use implicit `git pull` in the maintained release protocol"
        ));
    }

    if !text.contains("git fetch origin --prune --tags") {
        errors.push(format!(
            "{display_path} is missing the explicit `git fetch origin --prune --tags` sync step"
        ));
    }

    if !text.contains("git merge --ff-only origin/main") {
        errors.push(format!(
            "{display_path} is missing the explicit `git merge --ff-only origin/main` sync step"
        ));
    }

    errors
}

fn worktree_handoff_errors(display_path: &str, text: &str) -> Vec<String> {
    (!text.contains("checkout --detach"))
        .then(|| {
            format!(
                "{display_path} must document detaching a disposable release worktree before the primary checkout reclaims `main`"
            )
        })
        .into_iter()
        .collect()
}

fn version_banner_check_errors(display_path: &str, text: &str) -> Vec<String> {
    let mut errors = Vec::new();

    if !text.contains("grep \"^HTMLCut X.Y.Z$\"") {
        errors.push(format!(
            "{display_path} must validate the canonical `HTMLCut X.Y.Z` first version line during host-native release verification"
        ));
    }

    if text.contains("grep \"^htmlcut X.Y.Z$\"") {
        errors.push(format!(
            "{display_path} still documents the stale lowercase `htmlcut X.Y.Z` version-line check"
        ));
    }

    errors
}

fn missing_literals(
    display_path: &str,
    text: &str,
    expected_literals: &[String],
    label: &str,
) -> Vec<String> {
    expected_literals
        .iter()
        .filter(|expected| !text.contains(expected.as_str()))
        .map(|expected| format!("{display_path} is missing {label}: {expected}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    use htmlcut_tempdir::tempdir;

    #[test]
    fn release_doc_errors_report_registry_load_failures() {
        let repo_root = tempdir().expect("tempdir");

        let getting_started_errors = release_doc_errors(
            repo_root.path(),
            "docs/getting-started.md",
            "# Getting Started",
        );
        assert!(getting_started_errors.iter().any(|error| {
            error.contains("docs/getting-started.md could not load canonical workspace version")
        }));

        let publishing_errors = release_doc_errors(
            repo_root.path(),
            "docs/release-publishing.md",
            "# Release Publishing",
        );
        assert!(publishing_errors.iter().any(|error| {
            error.contains("docs/release-publishing.md could not load canonical release assets")
        }));

        let platform_errors = release_doc_errors(
            repo_root.path(),
            "docs/platform-support.md",
            "# Platform Support",
        );
        assert!(platform_errors.iter().any(|error| {
            error.contains("docs/platform-support.md could not load canonical release targets")
        }));
        assert!(platform_errors.iter().any(|error| {
            error.contains("docs/platform-support.md could not load canonical release matrix")
        }));
        assert!(platform_errors.iter().any(|error| {
            error.contains(
                "docs/platform-support.md could not load canonical macOS deployment floor",
            )
        }));
    }

    #[test]
    fn release_doc_errors_report_missing_macos_floor_from_registry() {
        let repo_root = tempdir().expect("tempdir");
        let scripts_dir = repo_root.path().join("scripts");
        fs::create_dir_all(&scripts_dir).expect("create scripts dir");
        fs::write(
            scripts_dir.join("release-targets.sh"),
            r#"#!/usr/bin/env bash
release_target_triples() {
    printf 'aarch64-apple-darwin\n'
}

release_matrix_json() {
    printf '{"include":[{"id":"macos-arm64","runs_on":"macos-15","target_triple":"aarch64-apple-darwin","artifact_bundle_name":"standalone-macos-arm64","needs_musl_tools":false}]}\n'
}

release_asset_names_for_version() {
    local release_version="$1"
    printf 'htmlcut-%s-checksums.txt\n' "${release_version}"
}

macos_deployment_target_for_target() {
    :
}
"#,
        )
        .expect("write release-targets.sh");

        let errors = release_doc_errors(
            repo_root.path(),
            "docs/platform-support.md",
            "- `aarch64-apple-darwin`\n- `macos-15` for `aarch64-apple-darwin`\n- `htmlcut-X.Y.Z-checksums.txt`\n",
        );

        assert!(errors.iter().any(|error| error.contains(
            "docs/platform-support.md could not determine the canonical macOS deployment floor"
        )));
    }

    #[test]
    fn explicit_main_sync_errors_reject_implicit_git_pull() {
        let errors =
            explicit_main_sync_errors("docs/release-preflight.md", "git checkout main\ngit pull\n");

        assert!(errors.iter().any(|error| {
            error.contains("docs/release-preflight.md must not use implicit `git pull`")
        }));
        assert!(errors.iter().any(|error| {
            error.contains("docs/release-preflight.md is missing the explicit `git fetch origin --prune --tags` sync step")
        }));
        assert!(errors.iter().any(|error| {
            error.contains("docs/release-preflight.md is missing the explicit `git merge --ff-only origin/main` sync step")
        }));
    }

    #[test]
    fn worktree_handoff_errors_require_detach_guidance() {
        let errors = worktree_handoff_errors("docs/release-closeout.md", "# Release Closeout");

        assert!(errors.iter().any(|error| {
            error.contains(
                "docs/release-closeout.md must document detaching a disposable release worktree",
            )
        }));
    }

    #[test]
    fn release_doc_errors_report_version_banner_check_drift() {
        let errors = release_doc_errors(
            Path::new("."),
            "docs/release-publishing.md",
            r#"# Release Publishing

./htmlcut-X.Y.Z-host/htmlcut --version | grep "^htmlcut X.Y.Z$"
"#,
        );

        assert!(errors.iter().any(|error| {
            error.contains("must validate the canonical `HTMLCut X.Y.Z` first version line")
        }));
        assert!(errors.iter().any(|error| {
            error.contains("still documents the stale lowercase `htmlcut X.Y.Z` version-line check")
        }));
    }
}
