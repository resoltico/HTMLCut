// SPDX-License-Identifier: MPL-2.0
mod support;
use std::process::Stdio;
fn assert_error(output: std::process::Output, status: i32, code: &str) {
    assert_eq!(output.status.code(), Some(status), "{output:?}");
    assert!(output.stdout.is_empty());
    assert_eq!(output.stderr.iter().filter(|b| **b == b'\n').count(), 1);
    let e: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(e["error"], code);
}
#[test]
fn contract_paths_inline_json_and_literal_stdin_have_distinct_operands() {
    use std::io::Write;
    let root = tempfile::tempdir().unwrap();
    let page = root.path().join("page");
    let contract = root.path().join("contract");
    std::fs::write(&page, b"<p>x</p>").unwrap();
    std::fs::write(&contract, b"\"p\"").unwrap();
    for inline in [false, true] {
        let mut c = support::command();
        c.arg("run");
        if inline {
            c.args(["--json", "\"p\""]);
        } else {
            c.arg(&contract);
        }
        let o = c.arg(&page).output().unwrap();
        assert!(o.status.success(), "{o:?}");
        assert_eq!(o.stdout, b"\"x\"\n");
    }
    assert_error(
        support::command()
            .args(["run", "--json", "\"p\""])
            .arg(&contract)
            .arg(&page)
            .output()
            .unwrap(),
        2,
        "invalid_request",
    );
    for file in [None, Some("-")] {
        let mut c = support::command();
        c.args(["run", "-"]);
        if let Some(f) = file {
            c.arg(f);
        }
        assert_error(c.output().unwrap(), 2, "invalid_request");
    }
    let mut child = support::command()
        .args(["run", "-"])
        .arg(&page)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(b"\"p\"").unwrap();
    let o = child.wait_with_output().unwrap();
    assert!(o.status.success(), "{o:?}");
    assert_eq!(o.stdout, b"\"x\"\n");
    let mut child = support::command()
        .args(["run", "--json", "\"p\"", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(b"<p>x</p>").unwrap();
    assert_eq!(child.wait_with_output().unwrap().stdout, b"\"x\"\n");
}
#[test]
fn input_contract_bounds_and_utf8_are_enforced_before_output() {
    let root = tempfile::tempdir().unwrap();
    let page = root.path().join("page");
    let contract = root.path().join("contract");
    std::fs::write(&page, b"<p>x</p>").unwrap();
    let o = support::command()
        .args(["get", "p"])
        .arg(&page)
        .args(["--max-input-bytes", "8"])
        .output()
        .unwrap();
    assert!(o.status.success());
    assert_eq!(o.stdout, b"\"x\"\n");
    assert_error(
        support::command()
            .args(["get", "p"])
            .arg(&page)
            .args(["--max-input-bytes", "7"])
            .output()
            .unwrap(),
        4,
        "limit_exceeded",
    );
    for bytes in [vec![b' '; htmlcut_core::Contract::MAX_BYTES + 1], vec![255]] {
        std::fs::write(&contract, bytes).unwrap();
        assert_error(
            support::command()
                .arg("run")
                .arg(&contract)
                .arg(&page)
                .output()
                .unwrap(),
            2,
            "invalid_contract",
        );
    }
    let mut exact = b"\"p\"".to_vec();
    exact.resize(htmlcut_core::Contract::MAX_BYTES, b' ');
    std::fs::write(&contract, exact).unwrap();
    assert!(
        support::command()
            .arg("run")
            .arg(&contract)
            .arg(&page)
            .output()
            .unwrap()
            .status
            .success()
    );
}
#[test]
fn preflight_and_zero_deadlines_refuse_before_opening_html() {
    assert_error(
        support::command()
            .args([
                "run",
                "--json",
                "{\"fields\":{\"x\":{}}}",
                "missing-html",
                "--raw",
            ])
            .output()
            .unwrap(),
        3,
        "unrepresentable_output",
    );
    assert_error(
        support::command()
            .args(["run", "--json", "[", "missing-html"])
            .output()
            .unwrap(),
        2,
        "invalid_contract",
    );
    assert_error(
        support::command()
            .args(["get", "p", "missing-html", "--timeout", "0"])
            .output()
            .unwrap(),
        4,
        "limit_exceeded",
    );
    let o = support::command()
        .args(["get", "p", "https://example.invalid/snapshot"])
        .output()
        .unwrap();
    assert!(String::from_utf8_lossy(&o.stderr).contains("fetch it first"));
    assert_error(o, 5, "input_unreadable");
}
#[cfg(unix)]
#[test]
fn implicit_terminal_stdin_is_refused() {
    let mut c = std::process::Command::new("script");
    #[cfg(target_os = "macos")]
    {
        c.args(["-q", "/dev/null"])
            .arg(support::binary())
            .args(["get", "p"]);
    }
    #[cfg(not(target_os = "macos"))]
    {
        let binary = support::binary().to_string_lossy().replace('\'', "'\\''");
        c.args(["-q", "-c", &format!("'{binary}' get p"), "/dev/null"]);
    }
    let o = c.stdin(Stdio::null()).output().unwrap();
    assert!(
        String::from_utf8_lossy(&o.stdout).contains("invalid_request"),
        "{o:?}"
    );
}
