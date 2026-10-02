use http::StatusCode;

use crate::Error;
use crate::body::BodyWriter;
use crate::parser::try_parse_response;

use super::state::Await100;
use super::{Await100Result, Call};
use crate::CloseReason;

impl Call<Await100> {
    /// Attempt to read a 100-continue response.
    ///
    /// Tries to interpret bytes sent by the server as a 100-continue response. The expect-100 mechanic
    /// means we hope the server will give us an indication on whether to upload a potentially big
    /// request body, before we start doing it.
    ///
    /// * If the server supports expect-100, it will respond `HTTP/1.1 100 Continue\r\n\r\n`, or
    ///   some other response code (such as 403) if we are not allowed to post the body.
    /// * If the server does not support expect-100, it will not respond at all, in which case
    ///   we will proceed to sending the request body after some timeout.
    ///
    /// The results are:
    ///
    /// * `Ok(0)` - not enough data yet, continue waiting (or `proceed()` if you think we waited enough)
    /// * `Ok(n)` - `n` number of input bytes were consumed. Call `proceed()` next
    /// * `Err(e)` - some error that is not recoverable
    pub fn try_read_100(&mut self, input: &[u8]) -> Result<usize, Error> {
        let Some((input_used, response)) =
            try_parse_response::<{ super::MAX_RESPONSE_HEADERS }>(input)?
        else {
            self.inner.response_limits.pending(input.len())?;
            return Ok(0);
        };
        let status = response.status();
        if status.is_informational() && status != StatusCode::SWITCHING_PROTOCOLS {
            self.inner.response_limits.complete(input_used, true)?;
            if status == StatusCode::CONTINUE {
                self.inner.await_100_continue = false;
                assert!(self.inner.state.writer.has_body());
            }
            return Ok(input_used);
        }
        // The final response is left in the buffer for RecvResponse to consume and meter once.
        self.inner.response_limits.pending(input_used)?;
        self.inner.await_100_continue = false;
        self.inner
            .close_reason
            .push(if status == StatusCode::SWITCHING_PROTOCOLS {
                CloseReason::ProtocolSwitch
            } else {
                CloseReason::Not100Continue
            });
        self.inner.state.writer = BodyWriter::new_none();
        Ok(0)
    }

    /// Tell if there is any point in waiting for more data from the server.
    ///
    /// Becomes `false` as soon as `try_read_100()` got enough data to determine what to do next.
    /// This might become `false` even if `try_read_100` returns `Ok(0)`.
    ///
    /// If this returns `false`, the user should continue with `proceed()`.
    pub fn can_keep_await_100(&self) -> bool {
        self.inner.await_100_continue
    }

    /// Proceed to the next state.
    pub fn proceed(self) -> Result<Await100Result, Error> {
        // We can always proceed out of Await100

        if self.inner.state.writer.has_body() {
            // TODO(martin): do i need this?
            // call.inner.call.analyze_request()?;
            let call = Call::wrap(self.inner);
            Ok(Await100Result::SendBody(call))
        } else {
            Ok(Await100Result::RecvResponse(Call::wrap(self.inner)))
        }
    }
}
