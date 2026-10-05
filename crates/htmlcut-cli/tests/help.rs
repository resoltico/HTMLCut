// SPDX-License-Identifier: MPL-2.0
mod support;
use support::invoke;

#[test]
fn current_help_version_and_retired_vocabulary() {
    for args in [
        vec!["--help"],
        vec!["extract", "--help"],
        vec!["inspect", "--help"],
        vec!["replay", "--help"],
    ] {
        let output = invoke(&args, b"");
        assert!(output.status.success());
        assert!(!output.stdout.is_empty());
        assert!(output.stderr.is_empty());
    }
    let output = invoke(&["--version"], b"");
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        format!("htmlcut {}\n", env!("CARGO_PKG_VERSION"))
    );
    for args in [
        vec!["select"],
        vec!["slice"],
        vec!["catalog"],
        vec!["extract", "--stdin", "--select", "p", "--match", "first"],
    ] {
        let output = invoke(&args, b"<p>A</p>");
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
    }
}
