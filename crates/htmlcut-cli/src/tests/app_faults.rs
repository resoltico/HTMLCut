use super::*;

#[test]
fn bounded_diagnostics_and_metadata_do_not_publish_prefixes() {
    let mut stderr = Vec::new();
    let mut error = options("bounded diagnostic");
    error.message = "synthetic private diagnostic".repeat(500);
    assert_eq!(report(error, &mut stderr), 5);
    assert!(stderr.is_empty());
    let metadata = crate::operation_metadata::describe(None).unwrap();
    let complete = crate::publication::json(&metadata, 16 * 1024).unwrap();
    for maximum in [0, complete.len() - 1, complete.len(), complete.len() + 1] {
        let mut stdout = Vec::new();
        let result = emit_json(&metadata, maximum, &mut stdout);
        if maximum < complete.len() {
            assert_eq!(
                result.unwrap_err().code,
                htmlcut_core::ErrorCode::ResourceLimit
            );
            assert!(stdout.is_empty());
        } else {
            result.unwrap();
            assert_eq!(stdout, complete);
        }
    }
}
