use super::*;

const TEST_LEGACY_REPO_TARGET_BYTES: u64 = 512 * 1024 * 1024 + 1;

fn with_test_artifact_overrides<T>(repo_root: &Path, operation: impl FnOnce() -> T) -> T {
    crate::plan::with_cargo_artifact_dir_overrides_for_tests(
        repo_root.join(".managed-artifacts/target"),
        repo_root.join(".managed-artifacts/build"),
        operation,
    )
}

#[test]
fn cargo_path_helpers_follow_the_configured_build_paths() {
    let repo_root = tempdir().expect("repo tempdir");
    with_test_artifact_overrides(repo_root.path(), || {
        let cargo_config_dir = repo_root.path().join(".cargo");
        fs::create_dir_all(&cargo_config_dir).expect("create .cargo dir");
        fs::write(
            cargo_config_dir.join("config.toml"),
            "[build]\ntarget-dir = \".managed-artifacts/target\"\nbuild-dir = \".managed-artifacts/build\"\n",
        )
        .expect("write cargo config");

        assert_eq!(
            cargo_target_dir(repo_root.path()),
            repo_root.path().join(".managed-artifacts/target")
        );
        assert_eq!(
            cargo_build_dir(repo_root.path()),
            repo_root.path().join(".managed-artifacts/build")
        );
        assert_eq!(
            crate::plan::cargo_target_dir_for_tests(
                repo_root.path(),
                Some(Path::new("managed-target"))
            ),
            repo_root.path().join("managed-target")
        );
        assert_eq!(
            crate::plan::cargo_build_dir_for_tests(
                repo_root.path(),
                Some(Path::new("managed-build"))
            ),
            repo_root.path().join("managed-build")
        );
        let absolute_build_root = tempdir().expect("absolute build tempdir");
        let absolute_build_dir = absolute_build_root.path().join("managed-build");
        assert_eq!(
            crate::plan::cargo_build_dir_for_tests(repo_root.path(), Some(&absolute_build_dir)),
            absolute_build_dir
        );
        assert_eq!(
            coverage_target_dir(repo_root.path()),
            repo_root.path().join(".managed-artifacts/coverage-target")
        );
        assert_eq!(
            coverage_build_dir(repo_root.path()),
            repo_root.path().join(".managed-artifacts/coverage-build")
        );
        assert_eq!(
            crate::plan::coverage_cargo_target_dir_for_tests(
                repo_root.path(),
                Some(Path::new("managed-target"))
            ),
            repo_root
                .path()
                .join("coverage-target")
                .join("llvm-cov-target")
        );
        assert_eq!(
            crate::plan::coverage_cargo_build_dir_for_tests(
                repo_root.path(),
                Some(Path::new("managed-build"))
            ),
            repo_root
                .path()
                .join("coverage-build")
                .join("llvm-cov-target")
        );
        assert_eq!(
            crate::plan::sibling_artifact_dir_for_tests(Path::new("target"), "coverage-target"),
            PathBuf::from("coverage-target")
        );
        assert_eq!(
            crate::plan::coverage_target_dir_for_tests(
                repo_root.path(),
                Some(Path::new("managed-target"))
            ),
            repo_root.path().join("coverage-target")
        );
        assert_eq!(
            crate::plan::coverage_build_dir_for_tests(
                repo_root.path(),
                Some(Path::new("managed-build"))
            ),
            repo_root.path().join("coverage-build")
        );
    });
}

#[test]
fn normalize_path_reports_missing_paths() {
    let repo_root = tempdir().expect("repo tempdir");
    let error = normalize_path(repo_root.path(), Path::new("missing-path"))
        .expect_err("missing path should fail");
    assert!(error.to_string().contains("No such file"));
}

