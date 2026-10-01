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
