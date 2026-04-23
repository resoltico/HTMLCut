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
                report: create_dir_path
                    .join("report.json")
                    .to_string_lossy()
                    .into_owned(),
            },
        )
        .expect_err("directory creation should fail")
        .code,
        "CLI_BUNDLE_DIRECTORY_CREATE_FAILED"
    );

    let html_temp = tempdir().expect("tempdir");
    fs::create_dir(html_temp.path().join("selection.html")).expect("html dir");
    assert_eq!(
        write_bundle(
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
                report: html_temp
                    .path()
                    .join("report.json")
                    .to_string_lossy()
                    .into_owned(),
            },
        )
        .expect_err("html write should fail")
        .code,
        "CLI_BUNDLE_HTML_WRITE_FAILED"
    );

    let text_temp = tempdir().expect("tempdir");
    fs::create_dir(text_temp.path().join("selection.txt")).expect("text dir");
    assert_eq!(
        write_bundle(
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
                report: text_temp
                    .path()
                    .join("report.json")
                    .to_string_lossy()
                    .into_owned(),
            },
        )
        .expect_err("text write should fail")
        .code,
        "CLI_BUNDLE_TEXT_WRITE_FAILED"
    );

    let report_temp = tempdir().expect("tempdir");
    fs::create_dir(report_temp.path().join("report.json")).expect("report dir");
    assert_eq!(
        write_bundle(
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
                report: report_temp
                    .path()
                    .join("report.json")
                    .to_string_lossy()
                    .into_owned(),
            },
        )
        .expect_err("report write should fail")
        .code,
        "CLI_BUNDLE_REPORT_WRITE_FAILED"
    );
}
