//! Bounded GET-only HTTP with adapter-owned deadlines and compression.

use std::io::Read;
use std::time::{Duration, Instant};

use url::Url;

use super::*;

const MAX_REDIRECTS: usize = 5;
const MAX_TRANSFER: usize = 50 * 1024 * 1024;

pub(super) fn validated_url(value: &str) -> Result<Url, ExtractionError> {
    let mut url = Url::parse(value).map_err(|_| options("The HTTP source URL is invalid."))?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err(options(
            "HTTP sources require an absolute HTTP(S) URL without embedded userinfo.",
        ));
    }
    url.set_fragment(None);
    Ok(url)
}

pub(super) fn fetch(value: &str) -> Result<(Vec<u8>, Option<String>, String), ExtractionError> {
    let mut url = validated_url(value)?;
    let deadline = Instant::now() + Duration::from_millis(15_000);
    for redirects in 0..=MAX_REDIRECTS {
        let remaining = deadline
            .checked_duration_since(Instant::now())
            .ok_or_else(|| limit("acquisition"))?;
        let config = ureq::Agent::config_builder()
            .http_status_as_error(false)
            .max_redirects(0)
            .max_redirects_will_error(false)
            .timeout_global(Some(remaining))
            .timeout_connect(Some(remaining.min(Duration::from_millis(5_000))))
            .build();
        let agent: ureq::Agent = config.into();
        let mut response = agent.get(url.as_str()).call().map_err(|error| {
            if matches!(error, ureq::Error::Timeout(_)) {
                limit("acquisition")
            } else {
                acquisition()
            }
        })?;
        let status = response.status().as_u16();
        if matches!(status, 301 | 302 | 303 | 307 | 308) {
            if redirects == MAX_REDIRECTS {
                return Err(limit("redirect"));
            }
            let location = response
                .headers()
                .get("location")
                .and_then(|value| value.to_str().ok())
                .ok_or_else(acquisition)?;
            let next = url.join(location).map_err(|_| acquisition())?;
            if url.scheme() == "https" && next.scheme() == "http" {
                return Err(options(
                    "HTTPS-to-HTTP redirects require a policy this invocation has not enabled.",
                ));
            }
            // Every redirected request is built afresh; no authorization/cookie headers exist
            // in this adapter to forward across origins, and userinfo is rejected again.
            url = validated_url(next.as_str())?;
            continue;
        }
        if !(200..300).contains(&status) {
            return Err(acquisition());
        }
        let charset = charset(
            response
                .headers()
                .get("content-type")
                .map(|v| v.to_str())
                .transpose()
                .map_err(|_| acquisition())?,
        )?;
        let compression = response
            .headers()
            .get("content-encoding")
            .map(|v| v.to_str())
            .transpose()
            .map_err(|_| acquisition())?
            .unwrap_or("identity")
            .to_ascii_lowercase();
        let reader = response.body_mut().as_reader();
        let mut transfer = Transfer {
            inner: reader,
            read: 0,
            deadline,
        };
        let body = match compression.trim() {
            "identity" | "" => read_bounded(&mut transfer, MAX_SOURCE_BYTES),
            "gzip" => read_bounded(
                &mut flate2::read::MultiGzDecoder::new(&mut transfer),
                MAX_SOURCE_BYTES,
            ),
            _ => return Err(acquisition()),
        };
        if transfer.read > MAX_TRANSFER || Instant::now() >= deadline {
            return Err(limit("acquisition"));
        }
        return Ok((body?, charset, url.into()));
    }
    Err(limit("redirect"))
}

fn charset(content_type: Option<&str>) -> Result<Option<String>, ExtractionError> {
    let mut charset = None;
    for part in content_type.unwrap_or("").split(';').skip(1) {
        let Some((name, value)) = part.split_once('=') else {
            continue;
        };
        if !name.trim().eq_ignore_ascii_case("charset") {
            continue;
        }
        let value = value.trim().trim_matches('"').to_ascii_lowercase();
        if charset.as_ref().is_some_and(|previous| previous != &value) {
            return Err(decoding());
        }
        charset = Some(value);
    }
    Ok(charset)
}

struct Transfer<R> {
    inner: R,
    read: usize,
    deadline: Instant,
}

impl<R: Read> Read for Transfer<R> {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        if Instant::now() >= self.deadline || self.read > MAX_TRANSFER {
            return Err(std::io::Error::other(
                "Acquisition resource limit exceeded.",
            ));
        }
        let size = self.inner.read(buffer)?;
        self.read = self
            .read
            .checked_add(size)
            .ok_or_else(|| std::io::Error::other("Transfer counter overflow."))?;
        if self.read > MAX_TRANSFER {
            return Err(std::io::Error::other("Transfer byte limit exceeded."));
        }
        Ok(size)
    }
}
