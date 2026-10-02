use crate::client::Await100Result;

use super::scenario::Scenario;

#[test]
fn proceed_without_100_continue() {
    let scenario = Scenario::builder()
        .put("https://q.test")
        .header("expect", "100-continue")
        .build();

    let call = scenario.to_await_100();

    assert!(call.can_keep_await_100());

    let inner = call.inner();
    assert!(inner.state.writer.has_body());
    assert!(inner.close_reason.is_empty());

    match call.proceed() {
        Ok(Await100Result::SendBody(_)) => {}
        _ => panic!("proceed without 100-continue should go to SendBody"),
    }
}

#[test]
fn proceed_after_100_continue() {
    let scenario = Scenario::builder()
        .put("https://q.test")
        .header("expect", "100-continue")
        .build();

    let mut call = scenario.to_await_100();

    let input = b"HTTP/1.1 100 Continue\r\n\r\n";
    let n = call.try_read_100(input).unwrap();
    assert_eq!(n, 25);

    assert!(!call.can_keep_await_100());

    let inner = call.inner();
    assert!(inner.state.writer.has_body());
    assert!(inner.close_reason.is_empty());

    match call.proceed() {
        Ok(Await100Result::SendBody(_)) => {}
        _ => panic!("proceed after 100-continue should go to SendBody"),
    }
}

#[test]
fn proceed_after_403() {
    let scenario = Scenario::builder()
        .put("https://q.test")
        .header("expect", "100-continue")
        .build();

    let mut call = scenario.to_await_100();

    let input = b"HTTP/1.1 403 Forbidden\r\n\r\n";
    let n = call.try_read_100(input).unwrap();
    assert_eq!(n, 0);

    assert!(!call.can_keep_await_100());

    let inner = call.inner();
    assert!(!inner.state.writer.has_body());
    assert!(!inner.close_reason.is_empty());

    match call.proceed() {
        Ok(Await100Result::RecvResponse(_)) => {}
        _ => panic!("proceed after 403 should go to RecvResponse"),
    }
}

#[test]
fn proceed_after_200() {
    let scenario = Scenario::builder()
        .put("https://q.test")
        .header("expect", "100-continue")
        .build();

    let mut call = scenario.to_await_100();

    let input = b"HTTP/1.1 200 Ok\r\n\r\n";
    let n = call.try_read_100(input).unwrap();
    assert_eq!(n, 0);

    assert!(!call.can_keep_await_100());

    let inner = call.inner();
    assert!(!inner.state.writer.has_body());
    assert!(!inner.close_reason.is_empty());

    match call.proceed() {
        Ok(Await100Result::RecvResponse(_)) => {}
        _ => panic!("proceed after 200 should go to RecvResponse"),
    }
}

#[test]
fn proceed_after_403_with_headers() {
    let scenario = Scenario::builder()
        .put("https://q.test")
        .header("expect", "100-continue")
        .build();

    let mut call = scenario.to_await_100();

    let input = b"HTTP/1.1 403 Forbidden\r\nContent-Length: 100\r\n\r\n";
    let n = call.try_read_100(input).unwrap();
    assert_eq!(n, 0);

    assert!(!call.can_keep_await_100());

    let inner = call.inner();
    assert!(!inner.state.writer.has_body());
    assert!(!inner.close_reason.is_empty());

    match call.proceed() {
        Ok(Await100Result::RecvResponse(_)) => {}
        _ => panic!("proceed after 403 should go to RecvResponse"),
    }
}

#[test]
fn consume_102_processing_and_continue_waiting() {
    let scenario = Scenario::builder()
        .put("https://q.test")
        .header("expect", "100-continue")
        .build();

    let mut call = scenario.to_await_100();

    // Server sends 102 Processing
    let input = b"HTTP/1.1 102 Processing\r\n\r\n";
    let n = call.try_read_100(input).unwrap();
    assert_eq!(n, input.len(), "102 should be consumed");

    // Should still be waiting for 100 Continue
    assert!(
        call.can_keep_await_100(),
        "Should continue waiting after 102"
    );

    let inner = call.inner();
    assert!(
        inner.state.writer.has_body(),
        "Should still plan to send body"
    );
    assert!(inner.close_reason.is_empty(), "No close reason yet");
}

