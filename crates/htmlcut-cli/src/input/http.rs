//! Bounded GET-only HTTP with adapter-owned deadlines and compression.

use std::io::Read;
use std::time::{Duration, Instant};

use url::Url;

use super::*;
use crate::media_type;

#[cfg(test)]
#[path = "http/tests.rs"]
mod tests;

const MAX_REDIRECTS: usize = 5;
const MAX_TRANSFER: usize = 50 * 1024 * 1024;
// Match the accepted snapshot metadata bound before any request is sent.
const MAX_SOURCE_URL_BYTES: usize = 8 * 1024;

pub(super) fn validated_url(value: &str) -> Result<Url, ExtractionError> {
    if value.len() > MAX_SOURCE_URL_BYTES {
        return Err(limit("acquisition"));
    }
    let mut url = Url::parse(value).map_err(|_| options("The HTTP source URL is invalid."))?;
    if !matches!(url.scheme(), "http" | "https")
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err(options(
            "HTTP sources require an absolute HTTP(S) URL without embedded userinfo.",
        ));
    }
    url.set_fragment(None);
    if url.as_str().len() > MAX_SOURCE_URL_BYTES {
        return Err(limit("acquisition"));
    }
    Ok(url)
}

#[derive(Clone, Copy)]
struct Policy {
    redirects: usize,
    transfer_bytes: usize,
    decompressed_bytes: usize,
    connect: Duration,
    total: Duration,
}
impl Default for Policy {
    fn default() -> Self {
        Self {
            redirects: MAX_REDIRECTS,
            transfer_bytes: MAX_TRANSFER,
            decompressed_bytes: MAX_SOURCE_BYTES,
            connect: Duration::from_millis(5000),
            total: Duration::from_millis(15000),
        }
    }
}

trait Clock {
    fn now(&self) -> Instant;
}
struct SystemClock;
impl Clock for SystemClock {
    fn now(&self) -> Instant {
        Instant::now()
    }
}

struct Response {
    status: u16,
    location: Option<String>,
    content_type: Option<Vec<u8>>,
    duplicate_content_type: bool,
    partial: bool,
    encoding: Option<String>,
    body: Box<dyn Read>,
}
trait Transport {
    fn get(
        &mut self,
        url: &Url,
        remaining: Duration,
        connect: Duration,
    ) -> Result<Response, ExtractionError>;
}
struct HttpTransport;
impl Transport for HttpTransport {
    fn get(
        &mut self,
        url: &Url,
        remaining: Duration,
        connect: Duration,
    ) -> Result<Response, ExtractionError> {
        let tls = ureq::tls::TlsConfig::builder()
            .root_certs(ureq::tls::RootCerts::PlatformVerifier)
            .build();
        let config = ureq::Agent::config_builder()
            .tls_config(tls)
            .http_status_as_error(false)
            .max_redirects(0)
            .max_redirects_will_error(false)
            .timeout_global(Some(remaining))
            .timeout_connect(Some(remaining.min(connect)))
            .build();
        let agent: ureq::Agent = config.into();
        let response = agent
            .get(url.as_str())
            .header("accept-encoding", "gzip")
            .call()
            .map_err(transport_error)?;
        let header = |name| {
            response
                .headers()
                .get(name)
                .map(|v| v.to_str().map(str::to_owned))
                .transpose()
                .map_err(|_| acquisition())
        };
        Ok(Response {
            status: response.status().as_u16(),
            location: header("location")?,
            content_type: response
                .headers()
                .get("content-type")
                .map(|value| value.as_bytes().to_vec()),
            duplicate_content_type: response.headers().get_all("content-type").iter().count() > 1,
            partial: response.headers().contains_key("content-range"),
            encoding: header("content-encoding")?,
            body: Box::new(response.into_body().into_with_config().reader()),
        })
    }
}
fn transport_error(error: ureq::Error) -> ExtractionError {
    use htmlcut_core::{FailureCause, TransportProblem};
    match error {
        ureq::Error::Protocol(
            ureq_proto::Error::ResponseHeaderLimit
            | ureq_proto::Error::InformationalResponseLimit
            | ureq_proto::Error::HttpParseTooManyHeaders,
        )
        | ureq::Error::LargeResponseHeader(..) => limit("acquisition"),
        ureq::Error::Timeout(_) => limit("acquisition").with_cause(FailureCause::Transport {
            problem: TransportProblem::Timeout,
        }),
        ureq::Error::HostNotFound => acquisition().with_cause(FailureCause::Transport {
            problem: TransportProblem::Dns,
        }),
        ureq::Error::ConnectionFailed => acquisition().with_cause(FailureCause::Transport {
            problem: TransportProblem::Connection,
        }),
        ureq::Error::Tls(_) | ureq::Error::Rustls(_) => {
            acquisition().with_cause(FailureCause::Transport {
                problem: TransportProblem::Tls,
            })
        }
        ureq::Error::Protocol(_) => acquisition().with_cause(FailureCause::Transport {
            problem: TransportProblem::Framing,
        }),
        ureq::Error::Io(error) => {
            let problem = match error.kind() {
                std::io::ErrorKind::UnexpectedEof => TransportProblem::IncompleteBody,
                std::io::ErrorKind::ConnectionRefused
                | std::io::ErrorKind::ConnectionReset
                | std::io::ErrorKind::ConnectionAborted => TransportProblem::Connection,
                _ => TransportProblem::Other,
            };
            acquisition().with_cause(FailureCause::Transport { problem })
        }
        _ => acquisition().with_cause(FailureCause::Transport {
            problem: TransportProblem::Other,
        }),
    }
}

