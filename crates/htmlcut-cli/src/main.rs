#![forbid(unsafe_code)]

use std::io::{self, Write};

struct BrokenPipeTolerantWriter<W> {
    inner: W,
    broken_pipe: bool,
}

impl<W> BrokenPipeTolerantWriter<W> {
    fn new(inner: W) -> Self {
        Self {
            inner,
            broken_pipe: false,
        }
    }
}

impl<W: Write> Write for BrokenPipeTolerantWriter<W> {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        if self.broken_pipe {
            return Ok(buffer.len());
        }

        match self.inner.write(buffer) {
            Err(error) if error.kind() == io::ErrorKind::BrokenPipe => {
                self.broken_pipe = true;
                Ok(buffer.len())
            }
            result => result,
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        if self.broken_pipe {
            return Ok(());
        }

        match self.inner.flush() {
            Err(error) if error.kind() == io::ErrorKind::BrokenPipe => {
                self.broken_pipe = true;
                Ok(())
            }
            result => result,
        }
    }
}

fn main() {
    let stdout = io::stdout();
    let stderr = io::stderr();
    let mut stdout = BrokenPipeTolerantWriter::new(stdout.lock());
    let mut stderr = stderr.lock();
    let code = match htmlcut_cli::run(std::env::args(), &mut stdout, &mut stderr) {
        Ok(code) => code,
        Err(error) => {
            let _ = writeln!(stderr, "htmlcut: failed to write CLI output: {error}");
            htmlcut_cli::EXIT_CODE_OUTPUT
        }
    };
    std::process::exit(code);
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FailingWriter(io::ErrorKind);

    impl Write for FailingWriter {
        fn write(&mut self, _buffer: &[u8]) -> io::Result<usize> {
            Err(io::Error::from(self.0))
        }

        fn flush(&mut self) -> io::Result<()> {
            Err(io::Error::from(self.0))
        }
    }

    #[test]
    fn stdout_adapter_suppresses_only_broken_pipe() {
        let mut broken_pipe =
            BrokenPipeTolerantWriter::new(FailingWriter(io::ErrorKind::BrokenPipe));
        assert_eq!(broken_pipe.write(b"payload").expect("broken pipe write"), 7);
        broken_pipe.flush().expect("broken pipe flush");

        let mut broken_pipe_flush =
            BrokenPipeTolerantWriter::new(FailingWriter(io::ErrorKind::BrokenPipe));
        broken_pipe_flush
            .flush()
            .expect("initial broken pipe flush");
        assert_eq!(
            broken_pipe_flush
                .write(b"payload")
                .expect("write after broken pipe flush"),
            7
        );

        let mut other_error = BrokenPipeTolerantWriter::new(FailingWriter(io::ErrorKind::Other));
        assert_eq!(
            other_error
                .write(b"payload")
                .expect_err("non-broken-pipe write")
                .kind(),
            io::ErrorKind::Other
        );
        assert_eq!(
            other_error
                .flush()
                .expect_err("non-broken-pipe flush")
                .kind(),
            io::ErrorKind::Other
        );
    }
}
