//! Complete response acquisition with finite transport/header work.
use super::*;

pub(super) fn recv_response(
    mut call: Call<RecvResponse>,
    connection: &mut Connection,
    config: &Config,
    timings: &mut CallTimings,
) -> Result<(Response<()>, RecvResponseResult), Error> {
    let response = loop {
        let timeout = timings.next_timeout(Timeout::RecvResponse);
        let made_progress = match connection.maybe_await_input(timeout) {
            Ok(progress) => progress,
            // Both EOF representations leave the complete-response parser authoritative.
            Err(Error::Io(error)) if error.kind() == io::ErrorKind::UnexpectedEof => false,
            Err(error) => return Err(error),
        };

        let input = connection.buffers().input();

        let (amount, maybe_response) = call.try_response(input)?;

        let check_size = if maybe_response.is_some() {
            // We got a parsed response, ensure the size is within
            // configured parameters.
            amount
        } else {
            // We did not parse a response, if input is too large,
            // we stop trying to get more data.
            input.len()
        };

        if check_size > config.max_response_header_size() {
            return Err(Error::LargeResponseHeader(
                input.len(),
                config.max_response_header_size(),
            ));
        }

        connection.consume_input(amount);

        if let Some(response) = maybe_response {
            assert!(call.can_proceed());
            break response;
        } else if !made_progress {
            return Err(Error::Protocol(ureq_proto::Error::HttpParseFail(
                "incomplete response header section".into(),
            )));
        }
    };

    timings.record_time(Timeout::RecvResponse);
    Ok((response, call.proceed().unwrap()))
}

#[cfg(test)]
#[path = "response_tests.rs"]
mod tests;
