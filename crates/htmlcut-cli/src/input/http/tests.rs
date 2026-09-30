use super::*;
use std::cell::Cell;
use std::collections::VecDeque;
use std::io::Cursor;

struct TestClock {
    base: Instant,
    calls: Cell<usize>,
    expire_at: usize,
}
impl Clock for TestClock {
    fn now(&self) -> Instant {
        let call = self.calls.get() + 1;
        self.calls.set(call);
        self.base
            + if call >= self.expire_at {
                Duration::from_secs(30)
            } else {
                Duration::ZERO
            }
    }
}
fn clock() -> TestClock {
    TestClock {
        base: Instant::now(),
        calls: Cell::new(0),
        expire_at: usize::MAX,
    }
}
struct Queue {
    replies: VecDeque<Result<Response, ExtractionError>>,
    urls: Vec<String>,
}
impl Transport for Queue {
    fn get(
        &mut self,
        url: &Url,
        remaining: Duration,
        connect: Duration,
    ) -> Result<Response, ExtractionError> {
        assert!(!remaining.is_zero());
        assert!(!connect.is_zero());
        self.urls.push(url.to_string());
        self.replies.pop_front().unwrap()
    }
}
fn response(status: u16, location: Option<&str>, body: &[u8]) -> Result<Response, ExtractionError> {
    Ok(Response {
        status,
        location: location.map(str::to_owned),
        content_type: None,
        encoding: None,
        body: Box::new(Cursor::new(body.to_vec())),
    })
}
fn queue(replies: Vec<Result<Response, ExtractionError>>) -> Queue {
    Queue {
        replies: replies.into(),
        urls: Vec::new(),
    }
}

#[test]
fn redirects_keep_query_fragments_and_final_base_without_retries() {
    let mut transport = queue(vec![
        response(302, Some("/next?q=2#fragment"), b""),
        response(200, None, b"body"),
    ]);
    let fetched = fetch_with(
        "http://example.test/start?q=1#fragment",
        false,
        Policy::default(),
        &clock(),
        &mut transport,
    )
    .unwrap();
    assert_eq!(
        transport.urls,
        [
            "http://example.test/start?q=1",
            "http://example.test/next?q=2"
        ]
    );
    assert_eq!(fetched.final_url, "http://example.test/next?q=2");
    assert_eq!(fetched.bytes, b"body");
    let mut transport = queue(vec![
        response(302, Some("http://other.test/path?token=SYNTHETIC"), b""),
        response(200, None, b"body"),
    ]);
    assert!(
        fetch_with(
            "http://example.test/",
            false,
            Policy::default(),
            &clock(),
            &mut transport
        )
        .is_ok()
    );
    assert_eq!(transport.urls.len(), 2);
    for (start, location, code) in [
        (
            "https://example.test/",
            Some("http://other.test/"),
            ErrorCode::InvalidOptions,
        ),
        ("http://example.test/", None, ErrorCode::Acquisition),
        (
            "http://example.test/",
            Some("http://["),
            ErrorCode::Acquisition,
        ),
        (
            "http://example.test/",
            Some("http://user:secret@example.test/"),
            ErrorCode::InvalidOptions,
        ),
    ] {
        let mut transport = queue(vec![response(302, location, b"")]);
        let error = fetch_with(start, false, Policy::default(), &clock(), &mut transport)
            .err()
            .unwrap();
        assert_eq!(error.code, code);
        assert!(!error.message.contains("secret"));
        assert_eq!(transport.urls.len(), 1);
    }
    let mut transport = queue((0..6).map(|_| response(302, Some("/loop"), b"")).collect());
    assert_eq!(
        fetch_with(
            "http://example.test/",
            false,
            Policy::default(),
            &clock(),
            &mut transport
        )
        .err()
        .unwrap()
        .code,
        ErrorCode::ResourceLimit
    );
    assert_eq!(transport.urls.len(), 6);
    let mut transport = queue(vec![response(404, None, b"not a success")]);
    assert_eq!(
        fetch_with(
            "http://example.test/",
            false,
            Policy::default(),
            &clock(),
            &mut transport
        )
        .err()
        .unwrap()
        .code,
        ErrorCode::Acquisition
    );
}

