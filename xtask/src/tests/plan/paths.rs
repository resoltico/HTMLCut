use super::*;

#[test]
fn shell_script_paths_returns_sorted_shell_scripts_only() {
    let repo_root = tempdir().expect("tempdir");
    let scripts_dir = repo_root.path().join("scripts");
    fs::create_dir_all(&scripts_dir).expect("create scripts dir");
    fs::write(repo_root.path().join("check.sh"), "#!/usr/bin/env bash\n").expect("write check.sh");
    fs::write(scripts_dir.join("b.sh"), "#!/usr/bin/env bash\n").expect("write b.sh");
    fs::write(scripts_dir.join("a.sh"), "#!/usr/bin/env bash\n").expect("write a.sh");
    fs::write(scripts_dir.join("note.txt"), "ignore").expect("write note.txt");

    let scripts = shell_script_paths(repo_root.path()).expect("script paths");

    assert_eq!(
        scripts,
        vec![
            repo_root.path().join("check.sh"),
            scripts_dir.join("a.sh"),
            scripts_dir.join("b.sh"),
        ]
    );
}

#[test]
fn shell_script_paths_returns_empty_when_scripts_dir_is_missing() {
    let repo_root = tempdir().expect("tempdir");

    let scripts = shell_script_paths(repo_root.path()).expect("script paths");

    assert!(scripts.is_empty());
}

#[test]
fn cargo_target_dir_prefers_explicit_environment_over_repo_config() {
    let repo_root = tempdir().expect("tempdir");
    let env_target = Path::new("tmp/managed-target");
    let config_target = Path::new("../.managed-artifacts/target");

    let resolved = crate::plan::cargo_target_dir_from_sources_for_tests(
        repo_root.path(),
        Some(env_target),
        Some(config_target),
    );

    assert_eq!(resolved, repo_root.path().join(env_target));
}

#[test]
fn cargo_build_dir_prefers_explicit_environment_over_repo_config() {
    let repo_root = tempdir().expect("tempdir");
    let env_target = Path::new("tmp/managed-target");
    let config_target = Path::new("../.managed-artifacts/target");
    let env_build = Path::new("tmp/managed-build");
    let config_build = Path::new("../.managed-artifacts/build");

    let resolved = crate::plan::cargo_build_dir_from_sources_for_tests(
        repo_root.path(),
        Some(env_target),
        Some(config_target),
        Some(env_build),
        Some(config_build),
    );

    assert_eq!(resolved, repo_root.path().join(env_build));
}

#[test]
fn cargo_build_dir_follows_environment_target_when_no_build_dir_override_exists() {
    let repo_root = tempdir().expect("tempdir");
    let env_target = Path::new("tmp/managed-target");
    let config_target = Path::new("../.managed-artifacts/target");
    let config_build = Path::new("../.managed-artifacts/build");

    let resolved = crate::plan::cargo_build_dir_from_sources_for_tests(
        repo_root.path(),
        Some(env_target),
        Some(config_target),
        None,
        Some(config_build),
    );

    assert_eq!(resolved, repo_root.path().join(config_build));
}

#[test]
fn cargo_path_helpers_can_opt_into_process_env_lookup_without_changing_defaults() {
    let repo_root = tempdir().expect("tempdir");
    let expected_target = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| repo_root.path().join("target"));
    let expected_build = std::env::var_os("CARGO_BUILD_BUILD_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| expected_target.clone());

    crate::plan::with_process_env_passthrough_for_tests(|| {
        assert_eq!(cargo_target_dir(repo_root.path()), expected_target);
        assert_eq!(cargo_build_dir(repo_root.path()), expected_build);
    });
}

#[test]
fn shell_script_paths_use_git_inventory_when_available() {
    let repo_root = tempdir().expect("tempdir");
    fs::write(repo_root.path().join(".git"), "gitdir: /tmp/htmlcut.git\n").expect("write .git");
    fs::write(repo_root.path().join("check.sh"), "#!/usr/bin/env bash\n").expect("write check.sh");
    let scripts_dir = repo_root.path().join("scripts");
    fs::create_dir_all(&scripts_dir).expect("create scripts dir");
    fs::write(
        scripts_dir.join("release-targets.sh"),
        "#!/usr/bin/env bash\n",
    )
    .expect("write release-targets.sh");
    fs::write(scripts_dir.join("local-only.sh"), "#!/usr/bin/env bash\n")
        .expect("write local-only.sh");

    let scripts = crate::command_exec::with_capture_command_output_override(
        |_, spec| {
            (spec.program == std::path::Path::new("git"))
                .then(|| Ok(b"check.sh\0scripts/release-targets.sh\0".to_vec()))
        },
        || shell_script_paths(repo_root.path()),
    )
    .expect("script paths");

    assert_eq!(
        scripts,
        vec![
            repo_root.path().join("check.sh"),
            scripts_dir.join("release-targets.sh"),
        ]
    );
}

#[test]
fn is_maintained_shell_script_rejects_paths_outside_the_repo_root() {
    let repo_root = tempdir().expect("repo tempdir");
    let outside_root = tempdir().expect("outside tempdir");
    let inside_check = repo_root.path().join("check.sh");
    let inside_script = repo_root.path().join("scripts").join("release-targets.sh");
    let nested_script = repo_root
        .path()
        .join("scripts")
        .join("nested")
        .join("release-targets.sh");
    let non_shell_note = repo_root.path().join("scripts").join("notes.txt");
    let outside_script = outside_root.path().join("check.sh");
    fs::create_dir_all(inside_script.parent().expect("parent")).expect("create scripts dir");
    fs::create_dir_all(nested_script.parent().expect("nested parent"))
        .expect("create nested scripts dir");
    fs::write(&inside_check, "#!/usr/bin/env bash\n").expect("write inside check");
    fs::write(&inside_script, "#!/usr/bin/env bash\n").expect("write inside script");
    fs::write(&nested_script, "#!/usr/bin/env bash\n").expect("write nested script");
    fs::write(&non_shell_note, "ignore").expect("write note");
    fs::write(&outside_script, "#!/usr/bin/env bash\n").expect("write outside script");

    assert!(crate::plan::is_maintained_shell_script_for_tests(
        repo_root.path(),
        &inside_check
    ));
    assert!(crate::plan::is_maintained_shell_script_for_tests(
        repo_root.path(),
        &inside_script
    ));
    assert!(!crate::plan::is_maintained_shell_script_for_tests(
        repo_root.path(),
        &nested_script
    ));
    assert!(!crate::plan::is_maintained_shell_script_for_tests(
        repo_root.path(),
        &non_shell_note
    ));
    assert!(!crate::plan::is_maintained_shell_script_for_tests(
        repo_root.path(),
        &outside_script
    ));
}
