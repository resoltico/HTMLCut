#[test]
fn slice_cursor_progress_rejects_a_non_advancing_matcher() {
    assert_eq!(
        crate::extract::slice_cursor_progress_for_tests(3, 4).expect("advancing cursor"),
        4
    );
    let error = crate::extract::slice_cursor_progress_for_tests(3, 3)
        .expect_err("non-advancing cursor must fail");
    assert_eq!(error.code, "NO_MATCH");
    assert!(error.message.contains("did not advance"));
}
