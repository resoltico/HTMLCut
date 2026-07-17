use super::*;

#[test]
fn capture_command_output_exports_managed_artifact_env() {
    let repo_root = tempdir().expect("repo tempdir");
    with_test_artifact_overrides(repo_root.path(), || {
        let cargo_config_dir = repo_root.path().join(".cargo");
        fs::create_dir_all(&cargo_config_dir).expect("create .cargo dir");
        fs::write(
            cargo_config_dir.join("config.toml"),
            "[build]\ntarget-dir = \".managed-artifacts/target\"\nbuild-dir = \".managed-artifacts/build\"\n",
        )
        .expect("write cargo config");

        let workspace_env = capture_command_output(
            repo_root.path(),
            &CommandSpec::new(
                "sh",
                [
                    "-c",
                    "printf '%s|%s' \"$CARGO_TARGET_DIR\" \"$CARGO_BUILD_BUILD_DIR\"",
                ],
                CommandStdout::Quiet,
                CommandToolchainEnv::Inherit,
            )
            .with_artifact_layout(CommandArtifactLayout::ManagedWorkspace),
        )
        .expect("workspace env");
        assert_eq!(
            String::from_utf8(workspace_env).expect("utf8"),
            format!(
                "{}|{}",
                cargo_target_dir(repo_root.path()).display(),
                cargo_build_dir(repo_root.path()).display()
            )
        );

        let coverage_env = capture_command_output(
            repo_root.path(),
            &CommandSpec::new(
                "sh",
                [
                    "-c",
                    "printf '%s|%s' \"$CARGO_TARGET_DIR\" \"$CARGO_BUILD_BUILD_DIR\"",
                ],
                CommandStdout::Quiet,
                CommandToolchainEnv::Inherit,
            )
            .with_artifact_layout(CommandArtifactLayout::ManagedCoverage),
        )
        .expect("coverage env");
        assert_eq!(
            String::from_utf8(coverage_env).expect("utf8"),
            format!(
                "{}|{}",
                coverage_target_dir(repo_root.path()).display(),
                coverage_build_dir(repo_root.path()).display()
            )
        );
    });
}

#[test]
fn hygiene_report_and_clean_cover_legacy_repo_local_roots() {
    let repo_root = tempdir().expect("repo tempdir");
    with_test_artifact_overrides(repo_root.path(), || {
        let cargo_config_dir = repo_root.path().join(".cargo");
        fs::create_dir_all(&cargo_config_dir).expect("create .cargo dir");
        fs::write(
            cargo_config_dir.join("config.toml"),
            "[build]\ntarget-dir = \".managed-artifacts/target\"\nbuild-dir = \".managed-artifacts/build\"\n",
        )
        .expect("write cargo config");

        let managed_workspace_target = cargo_target_dir(repo_root.path());
        let managed_workspace_build = cargo_build_dir(repo_root.path());
        let managed_coverage_target = coverage_target_dir(repo_root.path());
        let managed_coverage_build = coverage_build_dir(repo_root.path());
        let managed_gate_reports =
            prepare_gate_report_root(repo_root.path()).expect("prepare managed gate reports");
        fs::create_dir_all(managed_workspace_target.join("dist")).expect("create managed target");
        fs::create_dir_all(managed_workspace_build.join("debug/deps"))
            .expect("create managed build");
        fs::create_dir_all(managed_coverage_target.join("debug")).expect("create managed coverage");
        fs::create_dir_all(managed_coverage_build.join("debug"))
            .expect("create managed coverage build");
        fs::write(managed_workspace_target.join("dist/htmlcut"), "bin").expect("write bin");
        fs::write(managed_workspace_build.join("debug/deps/cache"), "cache")
            .expect("write managed cache");
        fs::write(managed_coverage_target.join("debug/cache"), "cache")
            .expect("write coverage cache");
        fs::write(managed_coverage_build.join("debug/cache"), "cache")
            .expect("write coverage build cache");

        let legacy_repo_target = repo_root.path().join("target/debug/deps");
        fs::create_dir_all(&legacy_repo_target).expect("create legacy repo target");
        let legacy_repo_target_file = std::fs::File::create(legacy_repo_target.join("legacy"))
            .expect("create legacy repo target file");
        legacy_repo_target_file
            .set_len(TEST_LEGACY_REPO_TARGET_BYTES)
            .expect("inflate legacy repo target file");

        let repo_tmp_cargo_root = repo_root.path().join("tmp/cargo-target-fieldfix/debug");
        fs::create_dir_all(&repo_tmp_cargo_root).expect("create repo tmp cargo target");
        fs::write(repo_tmp_cargo_root.join("artifact"), "artifact")
            .expect("write repo tmp artifact");

        assert!(crate::hygiene::looks_like_cargo_target_dir_for_tests(
            repo_root.path().join("tmp/cargo-target-fieldfix").as_path()
        ));

        let report = hygiene_report(repo_root.path()).expect("hygiene report");
        assert_eq!(
            report.total_bytes,
            report.entries.iter().map(|entry| entry.bytes).sum::<u64>()
        );
        assert!(
            report
                .violations
                .iter()
                .any(|violation| violation.id == "legacy-repo-target")
        );
        assert!(
            report
                .violations
                .iter()
                .any(|violation| violation.id == "repo-tmp-cargo-targets")
        );
        assert!(
            report
                .entries
                .iter()
                .find(|entry| entry.id == "repo-tmp")
                .expect("repo tmp entry")
                .details
                .iter()
                .any(|detail| detail.contains("Excludes 1 repo-local Cargo target roots"))
        );
        assert_eq!(crate::hygiene::format_bytes_for_tests(1024), "1.0 KiB");

        let safe_clean =
            clean_hygiene(repo_root.path(), HygieneCleanMode::Safe).expect("safe clean");
        assert!(
            safe_clean
                .removed_paths
                .iter()
                .any(|path| path == &repo_root.path().join("tmp"))
        );
        assert!(
            safe_clean
                .removed_paths
                .iter()
                .any(|path| path.ends_with("coverage-target"))
        );
        assert!(
            safe_clean
                .removed_paths
                .iter()
                .any(|path| path == &repo_root.path().join("target"))
        );
        assert!(!repo_root.path().join("tmp").exists());
        assert!(!repo_root.path().join("target").exists());
        assert!(
            managed_workspace_target.exists(),
            "safe clean keeps managed workspace target"
        );
        assert!(
            managed_workspace_build.exists(),
            "safe clean keeps managed workspace build"
        );
        assert!(
            managed_gate_reports.exists(),
            "safe clean keeps retained gate evidence"
        );

        let rebuildable_clean = clean_hygiene(repo_root.path(), HygieneCleanMode::Rebuildable)
            .expect("rebuildable clean");
        assert!(
            rebuildable_clean
                .removed_paths
                .iter()
                .any(|path| path == &managed_workspace_target)
        );
        assert!(
            rebuildable_clean
                .removed_paths
                .iter()
                .any(|path| path == &managed_workspace_build)
        );
        assert!(
            rebuildable_clean
                .removed_paths
                .iter()
                .any(|path| path == &managed_gate_reports)
        );
        assert!(
            rebuildable_clean
                .removed_paths
                .iter()
                .all(|path| path != &repo_root.path().join("target"))
        );
        assert!(!managed_workspace_target.exists());
        assert!(!managed_workspace_build.exists());
        assert!(!managed_gate_reports.exists());
    });
}

