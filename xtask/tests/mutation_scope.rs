#![forbid(unsafe_code)]
#![cfg(unix)]

use std::fs;
use std::process::Command;

use htmlcut_tempdir::tempdir;
use serde_json::json;

fn repo_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

#[test]
fn mutation_diff_scope_accepts_only_identities_from_the_verified_full_inventory() {
    let fixture = tempdir().expect("mutation scope fixture");
    let full = fixture.path().join("full.json");
    let subset = fixture.path().join("subset.json");
    let script = repo_root().join("scripts/verify-mutation-scope.sh");
    let identity = json!({
        "name": "xtask/src/example.rs:1: replace example",
        "package": "xtask",
        "file": "xtask/src/example.rs"
    });
    fs::write(&full, json!([identity]).to_string()).expect("full inventory");

    let verify = || {
        Command::new("bash")
            .arg(&script)
            .args(["--subset"])
            .arg(&subset)
            .arg(&full)
            .output()
            .expect("run mutation subset verifier")
    };

    fs::write(&subset, json!([identity]).to_string()).expect("matching subset");
    assert!(verify().status.success());
    fs::write(&subset, "[]").expect("empty subset");
    assert!(verify().status.success());
    fs::write(
        &subset,
        json!([{"name": identity["name"], "package": "other", "file": identity["file"]}])
            .to_string(),
    )
    .expect("foreign subset");
    let foreign = verify();
    assert!(!foreign.status.success());
    assert!(String::from_utf8_lossy(&foreign.stderr).contains("outside the verified full scope"));
    fs::write(&subset, "not JSON").expect("malformed subset");
    assert!(!verify().status.success());
}
