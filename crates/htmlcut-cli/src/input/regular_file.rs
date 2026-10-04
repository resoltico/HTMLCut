// SPDX-License-Identifier: MPL-2.0
//! Validate the opened input handle, without waiting for a FIFO writer.

use super::*;
use htmlcut_core::{FailureCause, IoOperation, IoProblem};

pub(super) fn open(path: &Path) -> Result<File, ExtractionError> {
    #[cfg(unix)]
    let file: File = rustix::fs::open(
        path,
        rustix::fs::OFlags::RDONLY
            .union(rustix::fs::OFlags::CLOEXEC)
            .union(rustix::fs::OFlags::NONBLOCK),
        rustix::fs::Mode::empty(),
    )
    .map_err(|error| super::io_failure(std::io::Error::from(error)))?
    .into();
    #[cfg(not(unix))]
    let file = {
        // Windows CreateFile does not wait for a named-pipe instance. Reject known special
        // paths and validate the resulting handle as well, including replacement races.
        validate_kind(
            std::fs::metadata(path)
                .map_err(super::io_failure)?
                .is_file(),
        )?;
        File::open(path).map_err(super::io_failure)?
    };
    validate_kind(file.metadata().map_err(super::io_failure)?.is_file())?;
    Ok(file)
}

fn kind_failure() -> ExtractionError {
    acquisition().with_cause(FailureCause::Io {
        operation: IoOperation::Input,
        problem: IoProblem::UnsupportedKind,
    })
}

fn validate_kind(regular: bool) -> Result<(), ExtractionError> {
    if regular { Ok(()) } else { Err(kind_failure()) }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_regular_file_kind_is_authorized_before_byte_reads() {
        assert!(validate_kind(true).is_ok());
        let error = validate_kind(false).unwrap_err();
        assert_eq!(error.code, htmlcut_core::ErrorCode::Acquisition);
        assert_eq!(
            error.evidence.cause,
            Some(FailureCause::Io {
                operation: IoOperation::Input,
                problem: IoProblem::UnsupportedKind,
            })
        );
    }
}
