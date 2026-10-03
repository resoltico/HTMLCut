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
fn borrowed_json_digest_enforces_encoded_bytes_and_shared_work() {
    use selectors::work_budget::SelectorWorkBudget;
    let value = serde_json::json!(["é\n"]);
    let encoded = serde_json::to_vec(&value).unwrap();
    for maximum in [encoded.len() - 1, encoded.len(), encoded.len() + 1] {
        let budget = SelectorWorkBudget::new(10);
        assert_eq!(
            data_digest(&value, maximum, &budget).is_ok(),
            maximum >= encoded.len()
        );
    }
    let budget = SelectorWorkBudget::new(1);
    assert!(budget.consume());
    assert_eq!(
        data_digest(&value, 100, &budget).unwrap_err().code,
        ErrorCode::ResourceLimit
    );
    let budget = SelectorWorkBudget::new(1);
    let mut writer = JsonDigestWriter {
        hash: Sha256::new(),
        bytes: 0,
        maximum: 4,
        budget: &budget,
    };
    writer.write_all(b"abcd").unwrap();
    writer.flush().unwrap();
    assert_eq!(writer.bytes, 4);
    assert_eq!(budget.remaining(), 0);
    assert!(writer.write_all(b"e").is_err());
    writer.write_all(b"").unwrap();
    assert_eq!(hex(&writer.hash.finalize()), sha256(b"abcd"));
}
