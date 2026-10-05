// SPDX-License-Identifier: MPL-2.0
mod support;
use support::invoke;

#[test]
fn index_one_description_and_named_schemas_share_closed_vocabulary() {
    let output = invoke(&["describe"], b"");
    assert!(output.status.success());
    let index: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(index["operations"].as_array().unwrap().len(), 6);
    let output = invoke(&["describe", "extract"], b"");
    assert!(output.status.success());
    let description: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(description["defaults"]["selection"], "single");
    assert_eq!(description["defaults"]["read"], "dom_text");
    for name in htmlcut_core::SCHEMA_NAMES
        .iter()
        .copied()
        .chain(["htmlcut.bundle"])
    {
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
