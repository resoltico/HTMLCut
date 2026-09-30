//! Bounded staging and atomic single-file publication.

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use htmlcut_core::{ErrorCode, ExtractionError};
use serde::Serialize;
use tempfile::NamedTempFile;

pub(crate) const MAX_OUTPUT_BYTES: usize = 128 * 1024 * 1024;
pub(crate) const MAX_AUDIT_BYTES: usize = 1024 * 1024;

pub(crate) fn failure() -> ExtractionError {
    ExtractionError::new(
        ErrorCode::Publication,
        "publication",
        "The staged output could not be published.",
    )
}

pub(crate) fn json(value: &impl Serialize, maximum: usize) -> Result<Vec<u8>, ExtractionError> {
    // Value uses sorted maps, so emitted object order is deterministic at every level.
    let mut value = serde_json::to_value(value).map_err(|_| failure())?;
    value.sort_all_objects();
    json_stream(&value, maximum)
}

pub(crate) fn json_stream(
    value: &impl Serialize,
    maximum: usize,
) -> Result<Vec<u8>, ExtractionError> {
    let mut output = Buffer {
        bytes: Vec::new(),
        maximum,
    };
    serde_json::to_writer(&mut output, value).map_err(|_| super::input::limit("serialization"))?;
    output
        .write_all(b"\n")
        .map_err(|_| super::input::limit("serialization"))?;
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
        .map(|path| fs::canonicalize(path).map_err(|_| super::input::acquisition()))
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

impl Staged {
    pub(crate) fn prepare(
        target: &Path,
        bytes: &[u8],
        overwrite: bool,
    ) -> Result<Self, ExtractionError> {
        let target = normalized_target(target)?;
        let mut file = NamedTempFile::new_in(target.parent().unwrap()).map_err(|_| failure())?;
        file.write_all(bytes)
            .and_then(|_| file.flush())
            .and_then(|_| file.as_file().sync_all())
            .map_err(|_| failure())?;
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
