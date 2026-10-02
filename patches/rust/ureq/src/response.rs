use http::Uri;

use crate::body::Body;
use crate::http;

#[derive(Debug, Clone)]
pub(crate) struct ResponseUri(pub http::Uri);

#[derive(Debug, Clone)]
pub(crate) struct RedirectHistory(pub Vec<Uri>);

/// Extension trait for [`http::Response<Body>`].
///
/// Adds additional convenience methods to the `Response` that are not available
/// in the plain http API.
pub trait ResponseExt {
    /// The Uri that ultimately this Response is about.
    ///
    /// This can differ from the request uri when we have followed redirects.
    ///
    /// ```
    /// use ureq::ResponseExt;
    ///
    /// # use std::{io::{Read, Write}, net::TcpListener, thread, time::{Duration, Instant}};
    /// # let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    /// # listener.set_nonblocking(true).unwrap();
    /// # let base = format!("http://{}", listener.local_addr().unwrap());
    /// # let worker = thread::spawn(move || {
    /// #     for (target, response) in [("/redirect", "HTTP/1.1 302 Found\r\nLocation: /get\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"), ("/get", "HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")] {
    /// #         let deadline = Instant::now() + Duration::from_secs(5);
    /// #         let mut stream = loop {
    /// #             match listener.accept() {
    /// #                 Ok((stream, _)) => break stream,
    /// #                 Err(error) if error.kind() == std::io::ErrorKind::WouldBlock && Instant::now() < deadline => thread::sleep(Duration::from_millis(5)),
    /// #                 Err(error) => panic!("loopback accept: {error}"),
    /// #             }
    /// #         };
    /// #         stream.set_read_timeout(Some(Duration::from_secs(1))).unwrap();
    /// #         stream.set_write_timeout(Some(Duration::from_secs(1))).unwrap();
    /// #         let mut request = Vec::new();
    /// #         while !request.ends_with(b"\r\n\r\n") {
    /// #             assert!(request.len() < 16384 && Instant::now() < deadline);
    /// #             let mut byte = [0]; assert_eq!(stream.read(&mut byte).unwrap(), 1); request.push(byte[0]);
    /// #         }
    /// #         assert!(request.starts_with(format!("GET {target} HTTP/1.1\r\n").as_bytes()));
    /// #         stream.write_all(response.as_bytes()).unwrap();
    /// #     }
    /// # });
    /// let initial = format!("{base}/redirect");
    /// let res = ureq::get(&initial)
    ///     .config().proxy(None).timeout_global(Some(Duration::from_secs(5)))
    ///     .build().call().unwrap();
    ///
    /// assert_eq!(res.get_uri(), format!("{base}/get").as_str());
    /// # worker.join().unwrap();
    /// ```
    fn get_uri(&self) -> &Uri;

    /// The full history of uris, including the request and final uri.
    ///
    /// Returns `None` when [`Config::save_redirect_history`][crate::config::Config::save_redirect_history]
    /// is `false`.
    ///
    ///
    /// ```
    /// # use ureq::http::Uri;
    /// use ureq::ResponseExt;
    ///
    /// # use std::{io::{Read, Write}, net::TcpListener, thread, time::{Duration, Instant}};
    /// # let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    /// # listener.set_nonblocking(true).unwrap();
    /// # let base = format!("http://{}", listener.local_addr().unwrap());
    /// # let worker = thread::spawn(move || {
    /// #     for (target, response) in [("/redirect", "HTTP/1.1 302 Found\r\nLocation: /get\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"), ("/get", "HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")] {
    /// #         let deadline = Instant::now() + Duration::from_secs(5);
    /// #         let mut stream = loop {
    /// #             match listener.accept() {
    /// #                 Ok((stream, _)) => break stream,
    /// #                 Err(error) if error.kind() == std::io::ErrorKind::WouldBlock && Instant::now() < deadline => thread::sleep(Duration::from_millis(5)),
    /// #                 Err(error) => panic!("loopback accept: {error}"),
    /// #             }
    /// #         };
    /// #         stream.set_read_timeout(Some(Duration::from_secs(1))).unwrap();
    /// #         stream.set_write_timeout(Some(Duration::from_secs(1))).unwrap();
    /// #         let mut request = Vec::new();
    /// #         while !request.ends_with(b"\r\n\r\n") {
    /// #             assert!(request.len() < 16384 && Instant::now() < deadline);
    /// #             let mut byte = [0]; assert_eq!(stream.read(&mut byte).unwrap(), 1); request.push(byte[0]);
    /// #         }
    /// #         assert!(request.starts_with(format!("GET {target} HTTP/1.1\r\n").as_bytes()));
    /// #         stream.write_all(response.as_bytes()).unwrap();
    /// #     }
    /// # });
    /// let uri1: Uri = format!("{base}/redirect").parse().unwrap();
    /// let uri2: Uri = format!("{base}/get").parse().unwrap();
    ///
    /// let res = ureq::get(&uri1)
    ///     .config()
    ///     .save_redirect_history(true)
    ///     .proxy(None).timeout_global(Some(Duration::from_secs(5)))
    ///     .build()
    ///     .call().unwrap();
    ///
    /// let history = res.get_redirect_history().unwrap();
    ///
    /// assert_eq!(history, &[uri1, uri2]);
    /// # worker.join().unwrap();
    /// ```
    fn get_redirect_history(&self) -> Option<&[Uri]>;
}

impl ResponseExt for http::Response<Body> {
    fn get_uri(&self) -> &Uri {
        &self
            .extensions()
            .get::<ResponseUri>()
            .expect("uri to have been set")
            .0
    }

    fn get_redirect_history(&self) -> Option<&[Uri]> {
        self.extensions()
            .get::<RedirectHistory>()
            .map(|r| r.0.as_ref())
    }
}
