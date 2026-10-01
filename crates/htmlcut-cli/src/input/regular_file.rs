//! Validate the opened input handle, without waiting for a FIFO writer.

use super::*;
use htmlcut_core::{FailureCause, IoOperation, IoProblem};

pub(super) fn open(path: &Path) -> Result<File, ExtractionError> {
    #[cfg(unix)]
    let file: File = rustix::fs::open(
        path,
        rustix::fs::OFlags::RDONLY | rustix::fs::OFlags::CLOEXEC | rustix::fs::OFlags::NONBLOCK,
        rustix::fs::Mode::empty(),
    )
    .map_err(|error| super::io_failure(std::io::Error::from(error)))?
    .into();
    #[cfg(not(unix))]
    let file = {
        // Windows CreateFile does not wait for a named-pipe instance. Reject known special
        // paths and validate the resulting handle as well, including replacement races.
        if !std::fs::metadata(path)
            .map_err(super::io_failure)?
            .is_file()
        {
            return Err(kind_failure());
        }
        File::open(path).map_err(super::io_failure)?
    };
    if !file.metadata().map_err(super::io_failure)?.is_file() {
        return Err(kind_failure());
    }
    Ok(file)
}

fn kind_failure() -> ExtractionError {
    acquisition().with_cause(FailureCause::Io {
        operation: IoOperation::Input,
        problem: IoProblem::UnsupportedKind,
    })
}
