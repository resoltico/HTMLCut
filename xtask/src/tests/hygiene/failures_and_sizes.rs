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
fn aggregate_entry_sums_each_member_exactly_once() {
    let repo_root = tempdir().expect("repo tempdir");
    let first = repo_root.path().join("first");
    let second = repo_root.path().join("second");
    fs::create_dir_all(&first).expect("create first");
    fs::create_dir_all(&second).expect("create second");
    fs::write(first.join("artifact"), "abc").expect("write first artifact");
    fs::write(second.join("artifact"), "wxyz").expect("write second artifact");

    let entry = crate::hygiene::aggregate_entry_for_tests(
        repo_root.path(),
        &[first.clone(), second.clone()],
    )
    .expect("aggregate entry");

    assert_eq!(entry.bytes, 7);
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
fn directory_size_excludes_only_the_declared_nested_roots() {
    let repo_root = tempdir().expect("repo tempdir");
    let root = repo_root.path().join("root");
    let retained = root.join("retained");
    let skipped = root.join("skipped");
    fs::create_dir_all(&retained).expect("create retained directory");
    fs::create_dir_all(&skipped).expect("create skipped directory");
    fs::write(retained.join("artifact"), "abc").expect("write retained artifact");
    fs::write(skipped.join("artifact"), "wxyz").expect("write skipped artifact");

    assert_eq!(
        crate::hygiene::dir_size_bytes_excluding_roots_for_tests(
            &root,
            std::slice::from_ref(&skipped),
        )
        .expect("sized directory"),
        3
    );
}

#[test]
fn artifact_container_helpers_remove_files_directories_and_missing_paths() {
    let root = tempdir().expect("artifact container root");
    let container = root.path().join("container");
    fs::create_dir_all(&container).expect("create container");
    let unowned_file = container.join("unowned.log");
    let unowned_directory = container.join("unowned-directory");
    fs::write(&unowned_file, "temporary output").expect("write unowned file");
    fs::create_dir_all(&unowned_directory).expect("create unowned directory");

    let entry = crate::hygiene::unmanaged_artifact_container_entry_for_tests(&container, &[])
        .expect("unowned container entry");
    assert!(entry.present);
    assert_eq!(entry.details.len(), 2);
    crate::hygiene::remove_artifact_path_if_exists_for_tests(&unowned_file)
        .expect("remove unowned file");
    crate::hygiene::remove_artifact_path_if_exists_for_tests(&unowned_directory)
        .expect("remove unowned directory");
    crate::hygiene::remove_artifact_path_if_exists_for_tests(&container.join("missing"))
        .expect("missing unowned artifact is harmless");
    assert!(!unowned_file.exists());
    assert!(!unowned_directory.exists());
}

#[test]
fn aggregate_size_rejects_overflow() {
    let root = tempdir().expect("aggregate root");
    assert!(crate::hygiene::checked_aggregate_bytes_for_tests(u64::MAX, 1, root.path()).is_err());
}

#[test]
fn reclaimed_byte_accounting_rejects_overflow() {
    let root = tempdir().expect("artifact root");
    assert!(crate::hygiene::checked_reclaimed_bytes_for_tests(u64::MAX, 1, root.path()).is_err());
}

#[cfg(unix)]
#[test]
fn artifact_path_cleanup_reports_metadata_errors_below_an_inaccessible_parent() {
    use std::os::unix::fs::PermissionsExt;

    let root = tempdir().expect("artifact parent");
    let parent = root.path().join("parent");
    let artifact = parent.join("artifact");
    fs::create_dir_all(&artifact).expect("create artifact");
    let original_permissions = fs::metadata(&parent)
        .expect("parent metadata")
        .permissions();
    let mut inaccessible_permissions = original_permissions.clone();
    inaccessible_permissions.set_mode(0o000);
    fs::set_permissions(&parent, inaccessible_permissions).expect("lock parent");

    let error = crate::hygiene::remove_artifact_path_if_exists_for_tests(&artifact)
        .expect_err("metadata failure must surface");
    fs::set_permissions(&parent, original_permissions).expect("unlock parent");
    assert!(error.to_string().contains("Permission denied"));
}

#[cfg(unix)]
#[test]
fn unreadable_artifact_container_reports_the_container_path() {
    use std::os::unix::fs::PermissionsExt;

    let root = tempdir().expect("artifact container root");
    let container = root.path().join("container");
    fs::create_dir_all(&container).expect("create container");
    let original_root_permissions = fs::metadata(root.path())
        .expect("root metadata")
        .permissions();
    let mut inaccessible_root_permissions = original_root_permissions.clone();
    inaccessible_root_permissions.set_mode(0o000);
    fs::set_permissions(root.path(), inaccessible_root_permissions).expect("lock parent root");
    let managed_error = crate::hygiene::managed_artifact_container_entry_for_tests(&container)
        .expect_err("managed container below inaccessible parent");
    fs::set_permissions(root.path(), original_root_permissions).expect("unlock parent root");

    let original_container_permissions = fs::metadata(&container)
        .expect("container metadata")
        .permissions();
    let mut unreadable_container_permissions = original_container_permissions.clone();
    unreadable_container_permissions.set_mode(0o111);
    fs::set_permissions(&container, unreadable_container_permissions).expect("lock container");
    let unmanaged_error =
        crate::hygiene::unmanaged_artifact_container_entry_for_tests(&container, &[])
            .expect_err("unreadable unowned container");

    fs::set_permissions(&container, original_container_permissions).expect("unlock container");
    assert!(
        managed_error
            .to_string()
            .contains(&container.display().to_string())
    );
    assert!(
        unmanaged_error
            .to_string()
            .contains(&container.display().to_string())
    );
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