#[test]
fn transfer_decompression_and_deadline_triplets_fail_closed() {
    for (maximum, accepted) in [(3, false), (4, true), (5, true)] {
        let policy = Policy {
            transfer_bytes: maximum,
            ..Default::default()
        };
        let mut transport = queue(vec![response(200, None, b"body")]);
        assert_eq!(
            fetch_with(
                "http://example.test/",
                false,
                policy,
                &clock(),
                &mut transport
            )
            .is_ok(),
            accepted
        );
        let policy = Policy {
            decompressed_bytes: maximum,
            ..Default::default()
        };
        let mut transport = queue(vec![response(200, None, b"body")]);
        assert_eq!(
            fetch_with(
                "http://example.test/",
                false,
                policy,
                &clock(),
                &mut transport
            )
            .is_ok(),
            accepted
        );
    }
    let mut compressed = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    std::io::Write::write_all(&mut compressed, b"expanded body").unwrap();
    let compressed = compressed.finish().unwrap();
    for (maximum, accepted) in [(12, false), (13, true), (14, true)] {
        let mut reply = response(200, None, &compressed).unwrap();
        reply.encoding = Some("gzip".into());
        let mut transport = queue(vec![Ok(reply)]);
        let policy = Policy {
            decompressed_bytes: maximum,
            ..Default::default()
        };
        assert_eq!(
            fetch_with(
                "http://example.test/",
                false,
                policy,
                &clock(),
                &mut transport
            )
            .is_ok(),
            accepted
        );
    }
    for expiry in [2, 3, 5] {
        let test_clock = TestClock {
            expire_at: expiry,
            ..clock()
        };
        let mut transport = queue(vec![response(200, None, b"body")]);
        assert_eq!(
            fetch_with(
                "http://example.test/",
                false,
                Policy::default(),
                &test_clock,
                &mut transport
            )
            .err()
            .unwrap()
            .code,
            ErrorCode::ResourceLimit
        );
    }
    let mut transport = queue(vec![Err(transport_error(ureq::Error::Timeout(
        ureq::Timeout::Global,
    )))]);
    assert_eq!(
        fetch_with(
            "http://example.test/",
            false,
            Policy::default(),
            &clock(),
            &mut transport
        )
        .err()
        .unwrap()
        .code,
        ErrorCode::ResourceLimit
    );
}

#[test]
fn explicit_encoding_priority_and_invalid_compression_are_unambiguous() {
    assert_eq!(
        charset(Some(
            "text/html; foo; other=x; CHARSET=\"UTF-8\"; charset=utf-8"
        ))
        .unwrap(),
        Some("utf-8".into())
    );
    assert!(charset(Some("text/html; charset=utf-8; charset=windows-1252")).is_err());
    let mut reply = response(200, None, b"body").unwrap();
    reply.content_type = Some("text/html; charset=utf-8; charset=windows-1252".into());
    let mut transport = queue(vec![Ok(reply)]);
    assert!(
        fetch_with(
            "http://example.test/",
            true,
            Policy::default(),
            &clock(),
            &mut transport
        )
        .unwrap()
        .charset
        .is_none()
    );
    let mut reply = response(200, None, b"body").unwrap();
    reply.encoding = Some("unsupported".into());
    let mut transport = queue(vec![Ok(reply)]);
    assert_eq!(
        fetch_with(
            "http://example.test/",
            false,
            Policy::default(),
            &clock(),
            &mut transport
        )
        .err()
        .unwrap()
        .code,
        ErrorCode::Acquisition
    );
    let mut transport = queue(vec![Err(transport_error(ureq::Error::BadUri(
        "synthetic detail".into(),
    )))]);
    assert_eq!(
        fetch_with(
            "http://example.test/",
            false,
            Policy::default(),
            &clock(),
            &mut transport
        )
        .err()
        .unwrap()
        .code,
        ErrorCode::Acquisition
    );
    for url in ["bad", "file:///tmp/a", "http://user:secret@example.test/"] {
        assert!(validated_url(url).is_err());
    }
}

