// SPDX-License-Identifier: MPL-2.0
mod support;
use support::invoke;

#[test]
fn source_unicode_crlf_quotes_and_inclusions_are_exact() {
    let source = "éSTART\r\n<X a='1'>✓</X>\r\nEND";
    let output = invoke(
        &[
            "extract", "--stdin", "--start", "START", "--end", "END", "--raw",
        ],
        source.as_bytes(),
    );
    assert!(output.status.success());
    assert_eq!(output.stdout, "\r\n<X a='1'>✓</X>\r\n".as_bytes());
    let output = invoke(
        &[
            "extract",
            "--stdin",
            "--start",
            "START",
            "--end",
            "END",
            "--include-start",
            "--include-end",
            "--raw",
        ],
        source.as_bytes(),
    );
    assert!(output.status.success());
    assert_eq!(output.stdout, "START\r\n<X a='1'>✓</X>\r\nEND".as_bytes());
}

#[test]
fn unmatched_tail_and_zero_length_regex_are_whole_operation_errors() {
    for source in ["STARToneENDSTARTtail", "STARTtail"] {
        let output = invoke(
            &["extract", "--stdin", "--start", "START", "--end", "END"],
            source.as_bytes(),
        );
        assert_eq!(output.status.code(), Some(3));
        assert!(output.stdout.is_empty());
    }
    let output = invoke(
        &[
            "extract", "--stdin", "--start", "^", "--end", "END", "--regex",
        ],
        b"STARTvalueEND",
    );
    assert_eq!(output.status.code(), Some(3));
    assert!(output.stdout.is_empty());
}
