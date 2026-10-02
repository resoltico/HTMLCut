//! Lazy real stream handles: unused streams are not prerequisites for work.

#[cfg(windows)]
use std::io::IsTerminal;
use std::io::{self, Read, Write};

#[derive(Default)]
pub(crate) struct Input {
    reader: Option<os_pipe::PipeReader>,
}
impl Read for Input {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        if bytes.is_empty() {
            return Ok(0);
        }
        if self.reader.is_none() {
            self.reader = Some(os_pipe::dup_stdin()?);
        }
        #[cfg(windows)]
        if io::stdin().is_terminal() {
            return io::stdin().lock().read(bytes);
        }
        self.reader
            .as_mut()
            .expect("initialized input handle")
            .read(bytes)
    }
}

pub(crate) struct Output {
    writer: Option<os_pipe::PipeWriter>,
    error: bool,
}
impl Output {
    pub(crate) fn stdout() -> Self {
        Self {
            writer: None,
            error: false,
        }
    }
    pub(crate) fn stderr() -> Self {
        Self {
            writer: None,
            error: true,
        }
    }
}
impl Write for Output {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.is_empty() {
            return Ok(0);
        }
        if self.writer.is_none() {
            self.writer = Some(if self.error {
                os_pipe::dup_stderr()?
            } else {
                os_pipe::dup_stdout()?
            });
        }
        // Windows console APIs preserve Unicode; redirection uses the strict duplicated file.
        #[cfg(windows)]
        {
            if self.error && io::stderr().is_terminal() {
                return io::stderr().lock().write(bytes);
            }
            if !self.error && io::stdout().is_terminal() {
                return io::stdout().lock().write(bytes);
            }
        }
        self.writer
            .as_mut()
            .expect("initialized output handle")
            .write(bytes)
    }
    fn flush(&mut self) -> io::Result<()> {
        // Writes are unbuffered; the duplicated File backend has no pending userspace bytes.
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn acquired_pipe_backend_is_reused_for_multiple_writes() {
        let (mut reader, writer) = os_pipe::pipe().unwrap();
        let mut output = Output {
            writer: Some(writer),
            error: false,
        };
        assert_eq!(output.write(b"first ").unwrap(), 6);
        assert_eq!(output.write(b"second").unwrap(), 6);
        output.flush().unwrap();
        drop(output);
        let mut delivered = Vec::new();
        reader.read_to_end(&mut delivered).unwrap();
        assert_eq!(delivered, b"first second");
    }

    #[test]
    fn empty_operations_do_not_acquire_unused_streams() {
        let mut input = Input::default();
        assert_eq!(input.read(&mut []).unwrap(), 0);
        assert!(input.reader.is_none());
        for mut output in [Output::stdout(), Output::stderr()] {
            assert_eq!(output.write(&[]).unwrap(), 0);
            output.flush().unwrap();
            assert!(output.writer.is_none());
        }
    }
}
