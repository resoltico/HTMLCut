//! Incremental stream retention for instrumented maintainer commands.

use std::fs::File;
use std::io::{self, Read, Write};
use std::process::Child;
use std::thread;

#[cfg(test)]
use std::cell::RefCell;
#[cfg(test)]
use std::sync::Mutex;
#[cfg(test)]
use std::sync::atomic::{AtomicBool, Ordering};

#[cfg(test)]
type StreamChildOverride = dyn FnMut() -> Option<io::Result<std::process::ExitStatus>>;
#[cfg(test)]
type LiveStreamWriteOverride = dyn FnMut(bool, &[u8]) -> Option<io::Result<()>>;

#[cfg(test)]
thread_local! {
    static STREAM_CHILD_OVERRIDE: RefCell<Option<Box<StreamChildOverride>>> = RefCell::new(None);
    static LIVE_STREAM_WRITE_OVERRIDE: RefCell<Option<Box<LiveStreamWriteOverride>>> = RefCell::new(None);
}

#[cfg(test)]
static LIVE_OUTPUT_TEST_LOCK: Mutex<()> = Mutex::new(());
#[cfg(test)]
static SUPPRESS_LIVE_OUTPUT: AtomicBool = AtomicBool::new(false);

/// Retains both child-process streams incrementally and returns its completed exit status.
pub(crate) fn stream_child_to_logs(
    mut child: Child,
    stdout_log: File,
    stderr_log: File,
    mirror: bool,
) -> io::Result<std::process::ExitStatus> {
    #[cfg(test)]
    if let Some(result) = stream_child_override_result() {
        return result;
    }

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| io::Error::other("spawned command did not expose stdout"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| io::Error::other("spawned command did not expose stderr"))?;
    let stdout_reader = thread::spawn(move || retain_stream(stdout, stdout_log, false, mirror));
    let stderr_reader = thread::spawn(move || retain_stream(stderr, stderr_log, true, mirror));
    let status = child.wait()?;
    join_retained_stream(stdout_reader)?;
    join_retained_stream(stderr_reader)?;
    Ok(status)
}

fn retain_stream(
    mut stream: impl Read,
    mut log: File,
    stderr: bool,
    mirror: bool,
) -> io::Result<()> {
    let mut buffer = [0_u8; 8 * 1024];
    loop {
        let read = stream.read(&mut buffer)?;
        if read == 0 {
            return log.flush();
        }
        let bytes = &buffer[..read];
        log.write_all(bytes)?;
        log.flush()?;
        if mirror {
            write_live_stream(stderr, bytes)?;
        }
    }
}

fn join_retained_stream(handle: thread::JoinHandle<io::Result<()>>) -> io::Result<()> {
    handle
        .join()
        .map_err(|_| io::Error::other("command-stream reader thread panicked"))?
}

fn write_live_stream(stderr: bool, bytes: &[u8]) -> io::Result<()> {
    #[cfg(test)]
    if let Some(result) = live_stream_write_override_result(stderr, bytes) {
        return result;
    }

    #[cfg(test)]
    if SUPPRESS_LIVE_OUTPUT.load(Ordering::Relaxed) {
        return Ok(());
    }

    let mut stdout = io::stdout().lock();
    let mut standard_error = io::stderr().lock();
    write_live_stream_to(stderr, bytes, &mut stdout, &mut standard_error)
}

fn write_live_stream_to(
    stderr: bool,
    bytes: &[u8],
    stdout: &mut impl Write,
    standard_error: &mut impl Write,
) -> io::Result<()> {
    let output: &mut dyn Write = if stderr { standard_error } else { stdout };
    output.write_all(bytes)?;
    output.flush()
}

#[cfg(test)]
fn stream_child_override_result() -> Option<io::Result<std::process::ExitStatus>> {
    STREAM_CHILD_OVERRIDE
        .with_borrow_mut(|override_fn| override_fn.as_mut().and_then(|override_fn| override_fn()))
}

#[cfg(test)]
fn live_stream_write_override_result(stderr: bool, bytes: &[u8]) -> Option<io::Result<()>> {
    LIVE_STREAM_WRITE_OVERRIDE.with_borrow_mut(|override_fn| {
        override_fn
            .as_mut()
            .and_then(|override_fn| override_fn(stderr, bytes))
    })
}

#[cfg(test)]
fn with_live_stream_write_override<F, T>(override_fn: F, operation: impl FnOnce() -> T) -> T
where
    F: FnMut(bool, &[u8]) -> Option<io::Result<()>> + 'static,
{
    LIVE_STREAM_WRITE_OVERRIDE.with_borrow_mut(|slot| {
        assert!(
            slot.is_none(),
            "live-stream override should not already be installed"
        );
        *slot = Some(Box::new(override_fn));
    });

    let outcome = operation();

    LIVE_STREAM_WRITE_OVERRIDE.with_borrow_mut(|slot| {
        *slot = None;
    });

    outcome
}

