//! Framed bytes, explicit caller text reading and original wire body length.
use super::*;

impl ResponseInfo {
    /// The known length of the body, if any.
    ///
    /// This is the value of the `Content-Length` header. For chunked or close-delimited
    /// responses this is `None`. Representation decoding belongs to the caller.
    pub(crate) fn content_length(&self) -> Option<u64> {
        match self.body_mode {
            BodyMode::NoBody => None,
            BodyMode::LengthDelimited(v) => Some(v),
            BodyMode::Chunked => None,
            BodyMode::CloseDelimited => None,
        }
    }
}

/// A reader of the response data.
///
/// If `Transfer-Encoding: chunked`, the returned reader unchunks it and ignores Content-Length.
///
/// Representation decoding is caller-owned. Original encoding labels and bytes are retained.
/// Content-Length, chunked framing and the requested body limit remain enforced.
///
/// Note: The reader is also limited by the [`Body::as_reader`] and
/// [`Body::into_reader`] calls. If that limit is set very high, a malicious
/// server might return enough bytes to exhaust available memory. If you're
/// making requests to untrusted servers, you should use set that
/// limit accordingly.
///
/// # Example
///
/// ```
/// use std::io::Read;
/// let mut res = ureq::get("http://httpbin.org/bytes/100")
///     .call()?;
///
/// assert!(res.headers().contains_key("Content-Length"));
/// let len: usize = res.headers().get("Content-Length")
///     .unwrap().to_str().unwrap().parse().unwrap();
///
/// let mut bytes: Vec<u8> = Vec::with_capacity(len);
/// res.body_mut().as_reader()
///     .read_to_end(&mut bytes)?;
///
/// assert_eq!(bytes.len(), len);
/// # Ok::<_, ureq::Error>(())
/// ```
pub struct BodyReader<'a> {
    reader: MaybeLossyDecoder<LimitReader<BodySourceRef<'a>>>,
    // If this reader is used as SendBody for another request, this
    // Framing remains unchanged because the reader retains representation bytes.
    outgoing_body_mode: BodyMode,
}

impl<'a> BodyReader<'a> {
    pub(super) fn new(
        reader: LimitReader<BodySourceRef<'a>>,
        info: &ResponseInfo,
        incoming_body_mode: BodyMode,
        lossy_utf8: bool,
    ) -> BodyReader<'a> {
        let reader = if info.is_text() && lossy_utf8 {
            MaybeLossyDecoder::Lossy(LossyUtf8Reader::new(reader))
        } else {
            MaybeLossyDecoder::PassThrough(reader)
        };

        BodyReader {
            outgoing_body_mode: incoming_body_mode,
            reader,
        }
    }

    pub(crate) fn body_mode(&self) -> BodyMode {
        self.outgoing_body_mode
    }
}

enum MaybeLossyDecoder<R> {
    Lossy(LossyUtf8Reader<R>),
    PassThrough(R),
}

impl<R: io::Read> io::Read for MaybeLossyDecoder<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self {
            MaybeLossyDecoder::Lossy(r) => r.read(buf),
            MaybeLossyDecoder::PassThrough(r) => r.read(buf),
        }
    }
}

impl<'a> io::Read for BodyReader<'a> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.reader.read(buf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::unversioned::transport::set_handler;
    use std::io::Read;

    #[test]
    fn original_encoding_headers_length_and_bytes_survive_the_carrier() {
        set_handler(
            "/raw-representation",
            200,
            &[
                ("content-type", "text/plain; charset=windows-1252"),
                ("content-encoding", "gzip"),
                ("content-length", "3"),
            ],
            "€".as_bytes(),
        );
        let mut response = crate::get("http://fixture.test/raw-representation")
            .call()
            .unwrap();
        assert_eq!(response.headers().get("content-encoding").unwrap(), "gzip");
        assert_eq!(response.headers().get("content-length").unwrap(), "3");
        assert_eq!(response.body().content_length(), Some(3));
        let mut bytes = Vec::new();
        response
            .body_mut()
            .as_reader()
            .read_to_end(&mut bytes)
            .unwrap();
        assert_eq!(bytes, "€".as_bytes());
    }

    #[test]
    fn framing_length_has_independent_complete_mode_oracles() {
        for (mode, expected) in [
            (BodyMode::NoBody, None),
            (BodyMode::LengthDelimited(3), Some(3)),
            (BodyMode::Chunked, None),
            (BodyMode::CloseDelimited, None),
        ] {
            let info = ResponseInfo::new(&crate::http::HeaderMap::new(), mode);
            assert_eq!(info.content_length(), expected);
        }
    }

    #[test]
    fn byte_reader_and_explicit_lossy_text_keep_distinct_values_and_framing() {
        for (mime, lossy, expected) in [
            ("text/plain", false, vec![0xff, b'A']),
            ("application/octet-stream", true, vec![0xff, b'A']),
            ("text/plain", true, b"?A".to_vec()),
        ] {
            let body = Body::builder()
                .mime_type(mime)
                .charset("windows-1252")
                .data(vec![0xff, b'A']);
            let mut reader = body.into_with_config().lossy_utf8(lossy).reader();
            assert_eq!(reader.body_mode(), BodyMode::LengthDelimited(2));
            let mut actual = Vec::new();
            reader.read_to_end(&mut actual).unwrap();
            assert_eq!(actual, expected);
        }
    }
}
