// SPDX-License-Identifier: AGPL-3.0-only
//! Bounded snapshot and UTF-8 contract acquisition.
use htmlcut_core::{Contract, Error, Limit, Stop};
use std::{
    fs::File,
    io::{self, IsTerminal, Read},
    path::Path,
};
fn unreadable(file: &str, error: io::Error) -> Error {
    let mut detail = error.to_string();
    if file.starts_with("http://") || file.starts_with("https://") {
        detail.push_str("; FILE is a local snapshot path; fetch it first (see README)");
    }
    Error::InputUnreadable {
        file: file.into(),
        detail,
    }
}
fn bounded(
    reader: &mut impl Read,
    file: &str,
    max: usize,
    contract: bool,
    stop: &Stop,
) -> Result<Vec<u8>, Error> {
    let mut bytes = Vec::new();
    let mut chunk = [0; 64 * 1024];
    loop {
        stop.check()?;
        let remaining = max.saturating_sub(bytes.len());
        let length = chunk.len().min(remaining.saturating_add(1));
        let read = match reader.read(&mut chunk[..length]) {
            Ok(n) => n,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(unreadable(file, e)),
        };
        if read == 0 {
            stop.check()?;
            return Ok(bytes);
        }
        if read > remaining {
            return Err(if contract {
                Error::InvalidContract {
                    path: None,
                    reason: format!("The contract exceeds the {max}-byte grammar bound."),
                }
            } else {
                Error::LimitExceeded {
                    limit: Limit::InputBytes,
                    bound: max.into(),
                }
            });
        }
        bytes.extend_from_slice(&chunk[..read]);
    }
}
pub(crate) fn read(
    path: Option<&Path>,
    max: usize,
    contract: bool,
    stop: &Stop,
) -> Result<Vec<u8>, Error> {
    stop.check()?;
    match path {
        Some(path) if path.as_os_str() != "-" => {
            let name = path.to_string_lossy();
            let mut file = File::open(path).map_err(|e| unreadable(&name, e))?;
            bounded(&mut file, &name, max, contract, stop)
        }
        path => {
            if path.is_none() && io::stdin().is_terminal() {
                return Err(Error::InvalidRequest {
                    reason: "Standard input is a terminal; provide FILE or explicitly use -."
                        .into(),
                });
            }
            bounded(&mut io::stdin().lock(), "-", max, contract, stop)
        }
    }
}
pub(crate) fn contract(path: &Path, stop: &Stop) -> Result<Contract, Error> {
    let bytes = read(Some(path), Contract::MAX_BYTES, true, stop)?;
    let text = std::str::from_utf8(&bytes).map_err(|e| Error::InvalidContract {
        path: None,
        reason: format!("The contract is not UTF-8: {e}."),
    })?;
    Contract::from_json(text, stop)
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounded_retries_interrupted_reads_and_propagates_other_errors() {
        struct ReadThenRetry(Option<io::ErrorKind>, io::Cursor<&'static [u8]>);

        impl Read for ReadThenRetry {
            fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
                match self.0.take() {
                    Some(kind) => Err(kind.into()),
                    None => self.1.read(buf),
                }
            }
        }

        let stop = Stop::none();
        let mut interrupted =
            ReadThenRetry(Some(io::ErrorKind::Interrupted), io::Cursor::new(b"x"));
        assert_eq!(
            bounded(&mut interrupted, "snapshot", 1, false, &stop).unwrap(),
            b"x"
        );

        let kind = io::ErrorKind::PermissionDenied;
        let mut refused = ReadThenRetry(Some(kind), io::Cursor::new(b"x"));
        assert!(matches!(
            bounded(&mut refused, "snapshot", 1, false, &stop),
            Err(Error::InputUnreadable { file, detail })
                if file == "snapshot" && detail == io::Error::from(kind).to_string()
        ));
        assert_eq!(refused.1.position(), 0);
    }

    #[test]
    fn bounded_reads_stop_at_one_excess_byte_and_check_cancellation() {
        let mut source = io::Cursor::new(vec![0; 100_000]);
        let error = bounded(&mut source, "test", 65_536, false, &Stop::none()).unwrap_err();
        assert!(matches!(
            error,
            Error::LimitExceeded {
                limit: Limit::InputBytes,
                ..
            }
        ));
        assert_eq!(source.position(), 65_537);
        let stop = Stop::none().with_cancel(std::sync::Arc::new(
            std::sync::atomic::AtomicBool::new(true),
        ));
        let mut source = io::Cursor::new(b"x");
        assert!(matches!(
            bounded(&mut source, "test", 1, false, &stop),
            Err(Error::Cancelled)
        ));
        assert_eq!(source.position(), 0);
    }
}
