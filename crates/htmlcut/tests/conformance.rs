// SPDX-License-Identifier: MPL-2.0
use htmlcut_conformance as fixture;
mod support;
use std::path::Path;
#[test]
fn every_case_runs_through_the_delivered_binary() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../conformance");
    let cases = fixture::discover(&root);
    let mut executed = 0;
    for case in &cases {
        let mut command = support::command();
        if case.contract.is_some() {
            command.arg("run").arg(case.path.join("contract.json"));
        } else {
            let o = case.outline.as_ref().unwrap();
            command.arg("outline");
            if let Some(s) = &o.select {
                command.arg("--select").arg(s);
            }
            if let Some(n) = o.depth {
                command.arg("--depth").arg(n.to_string());
            }
            if let Some(n) = o.max_lines {
                command.arg("--max-lines").arg(n.to_string());
            }
            if o.attrs {
                command.arg("--attrs");
            }
        }
        command.arg(case.path.join("page.html"));
        let o = &case.options;
        for (name, value) in [("encoding", &o.encoding), ("base-url", &o.base_url)] {
            if let Some(value) = value {
                command.arg(format!("--{name}")).arg(value);
            }
        }
        for (name, value) in [
            ("max-input-bytes", o.max_input_bytes),
            ("max-nodes", o.max_nodes),
            ("max-reading-bytes", o.max_reading_bytes),
            ("max-output-bytes", o.max_output_bytes),
        ] {
            if let Some(value) = value {
                command.arg(format!("--{name}")).arg(value.to_string());
            }
        }
        if o.raw {
            command.arg("--raw");
        }
        let output = command.output().unwrap();
        if let Some(expected) = &case.error {
            let status = match expected["error"].as_str().unwrap() {
                "limit_exceeded" => 4,
                "input_unreadable" | "undecodable_input" | "output_unwritable" => 5,
                "internal" => 6,
                "cancelled" => 130,
                s if s.starts_with("invalid_") => 2,
                _ => 3,
            };
            assert_eq!(
                output.status.code(),
                Some(status),
                "{}: {output:?}",
                case.identity()
            );
            assert!(output.stdout.is_empty());
            assert_eq!(output.stderr.iter().filter(|b| **b == b'\n').count(), 1);
            case.assert_error(serde_json::from_slice(&output.stderr).unwrap());
        } else {
            assert!(output.status.success(), "{}: {output:?}", case.identity());
            assert!(output.stderr.is_empty());
            case.assert_bytes(&output.stdout);
            if !o.raw
                && let Some(expected) = &case.expected
            {
                assert_eq!(
                    &serde_json::from_slice::<serde_json::Value>(&output.stdout).unwrap(),
                    expected
                );
            }
        }
        println!("HTMLCUT_CONFORMANCE_ID={}", case.identity());
        executed += 1;
    }
    assert_eq!(executed, cases.len());
    println!("HTMLCUT_CONFORMANCE_CASES={executed}");
}
#[test]
fn help_version_and_bad_arguments_use_the_declared_streams() {
    for args in [vec!["--help"], vec!["run", "--help"], vec!["-V"]] {
        let o = support::command().args(args).output().unwrap();
        assert!(o.status.success());
        assert!(!o.stdout.is_empty());
        assert!(o.stderr.is_empty());
    }
    for args in [vec![], vec!["--unknown"], vec!["schema", "--timeout", "1"]] {
        let o = support::command().args(args).output().unwrap();
        assert_eq!(o.status.code(), Some(2));
        assert!(o.stdout.is_empty());
        assert_eq!(o.stderr.iter().filter(|b| **b == b'\n').count(), 1);
        let e: serde_json::Value = serde_json::from_slice(&o.stderr).unwrap();
        assert_eq!(e["error"], "invalid_request");
    }
}
