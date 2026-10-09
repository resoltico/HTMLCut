// SPDX-License-Identifier: MPL-2.0
//! Native file links and atomic file-kind replacement stress.
use super::*;
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};

#[test]
fn directory_device_and_socket_paths_cannot_be_file_sources_or_plans() {
    let root = tempfile::tempdir().unwrap();
    let socket_path = root.path().join("input.socket");
    let _socket = std::os::unix::net::UnixListener::bind(&socket_path).unwrap();
    for path in [root.path(), std::path::Path::new("/dev/null"), &socket_path] {
        for arguments in [
            vec!["extract", "--file", path.to_str().unwrap(), "--select", "p"],
            vec!["extract", "--stdin", "--plan", path.to_str().unwrap()],
        ] {
            let mut child = command()
                .args(arguments)
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .unwrap();
            let deadline = Instant::now() + Duration::from_secs(3);
            while child.try_wait().unwrap().is_none() {
                if Instant::now() >= deadline {
                    child.kill().unwrap();
                    let _ = child.wait();
                    panic!("special file acquisition blocked");
                }
                std::thread::sleep(Duration::from_millis(5));
            }
            let output = child.wait_with_output().unwrap();
            assert_eq!(output.status.code(), Some(5));
            assert!(output.stdout.is_empty());
            let error: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
            assert_eq!(error["code"], "acquisition");
            assert_eq!(error["cause"]["kind"], "io");
            if path != socket_path {
                assert_eq!(error["cause"]["problem"], "unsupported_kind");
            }
        }
    }
}

#[test]
fn regular_symlink_and_hardlink_inputs_preserve_complete_values() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("source.html");
    std::fs::write(&source, "<p>LINK VALUE</p>").unwrap();
    let symbolic = root.path().join("symbolic.html");
    let hard = root.path().join("hard.html");
    std::os::unix::fs::symlink(&source, &symbolic).unwrap();
    std::fs::hard_link(&source, &hard).unwrap();
    for path in [&source, &symbolic, &hard] {
        let output = command()
            .args([
                "extract",
                "--file",
                path.to_str().unwrap(),
                "--select",
                "p",
                "--raw",
            ])
            .output()
            .unwrap();
        assert!(output.status.success());
        assert_eq!(output.stdout, b"LINK VALUE");
        assert!(output.stderr.is_empty());
    }
}

#[test]
fn concurrent_atomic_regular_fifo_replacement_never_blocks_or_returns_partial_values() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("stable-source.html");
    std::fs::write(&source, "<p>HELLO</p>").unwrap();
    let plan =
        htmlcut_core::canonical_json(&htmlcut_core::ExtractionPlan::css("p").unwrap()).unwrap();
    for role in ["source", "plan"] {
        let path = root.path().join(format!("{role}.input"));
        let content = match role {
            "source" => b"<p>HELLO</p>".to_vec(),
            "plan" => plan.as_bytes().to_vec(),
            _ => unreachable!(),
        };
        std::fs::write(&path, &content).unwrap();
        let stop = Arc::new(AtomicBool::new(false));
        let replacements = Arc::new(AtomicUsize::new(0));
        let worker_stop = stop.clone();
        let worker_count = replacements.clone();
        let target = path.clone();
        let worker = std::thread::spawn(move || {
            let staging = target.with_file_name("replacement");
            while !worker_stop.load(Ordering::Relaxed) {
                std::fs::write(&staging, &content).unwrap();
                std::fs::rename(&staging, &target).unwrap();
                assert!(
                    Command::new("mkfifo")
                        .arg(&staging)
                        .status()
                        .unwrap()
                        .success()
                );
                std::fs::rename(&staging, &target).unwrap();
                worker_count.fetch_add(1, Ordering::Relaxed);
            }
        });
        let result = std::panic::catch_unwind(|| {
            for _ in 0..24 {
                let arguments = match role {
                    "source" => vec![
                        "extract",
                        "--file",
                        path.to_str().unwrap(),
                        "--select",
                        "p",
                        "--raw",
                    ],
                    "plan" => vec![
                        "extract",
                        "--file",
                        source.to_str().unwrap(),
                        "--plan",
                        path.to_str().unwrap(),
                        "--raw",
                    ],
                    _ => unreachable!(),
                };
                let mut child = command()
                    .args(arguments)
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .spawn()
                    .unwrap();
                let deadline = Instant::now() + Duration::from_secs(3);
                while child.try_wait().unwrap().is_none() {
                    if Instant::now() >= deadline {
                        child.kill().unwrap();
                        let _ = child.wait();
                        panic!("atomic FIFO replacement caused a blocking open");
                    }
                    std::thread::sleep(Duration::from_millis(5));
                }
                let output = child.wait_with_output().unwrap();
                match output.status.code() {
                    Some(0) => {
                        assert_eq!(output.stdout, b"HELLO");
                        assert!(output.stderr.is_empty());
                    }
                    Some(5) => {
                        assert!(output.stdout.is_empty());
                        let error: serde_json::Value =
                            serde_json::from_slice(&output.stderr).unwrap();
                        assert_eq!(error["cause"]["problem"], "unsupported_kind");
                    }
                    status => panic!(
                        "unexpected replacement outcome {status:?}: {}",
                        String::from_utf8_lossy(&output.stderr)
                    ),
                }
            }
        });
        stop.store(true, Ordering::Relaxed);
        worker.join().unwrap();
        assert!(replacements.load(Ordering::Relaxed) > 0);
        if let Err(error) = result {
            std::panic::resume_unwind(error);
        }
    }
}
