use super::*;

#[test]
fn publication_buffer_flush_never_resets_serialization_capacity() {
    let mut buffer = Buffer {
        bytes: Vec::new(),
        maximum: 4,
    };
    buffer.write_all("éé".as_bytes()).unwrap();
    buffer.flush().unwrap();
    assert!(buffer.write_all(b"x").is_err());
    assert_eq!(buffer.bytes, "éé".as_bytes());
}

#[test]
fn counts_only_audit_is_complete_or_error_at_every_serialization_boundary() {
    use crate::command::AuditField;
    use htmlcut_core::{
        CompiledPlan, ExtractionPlan, PreparedDocument, SnapshotMetadata, SourceSnapshot,
    };
    let plan = CompiledPlan::compile(&ExtractionPlan::css("p").unwrap()).unwrap();
    let source = PreparedDocument::new(
        SourceSnapshot::new("<p>private body</p>", SnapshotMetadata::default()).unwrap(),
        Default::default(),
    )
    .unwrap();
    let result = source.execute(&plan).unwrap();
    let evidence = crate::evidence::Evidence {
        fields: &[AuditField::Counts],
        result: &result,
        plan: plan.plan(),
    };
    let expected = json_stream(&evidence, 1024).unwrap();
    let value: serde_json::Value = serde_json::from_slice(&expected).unwrap();
    assert_eq!(
        value["counts"],
        serde_json::json!({"candidates":1,"selected":1})
    );
    assert!(!String::from_utf8_lossy(&expected).contains("private body"));
    for maximum in 0..=expected.len() + 1 {
        let actual = json_stream(&evidence, maximum);
        if maximum < expected.len() {
            assert_eq!(actual.unwrap_err().code, ErrorCode::ResourceLimit);
        } else {
            assert_eq!(actual.unwrap(), expected);
        }
    }
}