pub(super) struct Acquired {
    pub(super) bytes: Vec<u8>,
    pub(super) charset: Option<String>,
    pub(super) final_url: String,
    pub(super) deadline: Instant,
}

pub(super) fn fetch(value: &str, explicit_encoding: bool) -> Result<Acquired, ExtractionError> {
    fetch_with(
        value,
        explicit_encoding,
        Policy::default(),
        &SystemClock,
        &mut HttpTransport,
    )
}

fn fetch_with(
    value: &str,
    explicit_encoding: bool,
    policy: Policy,
    clock: &impl Clock,
    transport: &mut impl Transport,
) -> Result<Acquired, ExtractionError> {
    let hard = Policy::default();
    if policy.redirects > hard.redirects
        || policy.transfer_bytes == 0
        || policy.transfer_bytes > hard.transfer_bytes
        || policy.decompressed_bytes == 0
        || policy.decompressed_bytes > hard.decompressed_bytes
        || policy.connect.is_zero()
        || policy.connect > hard.connect
        || policy.total.is_zero()
        || policy.total > hard.total
    {
        return Err(options(
            "HTTP acquisition limits are outside the supported range.",
        ));
    }
    let mut url = validated_url(value)?;
    let deadline = clock.now() + policy.total;
    let mut redirects = 0;
    loop {
        let remaining = deadline
            .checked_duration_since(clock.now())
            .filter(|value| !value.is_zero())
            .ok_or_else(|| limit("acquisition"))?;
        let response = transport.get(&url, remaining, policy.connect)?;
        if matches!(response.status, 301 | 302 | 303 | 307 | 308) {
            if redirects == policy.redirects {
                return Err(limit("redirect"));
            }
            let location = response.location.as_deref().ok_or_else(acquisition)?;
            let next = url.join(location).map_err(|_| acquisition())?;
            if url.scheme() == "https" && next.scheme() == "http" {
                return Err(options(
                    "HTTPS-to-HTTP redirects require a policy this invocation has not enabled.",
                ));
            }
            // A fresh GET is constructed for every redirect; no caller authentication/cookie
            // headers are carried by this adapter, and redirected userinfo is rejected.
            url = validated_url(next.as_str())?;
            redirects += 1;
            continue;
        }
        if response.status == 206 || ((200..300).contains(&response.status) && response.partial) {
            return Err(
                acquisition().with_cause(htmlcut_core::FailureCause::PartialResponse {
                    status: response.status,
                }),
            );
        }
        if !(200..300).contains(&response.status) {
            return Err(
                acquisition().with_cause(htmlcut_core::FailureCause::HttpStatus {
                    status: response.status,
                }),
            );
        }
        let charset = if explicit_encoding {
            None
        } else {
            if response.duplicate_content_type {
                return Err(acquisition().with_cause(htmlcut_core::FailureCause::MediaType {}));
            }
            charset(response.content_type.as_deref())?
        };
        let compression = response
            .encoding
            .as_deref()
            .unwrap_or("identity")
            .trim()
            .to_ascii_lowercase();
        let mut transfer = Transfer {
            inner: response.body,
            read: 0,
            deadline,
            maximum: policy.transfer_bytes,
            clock,
        };
        let body = match compression.as_str() {
            "identity" | "" => read_bounded(&mut transfer, policy.decompressed_bytes),
            "gzip" => read_bounded(
                &mut flate2::read::MultiGzDecoder::new(&mut transfer),
                policy.decompressed_bytes,
            ),
            _ => return Err(acquisition().with_cause(htmlcut_core::FailureCause::Compression {})),
        };
        if transfer.read > policy.transfer_bytes || clock.now() >= deadline {
            return Err(limit("acquisition"));
        }
        let body = body.map_err(|error| {
            if error.code == ErrorCode::ResourceLimit {
                error
            } else if compression == "gzip" {
                acquisition().with_cause(htmlcut_core::FailureCause::Compression {})
            } else {
                acquisition().with_cause(htmlcut_core::FailureCause::Transport {
                    problem: htmlcut_core::TransportProblem::IncompleteBody,
                })
            }
        })?;
        return Ok(Acquired {
            bytes: body,
            charset,
            final_url: url.into(),
            deadline,
        });
    }
}

fn charset(content_type: Option<&[u8]>) -> Result<Option<String>, ExtractionError> {
    media_type::charset(content_type)
}

struct Transfer<'a, R, C> {
    inner: R,
    read: usize,
    deadline: Instant,
    maximum: usize,
    clock: &'a C,
}

impl<R: Read, C: Clock> Read for Transfer<'_, R, C> {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        if self.clock.now() >= self.deadline || self.read > self.maximum {
            return Err(std::io::Error::other(
                "Acquisition resource limit exceeded.",
            ));
        }
        let size = self.inner.read(buffer)?;
        self.read = self
            .read
            .checked_add(size)
            .ok_or_else(|| std::io::Error::other("Transfer counter overflow."))?;
        if self.read > self.maximum {
            return Err(std::io::Error::other("Transfer byte limit exceeded."));
        }
        Ok(size)
    }
}
