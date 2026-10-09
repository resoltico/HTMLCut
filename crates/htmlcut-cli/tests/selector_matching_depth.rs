// SPDX-License-Identifier: MPL-2.0
mod support;
use support::invoke;

#[test]
fn excessive_matching_depth_exits_four_without_source_or_partial_output() {
    let selector = format!("{}span.private-query-marker", "* ".repeat(2_000));
    let source = b"<span class='private-query-marker'>private-source-marker</span>";
    let output = invoke(&["extract", "--stdin", "--select", &selector], source);
    assert_eq!(output.status.code(), Some(4));
    assert!(output.stdout.is_empty());
    let error: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(error["code"], "resource_limit");
    assert_eq!(error["stage"], "compilation");
    assert_eq!(error["resource_counter"], "selector_matching_depth");
    assert_eq!(error["configured_bound"], 64);
    let diagnostic = String::from_utf8(output.stderr).unwrap();
    for secret in ["private-query-marker", "private-source-marker", &selector] {
        assert!(!diagnostic.contains(secret));
    }
}
