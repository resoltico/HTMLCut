#![cfg_attr(not(test), cfg_attr(feature = "fuzzing", no_main))]
#[cfg(all(feature = "fuzzing", not(test)))]
use libfuzzer_sys::fuzz_target;

#[cfg(all(feature = "fuzzing", not(test)))]
#[derive(Debug, PartialEq, Eq)]
struct FinalResponse {
    version: ureq_proto::http::Version,
    status: u16,
    headers: Vec<(String, Vec<u8>)>,
}

#[cfg(all(feature = "fuzzing", not(test)))]
fn outcome(wire: &[u8], chunk: usize) -> Result<Option<FinalResponse>, String> {
    use ureq_proto::{
        client::{Call, SendRequestResult},
        http::Request,
    };
    let request = Request::get("http://fixture.invalid/").body(()).unwrap();
    let mut request = Call::new(request).unwrap().proceed();
    request.write(&mut [0; 1024]).unwrap();
    let mut response = match request.proceed().unwrap().unwrap() {
        SendRequestResult::RecvResponse(response) => response,
        _ => panic!("bodyless GET must receive response"),
    };
    let mut pending = Vec::new();
    for bytes in wire.chunks(chunk.max(1)) {
        pending.extend_from_slice(bytes);
        loop {
            let (used, value) = response
                .try_response(&pending, false)
                .map_err(|error| error.to_string())?;
            assert!(used <= pending.len());
            if let Some(value) = value {
                let headers = value
                    .headers()
                    .iter()
                    .map(|(name, value)| (name.to_string(), value.as_bytes().to_vec()))
                    .collect();
                return Ok(Some(FinalResponse {
                    version: value.version(),
                    status: value.status().as_u16(),
                    headers,
                }));
            }
            if used == 0 {
                break;
            }
            pending.drain(..used);
        }
    }
    Ok(None)
}

#[cfg(all(feature = "fuzzing", not(test)))]
fuzz_target!(|data: &[u8]| {
    let count = usize::from(data.first().copied().unwrap_or(0) % 40);
    let padding = usize::from(data.get(1).copied().unwrap_or(0)) * 16;
    let mut generated = Vec::new();
    for index in 0..count {
        let status = if index % 2 == 0 {
            "100 Continue"
        } else {
            "103 Early Hints"
        };
        generated.extend_from_slice(
            format!(
                "HTTP/1.1 {status}\r\nX-Note: {}\r\n\r\n",
                "x".repeat(padding)
            )
            .as_bytes(),
        );
    }
    generated.extend_from_slice(
        b"HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: 0\r\n\r\n",
    );
    let whole = outcome(&generated, generated.len());
    assert_eq!(
        outcome(
            &generated,
            1 + usize::from(data.get(2).copied().unwrap_or(0))
        ),
        whole
    );
    if count <= 32 && generated.len() <= 65536 {
        assert_eq!(
            whole.unwrap().unwrap(),
            FinalResponse {
                version: ureq_proto::http::Version::HTTP_11,
                status: 200,
                headers: vec![
                    ("content-type".into(), b"text/html; charset=utf-8".to_vec()),
                    ("content-length".into(), b"0".to_vec())
                ]
            }
        );
    } else {
        assert!(whole.is_err());
    }
    let raw = &data[..data.len().min(4096)];
    assert_eq!(outcome(raw, 1 + count), outcome(raw, raw.len().max(1)));
});
#[cfg(any(test, not(feature = "fuzzing")))]
fn main() {}