#[test]
fn prepare_artifact_layout_creates_managed_roots_and_marker_files() {
    let repo_root = tempdir().expect("repo tempdir");
    with_test_artifact_overrides(repo_root.path(), || {
        let cargo_config_dir = repo_root.path().join(".cargo");
        fs::create_dir_all(&cargo_config_dir).expect("create .cargo dir");
        fs::write(
            cargo_config_dir.join("config.toml"),
            "[build]\ntarget-dir = \".managed-artifacts/target\"\nbuild-dir = \".managed-artifacts/build\"\n",
        )
        .expect("write cargo config");

        let (workspace_target, workspace_build) =
            prepare_artifact_layout(repo_root.path(), CommandArtifactLayout::ManagedWorkspace)
                .expect("prepare workspace layout")
                .expect("workspace dirs");
        let (coverage_target, coverage_build) =
            prepare_artifact_layout(repo_root.path(), CommandArtifactLayout::ManagedCoverage)
                .expect("prepare coverage layout")
                .expect("coverage dirs");
        let coverage_cargo_target = crate::plan::coverage_cargo_target_dir(repo_root.path());
        let coverage_cargo_build = crate::plan::coverage_cargo_build_dir(repo_root.path());

        for path in [
            workspace_target,
            workspace_build,
            coverage_target,
            coverage_build,
            coverage_cargo_target,
            coverage_cargo_build,
        ] {
            assert!(path.is_dir(), "{} should exist", path.display());
            assert!(path.join("CACHEDIR.TAG").is_file());
            assert!(path.join(".htmlcut-artifact.toml").is_file());
        }
    });
}

#[test]
fn prepare_artifact_layout_is_idempotent_when_marker_files_already_exist() {
    let repo_root = tempdir().expect("repo tempdir");
    with_test_artifact_overrides(repo_root.path(), || {
        let cargo_config_dir = repo_root.path().join(".cargo");
        fs::create_dir_all(&cargo_config_dir).expect("create .cargo dir");
        fs::write(
            cargo_config_dir.join("config.toml"),
            "[build]\ntarget-dir = \".managed-artifacts/target\"\nbuild-dir = \".managed-artifacts/build\"\n",
        )
        .expect("write cargo config");

        prepare_artifact_layout(repo_root.path(), CommandArtifactLayout::ManagedWorkspace)
            .expect("prepare workspace layout")
            .expect("workspace dirs");
        prepare_artifact_layout(repo_root.path(), CommandArtifactLayout::ManagedCoverage)
            .expect("prepare coverage layout")
            .expect("coverage dirs");

        prepare_artifact_layout(repo_root.path(), CommandArtifactLayout::ManagedWorkspace)
            .expect("prepare workspace layout again")
            .expect("workspace dirs");
        prepare_artifact_layout(repo_root.path(), CommandArtifactLayout::ManagedCoverage)
            .expect("prepare coverage layout again")
            .expect("coverage dirs");
    });
}

#[test]
fn prepare_artifact_layout_inherit_leaves_artifact_env_unmanaged() {
    let repo_root = tempdir().expect("repo tempdir");
    assert_eq!(
        prepare_artifact_layout(repo_root.path(), CommandArtifactLayout::Inherit)
            .expect("inherit layout"),
        None
    );
}

#[cfg(unix)]
#[test]
fn prepare_artifact_layout_reports_workspace_root_creation_failures() {
    let repo_root = tempdir().expect("repo tempdir");
    with_test_artifact_overrides(repo_root.path(), || {
        let cargo_config_dir = repo_root.path().join(".cargo");
        fs::create_dir_all(&cargo_config_dir).expect("create .cargo dir");
        fs::write(
            cargo_config_dir.join("config.toml"),
            "[build]\ntarget-dir = \".managed-artifacts/target\"\nbuild-dir = \".managed-artifacts/build\"\n",
        )
        .expect("write cargo config");

        let managed_target_file = repo_root.path().join(".managed-artifacts/target");
        fs::create_dir_all(
            managed_target_file
                .parent()
                .expect("managed target parent should exist"),
        )
        .expect("create managed target parent");
        fs::write(&managed_target_file, "not-a-directory").expect("write managed target file");

        let error =
            prepare_artifact_layout(repo_root.path(), CommandArtifactLayout::ManagedWorkspace)
                .expect_err("workspace layout should fail");
        assert!(
            error
                .to_string()
                .contains(&managed_target_file.display().to_string())
        );
    });
}

