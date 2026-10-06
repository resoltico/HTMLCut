// SPDX-License-Identifier: MPL-2.0
mod support;
use support::invoke;
#[test]
fn inspection_mode_flags_are_applicable_and_retired_modes_are_rejected() {
    for args in [
        vec!["inspect", "--stdin", "--samples", "1"],
        vec!["inspect", "--stdin", "--select", "p", "--samples", "0"],
        vec!["inspect", "--stdin", "--select", "p", "--samples", "11"],
        vec![
            "inspect", "--stdin", "--select", "p", "--cursor", "obsolete",
        ],
        vec![
            "inspect",
            "--stdin",
            "--select",
            "p",
            "--propose",
            "obsolete",
        ],
        vec![
            "inspect",
            "--stdin",
            "--select",
            "p",
            "--preview-plan",
            "obsolete",
        ],
    ] {
        let output = invoke(&args, b"<p>A</p>");
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
    }
}
