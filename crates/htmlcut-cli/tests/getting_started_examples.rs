// SPDX-License-Identifier: MPL-2.0
mod support;
use support::invoke;

#[test]
fn basic_extraction_uses_standard_native_help_without_schema() {
    for args in [vec!["--help"], vec!["extract", "--help"]] {
        let output = invoke(&args, b"");
        assert!(output.status.success());
        assert!(String::from_utf8_lossy(&output.stdout).contains("Usage:"));
    }
    let output = invoke(
        &["extract", "--stdin", "--select", "#amount", "--raw"],
        b"<p id='amount'>EUR 180</p>",
    );
    assert!(output.status.success());
    assert_eq!(output.stdout, b"EUR 180");
}
