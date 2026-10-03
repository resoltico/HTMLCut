//! Preserve underlying read/seek failures when a format library creates its own errors.

use htmlcut_core::ExtractionError;
use std::{
    cell::RefCell,
    io::{self, Read, Seek, SeekFrom},
};

pub(crate) struct ObservedReader<'a, R> {
    pub(crate) inner: R,
    pub(crate) failures: &'a RefCell<Option<ExtractionError>>,
    pub(crate) position: u64,
    pub(crate) size: u64,
}
impl<R: Read> Read for ObservedReader<'_, R> {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        match self.inner.read(bytes) {
            Ok(0) if !bytes.is_empty() && self.position < self.size => {
                let error = io::Error::from(io::ErrorKind::UnexpectedEof);
                *self.failures.borrow_mut() = Some(crate::input::io_failure_ref(&error));
                Err(error)
            }
            Ok(count) => {
                self.position += count as u64;
                Ok(count)
            }
            Err(error) => {
                *self.failures.borrow_mut() = Some(crate::input::io_failure_ref(&error));
                Err(error)
            }
        }
    }
}
impl<R: Seek> Seek for ObservedReader<'_, R> {
    fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
        match self.inner.seek(position) {
            Ok(position) => {
                self.position = position;
                Ok(position)
            }
            Err(error) => {
                *self.failures.borrow_mut() = Some(crate::input::io_failure_ref(&error));
                Err(error)
            }
        }
    }
}
pub(crate) fn archive_failure(failures: &RefCell<Option<ExtractionError>>) -> ExtractionError {
    failures
        .borrow_mut()
        .take()
        .unwrap_or_else(super::bundle::invalid)
}
