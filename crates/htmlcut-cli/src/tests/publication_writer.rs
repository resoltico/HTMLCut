use super::*;

#[test]
fn staging_write_flush_and_sync_faults_are_fatal() {
    struct Fault {
        stage: u8,
    }
    impl Write for Fault {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            if self.stage == 0 {
                Err(io::Error::other("write fault"))
            } else {
                Ok(bytes.len())
            }
        }
        fn flush(&mut self) -> io::Result<()> {
            if self.stage == 1 {
                Err(io::Error::other("flush fault"))
            } else {
                Ok(())
            }
        }
    }
    for stage in 0..=2 {
        let error = write_staged_bytes(&mut Fault { stage }, b"payload", |_| {
            Err(io::Error::other("sync fault"))
        })
        .unwrap_err();
        assert_eq!(error.code, ErrorCode::Publication);
    }
}

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

#[test]
fn audit_default_one_mebibyte_bound_includes_json_framing_exactly() {
    use crate::command::AuditField;
    use htmlcut_core::{
        CompiledPlan, ExtractionPlan, PreparedDocument, SnapshotMetadata, SourceSnapshot,
    };
    let plan = CompiledPlan::compile(&ExtractionPlan::css("p").unwrap()).unwrap();
    let framing = b"{\"values\":[\"\"]}\n".len();
    for (total, accepted) in [(1_048_575, true), (1_048_576, true), (1_048_577, false)] {
        let value = "x".repeat(total - framing);
        let source = PreparedDocument::new(
            SourceSnapshot::new(format!("<p>{value}</p>"), SnapshotMetadata::default()).unwrap(),
            Default::default(),
        )
        .unwrap();
        let result = source.execute(&plan).unwrap();
        let evidence = crate::evidence::Evidence {
            fields: &[AuditField::Values],
            result: &result,
            plan: plan.plan(),
        };
        let output = json_stream(&evidence, MAX_AUDIT_BYTES);
        if accepted {
            assert_eq!(output.unwrap().len(), total);
        } else {
            assert_eq!(output.unwrap_err().code, ErrorCode::ResourceLimit);
        }
    }
}

#[test]
fn io_recovery_categories_keep_operations_and_remove_raw_messages() {
    use htmlcut_core::{FailureCause, IoOperation, IoProblem};
    for (kind, problem) in [
        (
            std::io::ErrorKind::PermissionDenied,
            IoProblem::PermissionDenied,
        ),
        (std::io::ErrorKind::NotFound, IoProblem::NotFound),
        (std::io::ErrorKind::BrokenPipe, IoProblem::BrokenPipe),
        (std::io::ErrorKind::Other, IoProblem::Other),
    ] {
        let error = super::io_failure(
            std::io::Error::new(kind, "SYNTHETIC_SECRET"),
            IoOperation::Stdout,
        );
        assert_eq!(
            error.evidence.cause,
            Some(FailureCause::Io {
                operation: IoOperation::Stdout,
                problem
            })
        );
        assert!(
            !serde_json::to_string(&error)
                .unwrap()
                .contains("SYNTHETIC_SECRET")
        );
        let error = crate::input::io_failure(std::io::Error::new(kind, "SYNTHETIC_SECRET"));
        assert_eq!(
            error.evidence.cause,
            Some(FailureCause::Io {
                operation: IoOperation::Input,
                problem
            })
        );
        assert!(
            !serde_json::to_string(&error)
                .unwrap()
                .contains("SYNTHETIC_SECRET")
        );
    }
}

#[test]
fn bare_output_names_resolve_in_the_actual_working_directory() {
    let name = format!("htmlcut-output-boundary-{}.json", std::process::id());
    assert_eq!(
        super::normalized_target(std::path::Path::new(&name)).unwrap(),
        std::env::current_dir().unwrap().join(name),
    );
}
