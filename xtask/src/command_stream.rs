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
thread_local! {
    static STREAM_CHILD_OVERRIDE: RefCell<Option<Box<StreamChildOverride>>> = RefCell::new(None);
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
    if SUPPRESS_LIVE_OUTPUT.load(Ordering::Relaxed) {
        return Ok(());
    }

    if stderr {
        let mut output = io::stderr().lock();
        output.write_all(bytes)?;
        output.flush()
    } else {
        let mut output = io::stdout().lock();
        output.write_all(bytes)?;
        output.flush()
    }
}

#[cfg(test)]
fn stream_child_override_result() -> Option<io::Result<std::process::ExitStatus>> {
    STREAM_CHILD_OVERRIDE
        .with_borrow_mut(|override_fn| override_fn.as_mut().and_then(|override_fn| override_fn()))
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
