// SPDX-License-Identifier: MPL-2.0
//! Bounded staging and atomic single-file publication.

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use htmlcut_core::{ErrorCode, ExtractionError};
use serde::Serialize;
use tempfile::NamedTempFile;

pub(crate) const MAX_OUTPUT_BYTES: usize = htmlcut_core::MAX_DATA_BYTES + 1;
pub(crate) const MAX_RECEIPT_BYTES: usize = htmlcut_core::MAX_RECEIPT_BYTES + 1;

pub(crate) fn failure() -> ExtractionError {
    ExtractionError::new(
        ErrorCode::Publication,
        "publication",
        "The staged output could not be published.",
    )
}

pub(crate) fn io_failure(
    error: io::Error,
    operation: htmlcut_core::IoOperation,
) -> ExtractionError {
    let problem = match error.kind() {
        io::ErrorKind::PermissionDenied => htmlcut_core::IoProblem::PermissionDenied,
        io::ErrorKind::NotFound => htmlcut_core::IoProblem::NotFound,
        io::ErrorKind::BrokenPipe => htmlcut_core::IoProblem::BrokenPipe,
        _ => {
            #[cfg(unix)]
            if error.raw_os_error() == Some(rustix::io::Errno::BADF.raw_os_error()) {
                return failure().with_cause(htmlcut_core::FailureCause::Io {
                    operation,
                    problem: htmlcut_core::IoProblem::InvalidDescriptor,
                });
            }
            htmlcut_core::IoProblem::Other
        }
    };
    failure().with_cause(htmlcut_core::FailureCause::Io { operation, problem })
}

pub(crate) fn json(value: &impl Serialize, maximum: usize) -> Result<Vec<u8>, ExtractionError> {
    json_stream(value, maximum)
}

pub(crate) fn json_stream(
    value: &impl Serialize,
    maximum: usize,
) -> Result<Vec<u8>, ExtractionError> {
    let mut bytes = json_payload(value, maximum.saturating_sub(1))?;
    bytes.push(b'\n');
    Ok(bytes)
}

pub(crate) fn json_payload(
    value: &impl Serialize,
    maximum: usize,
) -> Result<Vec<u8>, ExtractionError> {
    let mut output = Buffer {
        bytes: Vec::new(),
        maximum,
    };
    serde_json::to_writer(&mut output, value).map_err(|_| super::input::limit("serialization"))?;
    Ok(output.bytes)
}

struct Buffer {
    bytes: Vec<u8>,
    maximum: usize,
}
impl Write for Buffer {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() > self.maximum.saturating_sub(self.bytes.len()) {
            return Err(io::Error::other("Serialized output limit exceeded."));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

pub(crate) fn normalized_target(path: &Path) -> Result<PathBuf, ExtractionError> {
    let name = path.file_name().ok_or_else(failure)?;
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let parent = fs::canonicalize(parent).map_err(|_| failure())?;
    let target = parent.join(name);
    if let Ok(metadata) = fs::symlink_metadata(&target)
        && (metadata.file_type().is_symlink() || !metadata.is_file())
    {
        return Err(failure());
    }
    Ok(target)
}

pub(crate) fn validate_destinations(
    inputs: &[PathBuf],
    targets: &[PathBuf],
    overwrite: bool,
) -> Result<(), ExtractionError> {
    let inputs = inputs
        .iter()
        .map(|path| fs::canonicalize(path).map_err(super::input::io_failure))
        .collect::<Result<Vec<_>, _>>()?;
    let mut seen = Vec::new();
    for path in targets {
        let target = normalized_target(path)?;
        if inputs.contains(&target) || seen.contains(&target) {
            return Err(super::input::options(
                "Input, result and evidence destinations must be distinct.",
            ));
        }
        if !overwrite && target.exists() {
            return Err(failure());
        }
        seen.push(target);
    }
    Ok(())
}

pub(crate) struct Staged {
    file: NamedTempFile,
    target: PathBuf,
    overwrite: bool,
}

fn finish_staged_writer<W: Write>(
    writer: &mut W,
    write: impl FnOnce(&mut W) -> Result<(), ExtractionError>,
    sync: impl FnOnce(&mut W) -> io::Result<()>,
) -> Result<(), ExtractionError> {
    write(writer)?;
    writer
        .flush()
        .and_then(|_| sync(writer))
        .map_err(|error| io_failure(error, htmlcut_core::IoOperation::Publication))
}

impl Staged {
    pub(crate) fn prepare(
        target: &Path,
        bytes: &[u8],
        overwrite: bool,
    ) -> Result<Self, ExtractionError> {
        Self::prepare_with(target, overwrite, |file| write_staged_bytes(file, bytes))
    }
    pub(crate) fn prepare_with(
        target: &Path,
        overwrite: bool,
        write: impl FnOnce(&mut fs::File) -> Result<(), ExtractionError>,
    ) -> Result<Self, ExtractionError> {
        let target = normalized_target(target)?;
        let mut file = NamedTempFile::new_in(target.parent().unwrap()).map_err(|_| failure())?;
        finish_staged_writer(file.as_file_mut(), write, |file| file.sync_all())?;
        Ok(Self {
            file,
            target,
            overwrite,
        })
    }
    pub(crate) fn commit(self) -> Result<(), ExtractionError> {
        // tempfile uses native atomic replacement/create primitives; persist_noclobber does
        // not turn a prior exists() check into a check-then-clobber publication race.
        if self.overwrite {
            self.file.persist(&self.target)
        } else {
            self.file.persist_noclobber(&self.target)
        }
        .map(|_| ())
        .map_err(|_| failure())
    }
}

#[cfg(test)]
#[path = "tests/publication_writer.rs"]
mod writer_tests;

fn write_staged_bytes(writer: &mut impl Write, bytes: &[u8]) -> Result<(), ExtractionError> {
    writer
        .write_all(bytes)
        .map_err(|error| io_failure(error, htmlcut_core::IoOperation::Publication))
}