#[test]
fn invalid_adapter_policies_do_not_send_requests() {
    let hard = Policy::default();
    for policy in [
        Policy {
            redirects: 6,
            ..hard
        },
        Policy {
            transfer_bytes: 0,
            ..hard
        },
        Policy {
            transfer_bytes: MAX_TRANSFER + 1,
            ..hard
        },
        Policy {
            decompressed_bytes: 0,
            ..hard
        },
        Policy {
            decompressed_bytes: MAX_SOURCE_BYTES + 1,
            ..hard
        },
        Policy {
            connect: Duration::ZERO,
            ..hard
        },
        Policy {
            connect: Duration::from_secs(6),
            ..hard
        },
        Policy {
            total: Duration::ZERO,
            ..hard
        },
        Policy {
            total: Duration::from_secs(16),
            ..hard
        },
    ] {
        let mut transport = queue(vec![]);
        assert_eq!(
            fetch_with(
                "http://example.test/",
                false,
                policy,
                &clock(),
                &mut transport
            )
            .err()
            .unwrap()
            .code,
            ErrorCode::InvalidOptions
        );
        assert!(transport.urls.is_empty());
    }
}

#[test]
fn transfer_state_and_deadline_errors_remain_sticky() {
    let now = clock();
    let mut transfer = Transfer {
        inner: Cursor::new(b"abcd"),
        read: 0,
        deadline: now.base + Duration::from_secs(10),
        maximum: 3,
        clock: &now,
    };
    let mut buffer = [0; 8];
    assert!(transfer.read(&mut buffer).is_err());
    assert!(transfer.read(&mut buffer).is_err());
    let elapsed = TestClock {
        expire_at: 1,
        ..clock()
    };
    let mut transfer = Transfer {
        inner: Cursor::new(b"body"),
        read: 0,
        deadline: elapsed.base + Duration::from_secs(1),
        maximum: 100,
        clock: &elapsed,
    };
    assert!(transfer.read(&mut buffer).is_err());
}

#[test]
fn secure_redirects_stay_secure_and_password_only_userinfo_is_rejected() {
    assert_eq!(
        validated_url("https://:synthetic@example.test")
            .unwrap_err()
            .code,
        ErrorCode::InvalidOptions
    );
    let clock = clock();
    let mut transport = queue(vec![
        response(302, Some("https://example.test/next"), b""),
        response(200, None, b"ok"),
    ]);
    let result = fetch_with(
        "https://example.test/start",
        false,
        Policy::default(),
        &clock,
        &mut transport,
    )
    .unwrap();
    assert_eq!(result.bytes, b"ok");
    assert_eq!(result.final_url, "https://example.test/next");
}

#[test]
fn source_url_raw_and_canonical_byte_limits_are_checked_before_network_io() {
    let prefix = "https://example.test/?q=";
    for (length, accepted) in [(8191, true), (8192, true), (8193, false)] {
        let url = format!("{prefix}{}", "x".repeat(length - prefix.len()));
        assert_eq!(validated_url(&url).is_ok(), accepted);
    }
    let url = format!("{prefix}{}", "é".repeat(2000));
    assert!(url.len() < MAX_SOURCE_URL_BYTES);
    assert_eq!(
        validated_url(&url).unwrap_err().code,
        ErrorCode::ResourceLimit
    );
    let mut transport = queue(vec![]);
    assert_eq!(
        fetch_with(&url, false, Policy::default(), &clock(), &mut transport)
            .err()
            .unwrap()
            .code,
        ErrorCode::ResourceLimit
    );
    assert!(transport.urls.is_empty());
}

#[test]
fn default_http_transfer_and_decode_bounds_match_accepted_source_capacity() {
    let policy = Policy::default();
    let source_bound = htmlcut_core::PreparationLimits::default().max_source_bytes as usize;
    assert_eq!(policy.transfer_bytes, source_bound);
    assert_eq!(policy.decompressed_bytes, source_bound);
}