#[test]
fn consume_103_early_hints_and_continue_waiting() {
    let scenario = Scenario::builder()
        .put("https://q.test")
        .header("expect", "100-continue")
        .build();

    let mut call = scenario.to_await_100();

    // Server sends 103 Early Hints (without headers since we're parsing with <0>)
    let input = b"HTTP/1.1 103 Early Hints\r\n\r\n";
    let n = call.try_read_100(input).unwrap();
    assert_eq!(n, input.len(), "103 should be consumed");

    // Should still be waiting for 100 Continue
    assert!(
        call.can_keep_await_100(),
        "Should continue waiting after 103"
    );

    let inner = call.inner();
    assert!(
        inner.state.writer.has_body(),
        "Should still plan to send body"
    );
    assert!(inner.close_reason.is_empty(), "No close reason yet");
}

#[test]
fn consume_multiple_1xx_then_100_continue() {
    let scenario = Scenario::builder()
        .put("https://q.test")
        .header("expect", "100-continue")
        .build();

    let mut call = scenario.to_await_100();

    // First: 102 Processing
    let input1 = b"HTTP/1.1 102 Processing\r\n\r\n";
    let n = call.try_read_100(input1).unwrap();
    assert_eq!(n, input1.len());
    assert!(call.can_keep_await_100(), "Should keep waiting");

    // Second: 103 Early Hints
    let input2 = b"HTTP/1.1 103 Early Hints\r\n\r\n";
    let n = call.try_read_100(input2).unwrap();
    assert_eq!(n, input2.len());
    assert!(call.can_keep_await_100(), "Should keep waiting");

    // Finally: 100 Continue
    let input3 = b"HTTP/1.1 100 Continue\r\n\r\n";
    let n = call.try_read_100(input3).unwrap();
    assert_eq!(n, input3.len());
    assert!(!call.can_keep_await_100(), "Should stop waiting after 100");

    let inner = call.inner();
    assert!(inner.state.writer.has_body());
    assert!(inner.close_reason.is_empty());

    match call.proceed() {
        Ok(Await100Result::SendBody(_)) => {}
        _ => panic!("proceed after 100-continue should go to SendBody"),
    }
}

#[test]
fn switching_protocols_101_stops_body_send() {
    let scenario = Scenario::builder()
        .put("https://q.test")
        .header("expect", "100-continue")
        .build();

    let mut call = scenario.to_await_100();

    // Server sends 101 Switching Protocols
    let input = b"HTTP/1.1 101 Switching Protocols\r\n\r\n";
    let n = call.try_read_100(input).unwrap();
    assert_eq!(
        n, 0,
        "101 should not consume input, let RecvResponse handle it"
    );

    assert!(!call.can_keep_await_100(), "Should stop waiting after 101");

    let inner = call.inner();
    assert!(
        !inner.state.writer.has_body(),
        "Should not send body after 101"
    );
    assert!(
        inner
            .close_reason
            .contains(&crate::CloseReason::ProtocolSwitch),
        "Should have ProtocolSwitch close reason"
    );

    match call.proceed() {
        Ok(Await100Result::RecvResponse(_)) => {}
        _ => panic!("proceed after 101 should go to RecvResponse"),
    }
}

#[test]
fn informational_headers_preserve_await_state_and_charge_consumption_once() {
    let scenario = Scenario::builder()
        .put("https://q.test")
        .header("expect", "100-continue")
        .build();
    let mut call = scenario.to_await_100();
    let early = b"HTTP/1.1 103 Early Hints\r\nLink: </style>\r\n\r\n";
    for length in 0..early.len() {
        assert_eq!(call.try_read_100(&early[..length]).unwrap(), 0);
        assert!(call.can_keep_await_100());
    }
    assert_eq!(call.try_read_100(early).unwrap(), early.len());
    assert!(call.can_keep_await_100());
    let ready = b"HTTP/1.1 100 Continue\r\nX-Info: ready\r\n\r\n";
    assert_eq!(call.try_read_100(ready).unwrap(), ready.len());
    assert!(!call.can_keep_await_100());
    assert!(matches!(
        call.proceed().unwrap(),
        Await100Result::SendBody(_)
    ));
}

#[test]
fn await_informational_count_and_pending_bytes_are_finite() {
    let scenario = Scenario::builder()
        .put("https://q.test")
        .header("expect", "100-continue")
        .build();
    let mut call = scenario.to_await_100();
    let early = b"HTTP/1.1 103 Early Hints\r\n\r\n";
    for _ in 0..32 {
        assert_eq!(call.try_read_100(early).unwrap(), early.len());
    }
    assert_eq!(
        call.try_read_100(early),
        Err(crate::Error::InformationalResponseLimit)
    );
    let mut call = scenario.to_await_100();
    let mut partial = b"HTTP/1.1 103 Early Hints\r\nX-Info: ".to_vec();
    partial.resize(65537, b'x');
    assert_eq!(
        call.try_read_100(&partial),
        Err(crate::Error::ResponseHeaderLimit)
    );
}