#[test]
fn render_hygiene_report_and_ensure_hygiene_surface_policy_failures() {
    let repo_root = tempdir().expect("repo tempdir");
    with_test_artifact_overrides(repo_root.path(), || {
        let cargo_config_dir = repo_root.path().join(".cargo");
        fs::create_dir_all(&cargo_config_dir).expect("create .cargo dir");
        fs::write(
            cargo_config_dir.join("config.toml"),
            "[build]\ntarget-dir = \".managed-artifacts/target\"\nbuild-dir = \".managed-artifacts/build\"\n",
        )
        .expect("write cargo config");

        let legacy_repo_target = repo_root.path().join("target/debug/deps");
        fs::create_dir_all(&legacy_repo_target).expect("create legacy repo target");
        let legacy_target_file =
            std::fs::File::create(legacy_repo_target.join("legacy")).expect("create legacy file");
        legacy_target_file
            .set_len(TEST_LEGACY_REPO_TARGET_BYTES)
            .expect("inflate legacy file");

        let report = hygiene_report(repo_root.path()).expect("hygiene report");
        let rendered = render_hygiene_report(&report);
        assert!(rendered.contains("violations:"));
        assert!(rendered.contains("legacy-repo-target:"));

        let error = ensure_hygiene(repo_root.path()).expect_err("hygiene should fail");
        let message = error.to_string();
        assert!(message.contains("artifact hygiene policy failed."));
        assert!(message.contains("cargo xtask hygiene report"));
        assert!(message.contains("cargo xtask hygiene clean --mode rebuildable"));
    });
}

#[cfg(unix)]
#[test]
fn hygiene_report_reports_managed_root_read_failures() {
    use std::os::unix::fs::PermissionsExt;

    let repo_root = tempdir().expect("repo tempdir");
    with_test_artifact_overrides(repo_root.path(), || {
        let cargo_config_dir = repo_root.path().join(".cargo");
        fs::create_dir_all(&cargo_config_dir).expect("create .cargo dir");
        fs::write(
            cargo_config_dir.join("config.toml"),
            "[build]\ntarget-dir = \".managed-artifacts/target\"\nbuild-dir = \".managed-artifacts/build\"\n",
        )
        .expect("write cargo config");

        let managed_target = cargo_target_dir(repo_root.path());
        fs::create_dir_all(managed_target.join("debug")).expect("create managed target");
        fs::write(managed_target.join("debug/artifact"), "artifact").expect("write artifact");

        let original_permissions = fs::metadata(&managed_target)
            .expect("managed target metadata")
            .permissions();
        let mut unreadable_permissions = original_permissions.clone();
        unreadable_permissions.set_mode(0o000);
        fs::set_permissions(&managed_target, unreadable_permissions).expect("lock managed target");

        let error = hygiene_report(repo_root.path()).expect_err("managed report should fail");

        fs::set_permissions(&managed_target, original_permissions).expect("unlock managed target");

        assert!(
            error
                .to_string()
                .contains(&managed_target.display().to_string())
        );
    });
}

