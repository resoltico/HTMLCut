use super::*;

#[test]
fn source_helpers_cover_remaining_locator_paths() {
    let file_metadata = empty_source_metadata(
        &file_source("fixtures/input.html")
            .with_base_url(Url::parse("https://example.com/base/").expect("base")),
    );
    assert_eq!(file_metadata.value, "fixtures/input.html");
    assert_eq!(
        file_metadata.input_base_url.as_deref(),
        Some("https://example.com/base/")
    );

    let stdin_metadata = empty_source_metadata(&SourceRequest::stdin());
    assert_eq!(stdin_metadata.value, "-");
    assert_eq!(stdin_metadata.kind, SourceKind::Stdin);

    let unnamed_memory_metadata =
        empty_source_metadata(&SourceRequest::memory("   ", "<article>Hello</article>"));
    assert_eq!(unnamed_memory_metadata.value, "memory");
}
#[test]
fn read_file_source_reports_unreadable_inputs() {
    let tempdir = htmlcut_tempdir::tempdir().expect("tempdir");
    let unreadable_path = tempdir.path().join("unreadable");

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        std::fs::write(&unreadable_path, "<article>Hello</article>").expect("write html");

        let mut permissions = std::fs::metadata(&unreadable_path)
            .expect("metadata")
            .permissions();
        permissions.set_mode(0o000);
        std::fs::set_permissions(&unreadable_path, permissions).expect("chmod 000");
    }

    #[cfg(not(unix))]
    {
        std::fs::create_dir(&unreadable_path).expect("create unreadable directory placeholder");
    }

    let error = read_file_source(&file_source(&unreadable_path), &RuntimeOptions::default())
        .expect_err("unreadable input");
    assert_eq!(error.code, "SOURCE_LOAD_FAILED");
    assert!(error.message.contains("Could not read file"));
}
