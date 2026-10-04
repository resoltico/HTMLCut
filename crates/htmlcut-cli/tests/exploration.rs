// SPDX-License-Identifier: MPL-2.0
mod support;
use support::invoke;
#[test]
fn targeted_inspection_requires_a_selector_and_has_no_retired_modes() {
    for args in [
        vec!["inspect", "--stdin"],
        vec!["inspect", "--stdin", "--css", "p", "--samples", "0"],
        vec!["inspect", "--stdin", "--css", "p", "--samples", "11"],
        vec!["inspect", "--stdin", "--css", "p", "--cursor", "obsolete"],
        vec!["inspect", "--stdin", "--css", "p", "--propose", "obsolete"],
        vec![
            "inspect",
            "--stdin",
            "--css",
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
