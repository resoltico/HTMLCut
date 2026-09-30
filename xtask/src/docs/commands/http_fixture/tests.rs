use super::*;
use std::io::Cursor;

struct Connection {
    input: Cursor<Vec<u8>>,
    output: Vec<u8>,
    read_failure: bool,
    write_failure: bool,
}
impl Read for Connection {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        if self.read_failure {
            return Err(io::Error::new(io::ErrorKind::TimedOut, "fixture timeout"));
        }
        self.input.read(bytes)
    }
}
impl Write for Connection {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self.write_failure {
            return Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "fixture disconnect",
            ));
        }
        self.output.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
fn connection(bytes: &[u8]) -> Connection {
    Connection {
        input: Cursor::new(bytes.to_vec()),
        output: Vec::new(),
        read_failure: false,
        write_failure: false,
    }
}

#[test]
fn complete_requests_receive_exact_utf8_body_and_incomplete_requests_are_not_published() {
    let mut stream = connection(b"GET /fixture HTTP/1.1\r\nHost: localhost\r\n\r\n");
    serve(&mut stream, "é", |_| Ok(())).unwrap();
    assert_eq!(
        String::from_utf8(stream.output).unwrap(),
        "HTTP/1.1 200 OK\r\nConnection: close\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: 2\r\n\r\né"
    );
    for bytes in [b"".as_slice(), b"GET /fixture HTTP/1.1\r\n".as_slice()] {
        let mut stream = connection(bytes);
        assert_eq!(
            serve(&mut stream, "body", |_| Ok(())).unwrap_err().kind(),
            io::ErrorKind::UnexpectedEof
        );
        assert!(stream.output.is_empty());
    }
    for size in [16383, 16384, 16385] {
        let mut bytes = vec![b'x'; size - 4];
        bytes.extend_from_slice(b"\r\n\r\n");
        let mut stream = connection(&bytes);
        assert_eq!(
            serve(&mut stream, "body", |_| Ok(())).is_ok(),
            size <= 16384
        );
        if size > 16384 {
            assert!(stream.output.is_empty());
        }
    }
}

#[test]
fn connection_configuration_read_and_write_failures_are_bounded_and_truthful() {
    let request = b"GET /fixture HTTP/1.1\r\n\r\n";
    let mut stream = connection(request);
    assert_eq!(
        serve(&mut stream, "body", |_| Err(io::Error::other(
            "configuration"
        )))
        .unwrap_err()
        .to_string(),
        "configuration"
    );
    assert!(stream.output.is_empty());
    assert_eq!(stream.input.position(), 0);
    stream.read_failure = true;
    assert_eq!(
        serve(&mut stream, "body", |_| Ok(())).unwrap_err().kind(),
        io::ErrorKind::TimedOut
    );
    stream.read_failure = false;
    stream.write_failure = true;
    assert_eq!(
        serve(&mut stream, "body", |_| Ok(())).unwrap_err().kind(),
        io::ErrorKind::BrokenPipe
    );
}

#[test]
fn accept_loop_stops_on_permanent_failure_or_owner_stop_after_would_block() {
    let flag = AtomicBool::new(false);
    let mut calls = 0;
    accept_loop(&flag, || {
        calls += 1;
        Err(io::Error::other("listener failed"))
    });
    assert_eq!(calls, 1);
    let mut calls = 0;
    accept_loop(&flag, || {
        calls += 1;
        if calls == 1 {
            Err(io::Error::from(io::ErrorKind::WouldBlock))
        } else {
            flag.store(true, Ordering::Relaxed);
            Ok(())
        }
    });
    assert_eq!(calls, 2);
}

#[test]
fn each_socket_policy_failure_short_circuits_before_later_changes() {
    use std::cell::Cell;
    struct Policy {
        calls: Cell<usize>,
        fail_at: usize,
    }
    impl Policy {
        fn step(&self) -> io::Result<()> {
            self.calls.set(self.calls.get() + 1);
            if self.calls.get() == self.fail_at {
                Err(io::Error::other("socket policy"))
            } else {
                Ok(())
            }
        }
    }
    impl SocketPolicy for Policy {
        fn blocking(&self) -> io::Result<()> {
            self.step()
        }
        fn read_deadline(&self) -> io::Result<()> {
            self.step()
        }
        fn write_deadline(&self) -> io::Result<()> {
            self.step()
        }
    }
    for fail_at in 1..=4 {
        let mut policy = Policy {
            calls: Cell::new(0),
            fail_at,
        };
        assert_eq!(configure(&mut policy).is_ok(), fail_at == 4);
        assert_eq!(policy.calls.get(), fail_at.min(3));
    }
}