#[cfg(unix)]
#[test]
fn prepare_artifact_layout_reports_cachedir_marker_write_failures() {
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

        let managed_target_dir = repo_root.path().join(".managed-artifacts/target");
        fs::create_dir_all(&managed_target_dir).expect("create managed target dir");
        let original_permissions = fs::metadata(&managed_target_dir)
            .expect("managed target metadata")
            .permissions();
        let mut readonly_permissions = original_permissions.clone();
        readonly_permissions.set_mode(0o555);
        fs::set_permissions(&managed_target_dir, readonly_permissions)
            .expect("lock managed target dir");

        let error =
            prepare_artifact_layout(repo_root.path(), CommandArtifactLayout::ManagedWorkspace)
                .expect_err("workspace layout should fail");

        fs::set_permissions(&managed_target_dir, original_permissions)
            .expect("unlock managed target dir");

        assert!(
            error
                .to_string()
                .contains(&managed_target_dir.display().to_string())
        );
    });
}

#[cfg(unix)]
#[test]
fn prepare_artifact_layout_reports_manifest_write_failures() {
    let repo_root = tempdir().expect("repo tempdir");
    with_test_artifact_overrides(repo_root.path(), || {
        let cargo_config_dir = repo_root.path().join(".cargo");
        fs::create_dir_all(&cargo_config_dir).expect("create .cargo dir");
        fs::write(
            cargo_config_dir.join("config.toml"),
            "[build]\ntarget-dir = \".managed-artifacts/target\"\nbuild-dir = \".managed-artifacts/build\"\n",
        )
        .expect("write cargo config");

        let managed_target_dir = repo_root.path().join(".managed-artifacts/target");
        fs::create_dir_all(managed_target_dir.join(".htmlcut-artifact.toml"))
            .expect("create manifest path as directory");

        let error =
            prepare_artifact_layout(repo_root.path(), CommandArtifactLayout::ManagedWorkspace)
                .expect_err("workspace layout should fail");

        assert!(
            error.to_string().contains(
                &managed_target_dir
                    .join(".htmlcut-artifact.toml")
                    .display()
                    .to_string()
            )
        );
    });
}

#[cfg(unix)]
#[test]
fn prepare_artifact_layout_reports_coverage_root_creation_failures() {
    let repo_root = tempdir().expect("repo tempdir");
    with_test_artifact_overrides(repo_root.path(), || {
        let cargo_config_dir = repo_root.path().join(".cargo");
        fs::create_dir_all(&cargo_config_dir).expect("create .cargo dir");
        fs::write(
            cargo_config_dir.join("config.toml"),
            "[build]\ntarget-dir = \".managed-artifacts/target\"\nbuild-dir = \".managed-artifacts/build\"\n",
        )
        .expect("write cargo config");

        let managed_coverage_target_file =
            repo_root.path().join(".managed-artifacts/coverage-target");
        fs::create_dir_all(
            managed_coverage_target_file
                .parent()
                .expect("managed coverage target parent should exist"),
        )
        .expect("create managed coverage target parent");
        fs::write(&managed_coverage_target_file, "not-a-directory")
            .expect("write managed coverage target file");

        let error =
            prepare_artifact_layout(repo_root.path(), CommandArtifactLayout::ManagedCoverage)
                .expect_err("coverage layout should fail");
        assert!(
            error
                .to_string()
                .contains(&managed_coverage_target_file.display().to_string())
        );
    });
}

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
                .all(|path| path != &repo_root.path().join("target"))
        );
        assert!(!managed_workspace_target.exists());
        assert!(!managed_workspace_build.exists());
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

