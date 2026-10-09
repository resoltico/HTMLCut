// SPDX-License-Identifier: MPL-2.0
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
        let error = finish_staged_writer(
            &mut Fault { stage },
            |writer| write_staged_bytes(writer, b"payload"),
            |_| Err(io::Error::other("sync fault")),
        )
        .unwrap_err();
        assert_eq!(error.code, ErrorCode::Publication);
    }
}

#[test]
fn publication_buffer_flush_never_resets_serialization_capacity() {
    let mut buffer = Buffer {
        bytes: Vec::new(),
        maximum: 4,
        limit_exceeded: false,
    };
    buffer.write_all("éé".as_bytes()).unwrap();
    buffer.flush().unwrap();
    assert!(buffer.write_all(b"x").is_err());
    assert_eq!(buffer.bytes, "éé".as_bytes());
}

#[test]
fn data_encoding_is_complete_or_error_at_each_serialization_boundary() {
    use htmlcut_core::{CompiledPlan, ExtractionPlan, PreparedDocument, SourceSnapshot};
    let plan = CompiledPlan::compile(&ExtractionPlan::css("p").unwrap()).unwrap();
    let source = PreparedDocument::new(
        SourceSnapshot::new("<p>private body</p>", Default::default()).unwrap(),
        Default::default(),
    )
    .unwrap();
    let result = source.execute(&plan).unwrap();
    let expected = json_stream(result.data(), 4096).unwrap();
    assert_eq!(expected, b"[\"private body\"]\n");
    for maximum in 0..=expected.len() + 1 {
        let actual = json_stream(result.data(), maximum);
        if maximum < expected.len() {
            assert_eq!(actual.unwrap_err().code, ErrorCode::ResourceLimit);
        } else {
            assert_eq!(actual.unwrap(), expected);
        }
    }
}

#[test]
fn complete_data_payload_and_framing_have_independent_exact_bounds() {
    let value = serde_json::json!(["é\n\""]);
    let payload = json_payload(&value, 100).unwrap();
    let framed = json_stream(&value, 100).unwrap();
    assert_eq!(framed.len(), payload.len() + 1);
    assert_eq!(framed.last(), Some(&b'\n'));
    for size in [payload.len() - 1, payload.len(), payload.len() + 1] {
        assert_eq!(json_payload(&value, size).is_ok(), size >= payload.len());
        assert_eq!(json_stream(&value, size).is_ok(), size >= framed.len());
    }
    struct Fault;
    impl serde::Serialize for Fault {
        fn serialize<S: serde::Serializer>(&self, _: S) -> Result<S::Ok, S::Error> {
            Err(serde::ser::Error::custom("SECRET"))
        }
    }
    assert_eq!(
        json_payload(&Fault, 100).unwrap_err().code,
        ErrorCode::InternalInvariant
    );
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
        (std::io::ErrorKind::AlreadyExists, IoProblem::AlreadyExists),
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
        std::fs::canonicalize(std::env::current_dir().unwrap())
            .unwrap()
            .join(name),
    );
}

#[test]
fn failed_commit_preserves_the_filesystem_cause_and_allows_repair() {
    use htmlcut_core::{FailureCause, IoOperation, IoProblem};
    let root = tempfile::tempdir().unwrap();
    let parent = root.path().join("publication-parent");
    std::fs::create_dir(&parent).unwrap();
    let target = parent.join("private-result");
    let staged = Staged::prepare(&target, b"new", false).unwrap();
    // Both fixtures reach tempfile's PersistError after successful staging.
    // Windows locks the parent while the staging handle is open: create a competing
    // destination there instead. POSIX permits moving the parent to make it absent.
    #[cfg(windows)]
    std::fs::write(&target, b"old").unwrap();
    #[cfg(not(windows))]
    let moved = root.path().join("moved-parent");
    #[cfg(not(windows))]
    std::fs::rename(&parent, &moved).unwrap();
    let error = staged.commit().unwrap_err();
    assert_eq!(error.code, ErrorCode::Publication);
    assert_eq!(
        error.evidence.cause,
        Some(FailureCause::Io {
            operation: IoOperation::Publication,
            problem: if cfg!(windows) {
                IoProblem::AlreadyExists
            } else {
                IoProblem::NotFound
            },
        })
    );
    assert!(
        !serde_json::to_string(&error)
            .unwrap()
            .contains("private-result")
    );
    #[cfg(windows)]
    assert_eq!(std::fs::read(&target).unwrap(), b"old");
    #[cfg(not(windows))]
    assert!(!target.exists());
    // The moved staging path is intentionally retained here; tempfile cannot know a renamed
    // directory's location, and the enclosing temporary directory removes it after the test.
    #[cfg(not(windows))]
    std::fs::rename(&moved, &parent).unwrap();
    Staged::prepare(&target, b"repaired", cfg!(windows))
        .unwrap()
        .commit()
        .unwrap();
    assert_eq!(std::fs::read(&target).unwrap(), b"repaired");
}
