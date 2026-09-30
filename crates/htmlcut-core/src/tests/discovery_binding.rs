use super::*;

#[test]
fn invalid_page_sizes_are_rejected_even_for_a_correctly_bound_handle() {
    let source =
        crate::SourceSnapshot::new("<p>value</p>", crate::SnapshotMetadata::default()).unwrap();
    let document = PreparedDocument::new(source, crate::PreparationLimits::default()).unwrap();
    for size in [0, 101] {
        let handle = token(document.prepared_sha256(), &binding(size), 3, "handle");
        assert_eq!(
            document.propose(&handle, size).unwrap_err().code,
            crate::ErrorCode::InvalidOptions
        );
    }
}
