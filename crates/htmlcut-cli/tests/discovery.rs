// SPDX-License-Identifier: MPL-2.0
mod support;
use support::invoke;

#[test]
fn native_help_and_named_schemas_share_current_closed_vocabulary() {
    let output = invoke(&["--help"], b"");
    assert!(output.status.success());
    let help = String::from_utf8(output.stdout).unwrap();
    for name in ["extract", "inspect", "schema"] {
        assert!(help.contains(name));
    }
    for name in ["describe", "outline"] {
        assert!(!help.contains(name));
    }
    let output = invoke(&["extract", "--help"], b"");
    assert!(output.status.success());
    let help = String::from_utf8(output.stdout).unwrap();
    for required in [
        "--stdin",
        "--file",
        "--nth",
        "--optional-field",
        "--many-field",
        "outer-html",
        "literal",
        "\"min\":0",
    ] {
        assert!(help.contains(required), "{required}");
    }
    for name in htmlcut_core::SCHEMA_NAMES.iter().copied() {
        let output = invoke(&["schema", name], b"");
        assert!(
            output.status.success(),
            "{name}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            serde_json::from_slice::<serde_json::Value>(&output.stdout)
                .unwrap()
                .is_object()
        );
    }
    for args in [
        vec!["describe", "unknown"],
        vec!["schema", "htmlcut.unknown"],
    ] {
        let output = invoke(&args, b"");
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
    }
}
