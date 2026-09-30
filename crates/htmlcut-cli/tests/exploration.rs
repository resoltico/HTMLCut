mod support;
use support::invoke;

#[test]
fn snapshot_pages_previews_and_explicit_proposals_reject_stale_evidence() {
    let root = htmlcut_tempdir::tempdir().unwrap();
    let source = {
        let path = root.path().join("page.html");
        std::fs::write(&path, "<p>A</p><p id='amount'>180</p>").unwrap();
        path
    };
    let first = invoke(
        &[
            "inspect",
            "--file",
            source.to_str().unwrap(),
            "--page-size",
            "2",
        ],
        b"",
    );
    assert!(first.status.success());
    let page: serde_json::Value = serde_json::from_slice(&first.stdout).unwrap();
    let cursor = page["next_cursor"].as_str().unwrap();
    let next = invoke(
        &[
            "inspect",
            "--file",
            source.to_str().unwrap(),
            "--page-size",
            "2",
            "--cursor",
            cursor,
        ],
        b"",
    );
    assert!(next.status.success());
    std::fs::write(&source, "<p>changed</p>").unwrap();
    let stale = invoke(
        &[
            "inspect",
            "--file",
            source.to_str().unwrap(),
            "--page-size",
            "2",
            "--cursor",
            cursor,
        ],
        b"",
    );
    assert_eq!(stale.status.code(), Some(2));
    assert!(stale.stdout.is_empty());
    let live = invoke(
        &[
            "inspect",
            "--url",
            "http://127.0.0.1:1/",
            "--cursor",
            cursor,
        ],
        b"",
    );
    assert_eq!(live.status.code(), Some(2));
    assert!(live.stdout.is_empty());
    let page = invoke(&["inspect", "--file", source.to_str().unwrap()], b"");
    let page: serde_json::Value = serde_json::from_slice(&page.stdout).unwrap();
    let element = page["elements"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["tag"] == "p")
        .unwrap();
    let proposal = invoke(
        &[
            "inspect",
            "--file",
            source.to_str().unwrap(),
            "--propose",
            element["handle"].as_str().unwrap(),
        ],
        b"",
    );
    assert!(proposal.status.success());
    let proposal: serde_json::Value = serde_json::from_slice(&proposal.stdout).unwrap();
    assert_eq!(proposal["kind"], "suggestion");
    let chosen = invoke(
        &[
            "extract",
            "--file",
            source.to_str().unwrap(),
            "--css",
            proposal["selector"].as_str().unwrap(),
            "--raw",
        ],
        b"",
    );
    assert!(chosen.status.success());
    assert_eq!(chosen.stdout, b"changed");
}
