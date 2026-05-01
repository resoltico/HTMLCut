use super::*;

struct CurrentDirGuard(PathBuf);

impl CurrentDirGuard {
    fn enter(path: &Path) -> Self {
        let previous = std::env::current_dir().expect("current dir");
        std::env::set_current_dir(path).expect("set current dir");
        Self(previous)
    }
}

impl Drop for CurrentDirGuard {
    fn drop(&mut self) {
        std::env::set_current_dir(&self.0).expect("restore current dir");
    }
}

#[test]
fn file_output_helpers_reject_blocked_parents_and_non_directory_targets() {
    let tempdir = tempdir().expect("tempdir");
    let blocked_parent = tempdir.path().join("blocked-parent");
    fs::write(&blocked_parent, "sentinel").expect("blocked parent");

    let output_parent_error = crate::file_output::validate_output_file_target(
        &blocked_parent.join("report.json"),
        crate::file_output::FileWriteMode::CreateFresh,
    )
    .expect_err("blocked output parent should fail");
    assert_eq!(output_parent_error.code, "CLI_OUTPUT_FILE_WRITE_FAILED");
    assert!(output_parent_error.message.contains("parent path"));

    let request_dir = tempdir.path().join("request-dir");
    fs::create_dir(&request_dir).expect("request dir");
    let request_target_error = crate::file_output::validate_request_file_target(
        &request_dir,
        crate::file_output::FileWriteMode::Overwrite,
    )
    .expect_err("directory target should fail");
    assert_eq!(request_target_error.code, "CLI_REQUEST_FILE_WRITE_FAILED");
    assert!(
        request_target_error
            .message
            .contains("target path is a directory")
    );

    let bundle_parent_error = crate::file_output::validate_bundle_target(
        &blocked_parent.join("bundle"),
        crate::file_output::FileWriteMode::CreateFresh,
    )
    .expect_err("blocked bundle parent should fail");
    assert_eq!(
        bundle_parent_error.code,
        "CLI_BUNDLE_DIRECTORY_CREATE_FAILED"
    );
    assert!(bundle_parent_error.message.contains("parent path"));

    let bundle_file = tempdir.path().join("bundle-file");
    fs::write(&bundle_file, "sentinel").expect("bundle file");
    let bundle_target_error = crate::file_output::validate_bundle_target(
        &bundle_file,
        crate::file_output::FileWriteMode::Overwrite,
    )
    .expect_err("bundle file should fail");
    assert_eq!(
        bundle_target_error.code,
        "CLI_BUNDLE_DIRECTORY_CREATE_FAILED"
    );
    assert!(
        bundle_target_error
            .message
            .contains("target path is not a directory")
    );
}

#[test]
fn file_output_helpers_prepare_nested_bundle_directories_for_overwrite_mode() {
    let tempdir = tempdir().expect("tempdir");
    let nested_bundle = tempdir.path().join("nested bundle/root/output-bundle");
    crate::file_output::prepare_bundle_directory(
        &nested_bundle,
        crate::file_output::FileWriteMode::Overwrite,
    )
    .expect("fresh overwrite bundle dir");
    assert!(nested_bundle.is_dir());

    let bundle_file = tempdir.path().join("existing-bundle-file");
    fs::write(&bundle_file, "sentinel").expect("bundle file");
    let error = crate::file_output::prepare_bundle_directory(
        &bundle_file,
        crate::file_output::FileWriteMode::Overwrite,
    )
    .expect_err("overwrite bundle path should reject files");
    assert_eq!(error.kind(), std::io::ErrorKind::AlreadyExists);

    let blocked_parent = tempdir.path().join("blocked-parent");
    fs::write(&blocked_parent, "sentinel").expect("blocked parent");
    let blocked_error = crate::file_output::prepare_bundle_directory(
        &blocked_parent.join("bundle"),
        crate::file_output::FileWriteMode::CreateFresh,
    )
    .expect_err("blocked parent should fail");
    assert_eq!(blocked_error.kind(), std::io::ErrorKind::AlreadyExists);
}

#[test]
fn file_output_helpers_prepare_parentless_and_existing_bundle_directories() {
    let tempdir = tempdir().expect("tempdir");
    let _guard = CurrentDirGuard::enter(tempdir.path());
    let bundle_dir = Path::new("bundle-dir");
    let output_file = Path::new("report.json");

    crate::file_output::validate_bundle_target(
        bundle_dir,
        crate::file_output::FileWriteMode::CreateFresh,
    )
    .expect("parentless bundle target is valid");
    crate::file_output::validate_output_file_target(
        output_file,
        crate::file_output::FileWriteMode::CreateFresh,
    )
    .expect("parentless file target is valid");

    let nested_bundle = tempdir.path().join("missing-parent").join("bundle-dir");
    crate::file_output::validate_bundle_target(
        &nested_bundle,
        crate::file_output::FileWriteMode::CreateFresh,
    )
    .expect("missing parent bundle target is valid before creation");

    crate::file_output::prepare_bundle_directory(
        bundle_dir,
        crate::file_output::FileWriteMode::Overwrite,
    )
    .expect("parentless overwrite bundle dir");
    assert!(bundle_dir.is_dir());

    crate::file_output::prepare_bundle_directory(
        bundle_dir,
        crate::file_output::FileWriteMode::Overwrite,
    )
    .expect("existing bundle directory remains reusable in overwrite mode");
}
