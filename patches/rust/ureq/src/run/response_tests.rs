//! Independent complete-header values and transport-failure propagation controls.
use crate::unversioned::transport::set_handler_raw;
use crate::{Error, get};

#[test]
fn exact_header_limit_accepts_a_complete_section_and_refuses_the_next_byte() {
    const HEADER: &[u8] = b"HTTP/1.1 200 OK\r\nContent-Length: 1\r\n\r\n";
    set_handler_raw("/exact-header", |writer| {
        writer.write_all(HEADER)?;
        writer.write_all(b"X")
    });
    for (limit, expected) in [(38, true), (37, false)] {
        let response = get("http://fixture.test/exact-header")
            .config()
            .max_response_header_size(limit)
            .build()
            .call();
        if expected {
            let mut response = response.unwrap();
            assert_eq!(response.body_mut().read_to_string().unwrap(), "X");
        } else {
            assert!(matches!(
                response.unwrap_err(),
                Error::LargeResponseHeader(_, 37)
            ));
        }
    }
}

#[test]
fn unfinished_redirect_at_eof_is_a_protocol_failure_without_synthetic_response() {
    set_handler_raw("/unfinished-headers", |writer| {
        writer.write_all(b"HTTP/1.1 302 Found\r\nLocation: /target\r\n")
    });
    for maximum in [0, 3] {
        let error = get("http://fixture.test/unfinished-headers")
            .config()
            .max_redirects(maximum)
            .build()
            .call()
            .unwrap_err();
        assert!(
            matches!(error, Error::Protocol(ureq_proto::Error::HttpParseFail(ref message))
            if message == "incomplete response header section")
        );
    }
}

#[derive(Debug)]
struct ReceiveResetConnector;

impl<In: crate::unversioned::transport::Transport> crate::unversioned::transport::Connector<In>
    for ReceiveResetConnector
{
    type Out = ReceiveResetTransport;
    fn connect(
        &self,
        _details: &crate::unversioned::transport::ConnectionDetails,
        _chained: Option<In>,
    ) -> Result<Option<Self::Out>, Error> {
        Ok(Some(ReceiveResetTransport {
            buffers: crate::unversioned::transport::LazyBuffers::new(4096, 4096),
        }))
    }
}

#[derive(Debug)]
struct ReceiveResetTransport {
    buffers: crate::unversioned::transport::LazyBuffers,
}
impl crate::unversioned::transport::Transport for ReceiveResetTransport {
    fn buffers(&mut self) -> &mut dyn crate::unversioned::transport::Buffers {
        &mut self.buffers
    }
    fn transmit_output(
        &mut self,
        _amount: usize,
        _timeout: crate::unversioned::transport::NextTimeout,
    ) -> Result<(), Error> {
        Ok(())
    }
    fn await_input(
        &mut self,
        _timeout: crate::unversioned::transport::NextTimeout,
    ) -> Result<bool, Error> {
        Err(Error::Io(std::io::Error::new(
            std::io::ErrorKind::ConnectionReset,
            "synthetic receive reset",
        )))
    }
    fn is_open(&mut self) -> bool {
        true
    }
}

#[test]
fn receive_reset_is_preserved_instead_of_becoming_incomplete_headers_or_success() {
    let agent = crate::Agent::with_parts(
        crate::config::Config::default(),
        ReceiveResetConnector,
        crate::unversioned::resolver::DefaultResolver::default(),
    );
    let error = agent
        .get("http://127.0.0.1/receive-reset")
        .call()
        .unwrap_err();
    assert!(
        matches!(error, Error::Io(ref error) if error.kind() == std::io::ErrorKind::ConnectionReset)
    );
}
