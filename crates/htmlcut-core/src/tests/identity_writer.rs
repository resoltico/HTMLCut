// SPDX-License-Identifier: MPL-2.0
use super::*;
use std::io::Write;

#[test]
fn canonical_buffer_flush_does_not_reset_or_expand_the_byte_budget() {
    let mut buffer = Buffer {
        value: Vec::new(),
        maximum: 4,
    };
    buffer.write_all("éé".as_bytes()).unwrap();
    buffer.flush().unwrap();
    assert_eq!(buffer.value, "éé".as_bytes());
    assert!(buffer.write_all(b"x").is_err());
    buffer.flush().unwrap();
    assert_eq!(buffer.value.len(), 4);
    buffer.write_all(b"").unwrap();
}

#[test]
fn retained_json_encoding_enforces_encoded_bytes_and_shared_work() {
    use selectors::work_budget::SelectorWorkBudget;
    let value = serde_json::json!(["é\n"]);
    let encoded = serde_json::to_vec(&value).unwrap();
    for maximum in [encoded.len() - 1, encoded.len(), encoded.len() + 1] {
        let budget = SelectorWorkBudget::new(10);
        assert_eq!(
            super::encoded(&value, maximum, &budget).is_ok(),
            maximum >= encoded.len()
        );
    }
    let budget = SelectorWorkBudget::new(1);
    assert!(budget.consume());
    assert_eq!(
        super::encoded(&value, 100, &budget).unwrap_err().code,
        ErrorCode::ResourceLimit
    );
    let budget = SelectorWorkBudget::new(1);
    let mut writer = JsonBuffer {
        bytes: Vec::new(),
        failure: None,
        maximum: 4,
        budget: &budget,
    };
    writer.write_all(b"abcd").unwrap();
    writer.flush().unwrap();
    assert_eq!(writer.bytes, b"abcd");
    assert_eq!(budget.remaining(), 0);
    assert!(writer.write_all(b"e").is_err());
    writer.write_all(b"").unwrap();
    assert_eq!(writer.bytes, b"abcd");
    assert_eq!(
        writer.failure.as_ref().unwrap().resource_counter.as_deref(),
        Some("encoded_bytes")
    );
}

#[test]
fn failed_encoding_keeps_first_work_owner_across_repeated_work_and_byte_failures() {
    let budget = selectors::work_budget::SelectorWorkBudget::new(1);
    assert!(budget.consume());
    let mut writer = JsonBuffer {
        bytes: Vec::new(),
        maximum: 1,
        budget: &budget,
        failure: None,
    };
    for bytes in [b"a".as_slice(), b"b".as_slice(), b"xx".as_slice()] {
        assert!(writer.write_all(bytes).is_err());
        let error = writer.failure.as_ref().unwrap();
        assert_eq!(error.resource_counter.as_deref(), Some("max_work"));
        assert_eq!(error.configured_bound, Some(1));
        assert!(writer.bytes.is_empty());
    }
    writer.flush().unwrap();
    assert_eq!(budget.remaining(), 0);
}
