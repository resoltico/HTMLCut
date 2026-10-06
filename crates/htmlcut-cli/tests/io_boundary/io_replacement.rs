// SPDX-License-Identifier: MPL-2.0
//! Actual regular-path replacement at the kernel-open boundary.
use super::*;

#[test]
fn regular_input_replaced_by_fifo_is_refused_for_source_plan_and_run() {
    let root = htmlcut_tempdir::tempdir().unwrap();
    let library = root.path().join("replaced-input.dylib");
    let compiled = Command::new("clang")
        .args(["-Wall", "-Wextra", "-Werror", "-dynamiclib"])
        .arg(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/replaced-input.c"),
        )
        .arg("-o")
        .arg(&library)
        .output()
        .unwrap();
    assert!(
        compiled.status.success(),
        "{}",
        String::from_utf8_lossy(&compiled.stderr)
    );
    for role in ["source", "plan", "replay"] {
        let target = root.path().join(format!("{role}.input"));
        let marker = root.path().join(format!("{role}.marker"));
        std::fs::write(&target, "<p>REGULAR BEFORE OPEN</p>").unwrap();
        let arguments = match role {
            "source" => vec![
                "extract",
                "--file",
                target.to_str().unwrap(),
                "--select",
                "p",
            ],
            "plan" => vec!["extract", "--stdin", "--plan", target.to_str().unwrap()],
            "replay" => vec!["replay", target.to_str().unwrap()],
            _ => unreachable!(),
        };
        let mut child = command()
            .args(arguments)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .env("DYLD_INSERT_LIBRARIES", &library)
            .env("HTMLCUT_RACE_PATH", &target)
            .env("HTMLCUT_RACE_MARKER", &marker)
            .spawn()
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(3);
        while child.try_wait().unwrap().is_none() {
            if Instant::now() >= deadline {
                child.kill().unwrap();
                let _ = child.wait();
                panic!("replacement FIFO open blocked for {role}");
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        let output = child.wait_with_output().unwrap();
        assert_eq!(
            std::fs::read_to_string(&marker).unwrap(),
            "replaced;nonblocking"
        );
        assert_eq!(
            output.status.code(),
            Some(5),
            "{role}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.stdout.is_empty());
        let error: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
        assert_eq!(error["cause"]["problem"], "unsupported_kind");
    }
}
