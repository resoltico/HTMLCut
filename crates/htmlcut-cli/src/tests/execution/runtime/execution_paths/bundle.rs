use super::*;

#[test]
fn write_bundle_reports_each_output_failure() {
    let report = build_extraction_report(
        "select",
        fixture_result(Value::String("Hello".to_owned()), ValueType::Text),
        None,
    );

    let create_dir_temp = tempdir().expect("tempdir");
    let create_dir_path = create_dir_temp.path().join("bundle");
    fs::write(&create_dir_path, "file").expect("write file");
    assert_eq!(
        write_bundle(
            &report,
            &BundlePaths {
                dir: create_dir_path.to_string_lossy().into_owned(),
                html: create_dir_path
                    .join("selection.html")
                    .to_string_lossy()
                    .into_owned(),
                text: create_dir_path
                    .join("selection.txt")
                    .to_string_lossy()
                    .into_owned(),
                json: create_dir_path
                    .join("selection.json")
                    .to_string_lossy()
                    .into_owned(),
                report: create_dir_path
                    .join("report.json")
                    .to_string_lossy()
                    .into_owned(),
            },
        )
        .expect_err("directory creation should fail")
        .code,
        "CLI_BUNDLE_PATH_EXISTS"
    );

    assert_eq!(
        crate::render::write_bundle(
            &report,
            &BundlePaths {
                dir: create_dir_path.to_string_lossy().into_owned(),
                html: create_dir_path
                    .join("selection.html")
                    .to_string_lossy()
                    .into_owned(),
                text: create_dir_path
                    .join("selection.txt")
                    .to_string_lossy()
                    .into_owned(),
                json: create_dir_path
                    .join("selection.json")
                    .to_string_lossy()
                    .into_owned(),
                report: create_dir_path
                    .join("report.json")
                    .to_string_lossy()
                    .into_owned(),
            },
            crate::file_output::FileWriteMode::Overwrite,
        )
        .expect_err("overwrite should still reject non-directory bundle targets")
        .code,
        "CLI_BUNDLE_DIRECTORY_CREATE_FAILED"
    );

    let blocked_parent = create_dir_temp.path().join("blocked-parent");
    fs::write(&blocked_parent, "sentinel").expect("blocked parent");
    assert_eq!(
        crate::render::write_bundle(
            &report,
            &BundlePaths {
                dir: blocked_parent.join("bundle").to_string_lossy().into_owned(),
                html: blocked_parent
                    .join("bundle")
                    .join("selection.html")
                    .to_string_lossy()
                    .into_owned(),
                text: blocked_parent
                    .join("bundle")
                    .join("selection.txt")
                    .to_string_lossy()
                    .into_owned(),
                json: blocked_parent
                    .join("bundle")
                    .join("selection.json")
                    .to_string_lossy()
                    .into_owned(),
                report: blocked_parent
                    .join("bundle")
                    .join("report.json")
                    .to_string_lossy()
                    .into_owned(),
            },
            crate::file_output::FileWriteMode::CreateFresh,
        )
        .expect_err("blocked parent should reject bundle creation")
        .code,
        "CLI_BUNDLE_DIRECTORY_CREATE_FAILED"
    );

    let html_temp = tempdir().expect("tempdir");
    let existing_bundle = html_temp.path().join("existing-bundle");
    fs::create_dir(&existing_bundle).expect("existing bundle dir");
    assert_eq!(
        write_bundle(
            &report,
            &BundlePaths {
                dir: existing_bundle.to_string_lossy().into_owned(),
                html: existing_bundle
                    .join("selection.html")
                    .to_string_lossy()
                    .into_owned(),
                text: existing_bundle
                    .join("selection.txt")
                    .to_string_lossy()
                    .into_owned(),
                json: existing_bundle
                    .join("selection.json")
                    .to_string_lossy()
                    .into_owned(),
                report: existing_bundle
                    .join("report.json")
                    .to_string_lossy()
                    .into_owned(),
            },
        )
        .expect_err("existing bundle dir should require overwrite")
        .code,
        "CLI_BUNDLE_PATH_EXISTS"
    );

    fs::create_dir(html_temp.path().join("selection.html")).expect("html dir");
    assert_eq!(
        crate::render::write_bundle(
            &report,
            &BundlePaths {
                dir: html_temp.path().to_string_lossy().into_owned(),
                html: html_temp
                    .path()
                    .join("selection.html")
                    .to_string_lossy()
                    .into_owned(),
                text: html_temp
                    .path()
                    .join("selection.txt")
                    .to_string_lossy()
                    .into_owned(),
                json: html_temp
                    .path()
                    .join("selection.json")
                    .to_string_lossy()
                    .into_owned(),
                report: html_temp
                    .path()
                    .join("report.json")
                    .to_string_lossy()
                    .into_owned(),
            },
            crate::file_output::FileWriteMode::Overwrite,
        )
        .expect_err("html write should fail")
        .code,
        "CLI_BUNDLE_HTML_WRITE_FAILED"
    );

    let text_temp = tempdir().expect("tempdir");
    fs::create_dir(text_temp.path().join("selection.txt")).expect("text dir");
    assert_eq!(
        crate::render::write_bundle(
            &report,
            &BundlePaths {
                dir: text_temp.path().to_string_lossy().into_owned(),
                html: text_temp
                    .path()
                    .join("selection.html")
                    .to_string_lossy()
                    .into_owned(),
                text: text_temp
                    .path()
                    .join("selection.txt")
                    .to_string_lossy()
                    .into_owned(),
                json: text_temp
                    .path()
                    .join("selection.json")
                    .to_string_lossy()
                    .into_owned(),
                report: text_temp
                    .path()
                    .join("report.json")
                    .to_string_lossy()
                    .into_owned(),
            },
            crate::file_output::FileWriteMode::Overwrite,
        )
        .expect_err("text write should fail")
        .code,
        "CLI_BUNDLE_TEXT_WRITE_FAILED"
    );

    let json_temp = tempdir().expect("tempdir");
    fs::create_dir(json_temp.path().join("selection.json")).expect("json dir");
    assert_eq!(
        crate::render::write_bundle(
            &report,
            &BundlePaths {
                dir: json_temp.path().to_string_lossy().into_owned(),
                html: json_temp
                    .path()
                    .join("selection.html")
                    .to_string_lossy()
                    .into_owned(),
                text: json_temp
                    .path()
                    .join("selection.txt")
                    .to_string_lossy()
                    .into_owned(),
                json: json_temp
                    .path()
                    .join("selection.json")
                    .to_string_lossy()
                    .into_owned(),
                report: json_temp
                    .path()
                    .join("report.json")
                    .to_string_lossy()
                    .into_owned(),
            },
            crate::file_output::FileWriteMode::Overwrite,
        )
        .expect_err("json write should fail")
        .code,
        "CLI_BUNDLE_JSON_WRITE_FAILED"
    );

    let report_temp = tempdir().expect("tempdir");
    fs::create_dir(report_temp.path().join("report.json")).expect("report dir");
    assert_eq!(
        crate::render::write_bundle(
            &report,
            &BundlePaths {
                dir: report_temp.path().to_string_lossy().into_owned(),
                html: report_temp
                    .path()
                    .join("selection.html")
                    .to_string_lossy()
                    .into_owned(),
                text: report_temp
                    .path()
                    .join("selection.txt")
                    .to_string_lossy()
                    .into_owned(),
                json: report_temp
                    .path()
                    .join("selection.json")
                    .to_string_lossy()
                    .into_owned(),
                report: report_temp
                    .path()
                    .join("report.json")
                    .to_string_lossy()
                    .into_owned(),
            },
            crate::file_output::FileWriteMode::Overwrite,
        )
        .expect_err("report write should fail")
        .code,
        "CLI_BUNDLE_REPORT_WRITE_FAILED"
    );
}
