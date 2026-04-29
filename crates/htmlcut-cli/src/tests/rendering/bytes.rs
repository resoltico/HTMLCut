use super::*;

#[test]
fn parse_byte_size_accepts_units() {
    assert_eq!(parse_byte_size("1kib").expect("byte size"), 1024);
    assert_eq!(parse_byte_size("1.5mib").expect("byte size"), 1_572_864);
    assert_eq!(parse_byte_size("0.5kib").expect("byte size"), 512);
    assert_eq!(parse_byte_size(".5kib").expect("byte size"), 512);
    assert_eq!(parse_byte_size("1gib").expect("byte size"), 1_073_741_824);
    assert!(parse_byte_size("banana").is_err());
    assert!(parse_byte_size("1tb").is_err());
    assert!(parse_byte_size("1..0kib").is_err());
    assert!(parse_byte_size("1.2.3kib").is_err());
    assert!(parse_byte_size(".kib").is_err());
    assert!(parse_byte_size("0.5b").is_err());
    assert!(parse_byte_size("0.1kib").is_err());
    assert!(parse_byte_size("0").is_err());
}
