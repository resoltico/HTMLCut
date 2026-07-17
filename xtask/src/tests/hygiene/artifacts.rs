use super::*;

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
        let gate_reports =
            prepare_gate_report_root(repo_root.path()).expect("prepare gate reports");

        for path in [
            workspace_target,
            workspace_build,
            coverage_target,
            coverage_build,
            coverage_cargo_target,
            coverage_cargo_build,
            gate_reports,
        ] {
            assert!(path.is_dir(), "{} should exist", path.display());
            assert!(path.join("CACHEDIR.TAG").is_file());
            assert!(path.join(".htmlcut-artifact.toml").is_file());
        }
    });
}

#[test]
fn prepare_gate_report_root_rejects_a_non_directory_evidence_root() {
    let repo_root = tempdir().expect("repo tempdir");
    with_test_artifact_overrides(repo_root.path(), || {
        let root = gate_report_dir(repo_root.path());
        fs::create_dir_all(root.parent().expect("gate report parent")).expect("create parent");
        fs::write(&root, "not a directory").expect("block report root");

        let error =
            prepare_gate_report_root(repo_root.path()).expect_err("report root should fail");
        assert!(error.to_string().contains(&root.display().to_string()));
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
