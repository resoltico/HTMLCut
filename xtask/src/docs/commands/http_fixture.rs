//! Offline loopback acquisition for executable URL/run documentation examples.

use std::io::{self, Read, Write};
use std::net::TcpListener;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::thread::JoinHandle;
use std::time::Duration;

pub(super) struct Fixture {
    pub(super) url: String,
    stopped: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}

impl Fixture {
    pub(super) fn new(html: &'static str) -> io::Result<Self> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        listener.set_nonblocking(true)?;
        let url = format!("http://{}/fixture", listener.local_addr()?);
        let stopped = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&stopped);
        let worker = std::thread::spawn(move || {
            while !flag.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        let _ = serve(&mut stream, html, |stream| {
                            stream.set_nonblocking(false)?;
                            stream.set_read_timeout(Some(Duration::from_secs(2)))?;
                            stream.set_write_timeout(Some(Duration::from_secs(2)))
                        });
                    }
                    Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(2))
                    }
                    Err(_) => break,
                }
            }
        });
        Ok(Self {
            url,
            stopped,
            worker: Some(worker),
        })
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.stopped.store(true, Ordering::Relaxed);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

// One bounded connection is kept separate from the listener's lifetime. This also
// allows deterministic read/write/configuration failure verification without OS races.
fn serve<S: Read + Write>(
    stream: &mut S,
    html: &str,
    configure: impl FnOnce(&mut S) -> io::Result<()>,
) -> io::Result<()> {
    configure(stream)?;
    let mut request = Vec::new();
    loop {
        let mut bytes = [0; 512];
        let count = stream.read(&mut bytes)?;
        if count == 0 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "Incomplete fixture request.",
            ));
        }
        if count > 16_384_usize.saturating_sub(request.len()) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Fixture request exceeded its bound.",
            ));
        }
        request.extend_from_slice(&bytes[..count]);
        if request.windows(4).any(|bytes| bytes == b"\r\n\r\n") {
            break;
        }
    }
    let response = format!(
        "HTTP/1.1 200 OK\r\nConnection: close\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\n\r\n{html}",
        html.len()
    );
    stream.write_all(response.as_bytes())
}

#[cfg(test)]
#[path = "http_fixture/tests.rs"]
mod tests;
