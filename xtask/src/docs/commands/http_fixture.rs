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
                        if stream.set_nonblocking(false).is_err()
                            || stream
                                .set_read_timeout(Some(Duration::from_secs(2)))
                                .is_err()
                            || stream
                                .set_write_timeout(Some(Duration::from_secs(2)))
                                .is_err()
                        {
                            continue;
                        }
                        let mut request = Vec::new();
                        while !request.ends_with(b"\r\n\r\n") && request.len() < 16_384 {
                            let mut byte = [0];
                            match stream.read(&mut byte) {
                                Ok(1) => request.push(byte[0]),
                                _ => break,
                            }
                        }
                        let response = format!(
                            "HTTP/1.1 200 OK\r\nConnection: close\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\n\r\n{html}",
                            html.len()
                        );
                        let _ = stream.write_all(response.as_bytes());
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
