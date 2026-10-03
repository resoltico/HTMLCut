//! Regular files and intentional streams supply exact UTF-8 snapshots; acquisition is caller-owned.

use htmlcut_core::{ErrorCode, ExtractionError, SnapshotMetadata, SourceSnapshot};
use std::{fs::File, io::Read, path::Path};

#[path = "input/regular_file.rs"]
mod regular_file;

pub(crate) const MAX_SOURCE_BYTES: usize = htmlcut_core::MAX_SOURCE_BYTES;
pub(crate) const MAX_CONFIG_BYTES: usize = htmlcut_core::MAX_PLAN_BYTES;

pub(crate) fn options(message: &'static str) -> ExtractionError {
    ExtractionError::new(ErrorCode::InvalidOptions, "options", message).with_cause(
        htmlcut_core::FailureCause::Configuration {
            role: htmlcut_core::ConfigurationRole::Arguments,
            problem: htmlcut_core::ConfigurationProblem::InvalidValue,
        },
    )
}

pub(crate) fn acquisition() -> ExtractionError {
    ExtractionError::new(
        ErrorCode::Acquisition,
        "acquisition",
        "The source could not be acquired.",
    )
}

pub(crate) fn open_file(path: &Path) -> Result<File, ExtractionError> {
    regular_file::open(path)
}

pub(crate) fn snapshot(
    file: Option<&Path>,
    stdin: &mut dyn Read,
    base: Option<&str>,
) -> Result<SourceSnapshot, ExtractionError> {
    // Reject invalid metadata before consuming an intentional input stream.
    let metadata = SourceSnapshot::new(
        "",
        SnapshotMetadata {
            base_url: base.map(str::to_owned),
        },
    )?
    .metadata()
    .clone();
    let bytes = match file {
        Some(path) => read_file(path, MAX_SOURCE_BYTES)?,
        None => read_bounded(stdin, MAX_SOURCE_BYTES)?,
    };
    let text = String::from_utf8(bytes).map_err(|_| {
        ExtractionError::new(
            ErrorCode::Decoding,
            "decoding",
            "Snapshot bytes must be valid UTF-8.",
        )
    })?;
    SourceSnapshot::new(text, metadata)
}

pub(crate) fn io_failure(error: std::io::Error) -> ExtractionError {
    io_failure_ref(&error)
}

pub(crate) fn io_failure_ref(error: &std::io::Error) -> ExtractionError {
    let problem = match error.kind() {
        std::io::ErrorKind::NotFound => htmlcut_core::IoProblem::NotFound,
        std::io::ErrorKind::PermissionDenied => htmlcut_core::IoProblem::PermissionDenied,
        std::io::ErrorKind::BrokenPipe => htmlcut_core::IoProblem::BrokenPipe,
        _ => {
            #[cfg(unix)]
            if error.raw_os_error() == Some(rustix::io::Errno::BADF.raw_os_error()) {
                return acquisition().with_cause(htmlcut_core::FailureCause::Io {
                    operation: htmlcut_core::IoOperation::Input,
                    problem: htmlcut_core::IoProblem::InvalidDescriptor,
                });
            }
            htmlcut_core::IoProblem::Other
        }
    };
    acquisition().with_cause(htmlcut_core::FailureCause::Io {
        operation: htmlcut_core::IoOperation::Input,
        problem,
    })
}

pub(crate) fn read_file(path: &Path, maximum: usize) -> Result<Vec<u8>, ExtractionError> {
    read_bounded(&mut regular_file::open(path)?, maximum)
}

pub(crate) fn read_bounded(
    reader: &mut dyn Read,
    maximum: usize,
) -> Result<Vec<u8>, ExtractionError> {
    let mut value = Vec::new();
    let mut chunk = [0_u8; 8192];
    loop {
        let size = reader.read(&mut chunk).map_err(io_failure)?;
        if size == 0 {
            return Ok(value);
        }
        if size > maximum.saturating_sub(value.len()) {
            return Err(limit("acquisition"));
        }
        value.extend_from_slice(&chunk[..size]);
    }
}

pub(crate) fn limit(stage: &'static str) -> ExtractionError {
    ExtractionError::new(
        ErrorCode::ResourceLimit,
        stage,
        "The operation exceeded its configured resource limit.",
    )
    .with_cause(htmlcut_core::FailureCause::Resource {})
}
