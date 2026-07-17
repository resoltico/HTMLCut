use super::*;

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