#[cfg(unix)]
#[test]
fn hygiene_report_reports_unmanaged_root_read_failures() {
    use std::os::unix::fs::PermissionsExt;

    let repo_root = tempdir().expect("repo tempdir");
    with_test_artifact_overrides(repo_root.path(), || {
        let cargo_config_dir = repo_root.path().join(".cargo");
        fs::create_dir_all(&cargo_config_dir).expect("create .cargo dir");
        fs::write(
            cargo_config_dir.join("config.toml"),
            "[build]\ntarget-dir = \".managed-artifacts/target\"\nbuild-dir = \".managed-artifacts/build\"\n",
        )
        .expect("write cargo config");

        let legacy_target = repo_root.path().join("target/debug");
        fs::create_dir_all(&legacy_target).expect("create legacy target");
        fs::write(legacy_target.join("artifact"), "artifact").expect("write artifact");

        let original_permissions = fs::metadata(repo_root.path().join("target").as_path())
            .expect("legacy target metadata")
            .permissions();
        let mut unreadable_permissions = original_permissions.clone();
        unreadable_permissions.set_mode(0o000);
        fs::set_permissions(
            repo_root.path().join("target").as_path(),
            unreadable_permissions,
        )
        .expect("lock legacy target");

        let error = hygiene_report(repo_root.path()).expect_err("unmanaged report should fail");

        fs::set_permissions(
            repo_root.path().join("target").as_path(),
            original_permissions,
        )
        .expect("unlock legacy target");

        assert!(
            error
                .to_string()
                .contains(&repo_root.path().join("target").display().to_string())
        );
    });
}

#[cfg(unix)]
#[test]
fn hygiene_report_reports_tmp_root_read_failures() {
    use std::os::unix::fs::PermissionsExt;

    let repo_root = tempdir().expect("repo tempdir");
    with_test_artifact_overrides(repo_root.path(), || {
        let tmp_root = repo_root.path().join("tmp");
        fs::create_dir_all(&tmp_root).expect("create tmp root");

        let original_permissions = fs::metadata(&tmp_root)
            .expect("tmp root metadata")
            .permissions();
        let mut unreadable_permissions = original_permissions.clone();
        unreadable_permissions.set_mode(0o000);
        fs::set_permissions(&tmp_root, unreadable_permissions).expect("lock tmp root");

        let error = hygiene_report(repo_root.path()).expect_err("tmp root report should fail");

        fs::set_permissions(&tmp_root, original_permissions).expect("unlock tmp root");

        assert!(error.to_string().contains(&tmp_root.display().to_string()));
    });
}

#[cfg(unix)]
#[test]
fn hygiene_report_reports_tmp_cargo_aggregate_read_failures() {
    use std::os::unix::fs::PermissionsExt;

    let repo_root = tempdir().expect("repo tempdir");
    with_test_artifact_overrides(repo_root.path(), || {
        let cargo_config_dir = repo_root.path().join(".cargo");
        fs::create_dir_all(&cargo_config_dir).expect("create .cargo dir");
        fs::write(
            cargo_config_dir.join("config.toml"),
            "[build]\ntarget-dir = \".managed-artifacts/target\"\nbuild-dir = \".managed-artifacts/build\"\n",
        )
        .expect("write cargo config");

        let tmp_cargo_root = repo_root.path().join("tmp/cargo-target-proof/debug");
        fs::create_dir_all(&tmp_cargo_root).expect("create repo tmp cargo target");
        fs::write(tmp_cargo_root.join("artifact"), "artifact").expect("write artifact");

        let tmp_cargo_parent = repo_root.path().join("tmp/cargo-target-proof");
        let original_permissions = fs::metadata(&tmp_cargo_parent)
            .expect("tmp cargo target metadata")
            .permissions();
        let mut unreadable_permissions = original_permissions.clone();
        unreadable_permissions.set_mode(0o000);
        fs::set_permissions(&tmp_cargo_parent, unreadable_permissions)
            .expect("lock tmp cargo target");

        let error = hygiene_report(repo_root.path()).expect_err("aggregate report should fail");

        fs::set_permissions(&tmp_cargo_parent, original_permissions)
            .expect("unlock tmp cargo target");

        assert!(
            error
                .to_string()
                .contains(&tmp_cargo_parent.display().to_string())
        );
    });
}