#[cfg(test)]
pub(crate) fn with_stream_child_override<F, T>(override_fn: F, operation: impl FnOnce() -> T) -> T
where
    F: FnMut() -> Option<io::Result<std::process::ExitStatus>> + 'static,
{
    STREAM_CHILD_OVERRIDE.with_borrow_mut(|slot| {
        assert!(
            slot.is_none(),
            "stream-child override should not already be installed"
        );
        *slot = Some(Box::new(override_fn));
    });

    let outcome = operation();

    STREAM_CHILD_OVERRIDE.with_borrow_mut(|slot| {
        *slot = None;
    });

    outcome
}

#[cfg(test)]
pub(crate) fn with_suppressed_live_output<T>(operation: impl FnOnce() -> T) -> T {
    let _lock = LIVE_OUTPUT_TEST_LOCK
        .lock()
        .expect("live-output test lock should not be poisoned");
    let previous = SUPPRESS_LIVE_OUTPUT.swap(true, Ordering::Relaxed);
    let outcome = operation();
    SUPPRESS_LIVE_OUTPUT.store(previous, Ordering::Relaxed);
    outcome
}

#[cfg(test)]
pub(crate) fn write_live_stream_for_tests(stderr: bool) -> io::Result<()> {
    write_live_stream(stderr, b"")
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::fs;
    use std::io::Read;
    use std::rc::Rc;

    use htmlcut_tempdir::tempdir;

    use super::*;

    #[test]
    fn stream_retention_preserves_payload_with_the_exact_bounded_read_buffer() {
        let observed_buffer_capacity = Rc::new(Cell::new(0));
        let reader = CapacityRecordingReader {
            remaining: b"retained stream payload".to_vec(),
            observed_buffer_capacity: Rc::clone(&observed_buffer_capacity),
        };
        let directory = tempdir().expect("temporary log directory");
        let path = directory.path().join("stream.log");
        let log = File::create(&path).expect("create stream log");

        retain_stream(reader, log, false, false).expect("retain stream");

        assert_eq!(observed_buffer_capacity.get(), 8 * 1024);
        assert_eq!(
            fs::read(&path).expect("read retained stream"),
            b"retained stream payload"
        );
    }

    #[test]
    fn retained_stream_join_propagates_reader_failures() {
        let reader = thread::spawn(|| Err(io::Error::other("retention failure")));

        let error = join_retained_stream(reader).expect_err("reader failure must propagate");

        assert!(error.to_string().contains("retention failure"));
    }

    #[test]
    fn live_stream_forwarding_preserves_destination_and_bytes() {
        let writes = Rc::new(RefCell::new(Vec::new()));
        let writes_for_override = Rc::clone(&writes);

        with_live_stream_write_override(
            move |stderr, bytes| {
                writes_for_override
                    .borrow_mut()
                    .push((stderr, bytes.to_vec()));
                Some(Ok(()))
            },
            || {
                write_live_stream(false, b"stdout bytes").expect("forward stdout");
                write_live_stream(true, b"stderr bytes").expect("forward stderr");
            },
        );

        assert_eq!(
            writes.borrow().as_slice(),
            [
                (false, b"stdout bytes".to_vec()),
                (true, b"stderr bytes".to_vec()),
            ]
        );
    }

    #[test]
    fn live_stream_writes_to_the_selected_destination_without_global_test_output() {
        let mut stdout = Vec::new();
        let mut standard_error = Vec::new();
        write_live_stream_to(false, b"stdout", &mut stdout, &mut standard_error)
            .expect("stdout write");
        write_live_stream_to(true, b"stderr", &mut stdout, &mut standard_error)
            .expect("stderr write");
        assert_eq!(stdout, b"stdout");
        assert_eq!(standard_error, b"stderr");
    }

    #[test]
    fn live_stream_empty_probe_exercises_the_real_terminal_writer() {
        let _lock = LIVE_OUTPUT_TEST_LOCK
            .lock()
            .expect("live-output test lock should not be poisoned");
        assert!(
            !SUPPRESS_LIVE_OUTPUT.load(Ordering::Relaxed),
            "the serialized probe requires real terminal output"
        );

        write_live_stream(false, b"").expect("empty stdout probe");
        write_live_stream(true, b"").expect("empty stderr probe");
    }

    struct CapacityRecordingReader {
        remaining: Vec<u8>,
        observed_buffer_capacity: Rc<Cell<usize>>,
    }

    impl Read for CapacityRecordingReader {
        fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
            self.observed_buffer_capacity.set(buffer.len());
            let count = self.remaining.len().min(buffer.len());
            buffer[..count].copy_from_slice(&self.remaining[..count]);
            self.remaining.drain(..count);
            Ok(count)
        }
    }
}