#[cfg(unix)]
#[test]
fn aggregate_entry_reports_member_read_failures_with_member_paths() {
    use std::os::unix::fs::PermissionsExt;

    let repo_root = tempdir().expect("repo tempdir");
    let aggregate_root = repo_root.path().join("aggregate");
    let aggregate_member = repo_root.path().join("aggregate-member");
    fs::create_dir_all(aggregate_member.join("debug")).expect("create aggregate member");
    fs::write(aggregate_member.join("debug/artifact"), "artifact").expect("write artifact");

    let original_permissions = fs::metadata(&aggregate_member)
        .expect("aggregate member metadata")
        .permissions();
    let mut unreadable_permissions = original_permissions.clone();
    unreadable_permissions.set_mode(0o000);
    fs::set_permissions(&aggregate_member, unreadable_permissions).expect("lock aggregate member");

    let error = crate::hygiene::aggregate_entry_for_tests(
        &aggregate_root,
        std::slice::from_ref(&aggregate_member),
    )
    .expect_err("aggregate entry should fail");

    fs::set_permissions(&aggregate_member, original_permissions).expect("unlock aggregate member");

    assert!(
        error
            .to_string()
            .contains(&aggregate_member.display().to_string())
    );
}

#[cfg(unix)]
#[test]
fn clean_hygiene_reports_removal_failures_with_artifact_paths() {
    use std::os::unix::fs::PermissionsExt;

    let repo_root = tempdir().expect("repo tempdir");
    let target_dir = repo_root.path().join("target/debug");
    fs::create_dir_all(&target_dir).expect("create legacy target");
    fs::write(target_dir.join("artifact"), "artifact").expect("write legacy artifact");

    let original_permissions = fs::metadata(repo_root.path())
        .expect("repo metadata")
        .permissions();
    let mut readonly_permissions = original_permissions.clone();
    readonly_permissions.set_mode(0o555);
    fs::set_permissions(repo_root.path(), readonly_permissions).expect("lock repo root");

    let error = clean_hygiene(repo_root.path(), HygieneCleanMode::Safe).expect_err("clean fails");

    fs::set_permissions(repo_root.path(), original_permissions).expect("unlock repo root");

    let message = error.to_string();
    assert!(message.contains("failed to remove hygiene artifact root"));
    assert!(message.contains(&repo_root.path().join("target").display().to_string()));
    assert!(repo_root.path().join("target").exists());
}

#[cfg(unix)]
#[test]
fn dir_size_helpers_ignore_symlinks_and_special_files() {
    use std::os::unix::fs::symlink;
    use std::os::unix::net::UnixListener;

    let repo_root = tempdir().expect("repo tempdir");
    let target_file = repo_root.path().join("artifact.txt");
    fs::write(&target_file, "artifact").expect("write file");
    let symlink_path = repo_root.path().join("artifact-link");
    symlink(&target_file, &symlink_path).expect("create symlink");
    let socket_path = repo_root.path().join("artifact.sock");
    let _listener = UnixListener::bind(&socket_path).expect("bind unix socket");

    assert_eq!(crate::hygiene::dir_size_bytes_for_tests(&symlink_path), 0);
    assert_eq!(crate::hygiene::dir_size_bytes_for_tests(&socket_path), 0);
}

#[cfg(unix)]
#[test]
fn dir_size_helpers_surface_metadata_failures_for_unreadable_parent_paths() {
    use std::os::unix::fs::PermissionsExt;

    let repo_root = tempdir().expect("repo tempdir");
    let restricted_root = repo_root.path().join("restricted");
    fs::create_dir_all(&restricted_root).expect("create restricted root");

    let original_permissions = fs::metadata(&restricted_root)
        .expect("restricted root metadata")
        .permissions();
    let mut unreadable_permissions = original_permissions.clone();
    unreadable_permissions.set_mode(0o000);
    fs::set_permissions(&restricted_root, unreadable_permissions).expect("lock restricted root");

    let error =
        crate::hygiene::dir_size_bytes_result_for_tests(&restricted_root.join("missing-child"))
            .expect_err("metadata lookup should fail");

    fs::set_permissions(&restricted_root, original_permissions).expect("unlock restricted root");

    assert!(
        error
            .to_string()
            .contains(&restricted_root.join("missing-child").display().to_string())
    );
}
