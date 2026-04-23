use super::*;

#[test]
fn parse_byte_size_accepts_units() {
    assert_eq!(parse_byte_size("1kb").expect("byte size"), 1024);
    assert_eq!(parse_byte_size("1.5mb").expect("byte size"), 1_572_864);
    assert_eq!(parse_byte_size("0.5kb").expect("byte size"), 512);
    assert_eq!(parse_byte_size(".5kb").expect("byte size"), 512);
    assert_eq!(parse_byte_size("1gb").expect("byte size"), 1_073_741_824);
    assert!(parse_byte_size("banana").is_err());
    assert!(parse_byte_size("1tb").is_err());
    assert!(parse_byte_size("1..0kb").is_err());
    assert!(parse_byte_size(".kb").is_err());
    assert!(parse_byte_size("0.5b").is_err());
    assert!(parse_byte_size("0.1kb").is_err());
    assert!(parse_byte_size("0").is_err());
}
