// SPDX-License-Identifier: MPL-2.0
//! Literal corpus through only the shared application facade.
use htmlcut_runtime::*;
use serde_json::Value as Json;
use std::path::Path;
fn completed<T>(result: Completion<T>) -> Result<T, Error> {
    match result {
        Completion::Finished(result) => result,
        Completion::Supervised(_) => panic!("unsupervised consumer entered executable enforcement"),
    }
}
#[test]
fn every_literal_case_uses_the_runtime_preflight_compute_and_delivery_boundary() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../conformance");
    let output = tempfile::tempdir().unwrap();
    let mut checked = 0;
    for kind in ["contract", "outline"] {
        let mut cases = std::fs::read_dir(root.join(kind))
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|p| p.is_dir())
            .collect::<Vec<_>>();
        cases.sort();
        assert!(!cases.is_empty());
        for path in cases {
            let options: Json = std::fs::read(path.join("options.json"))
                .ok()
                .map(|bytes| serde_json::from_slice(&bytes).unwrap())
                .unwrap_or_default();
            let mut limits = Limits::default();
            if let Some(nodes) = options["max_nodes"].as_u64() {
                limits.max_nodes = nodes as usize;
            }
            let operation = if kind == "contract" {
                Operation::Extract {
                    contract: ContractInput::Json(
                        std::fs::read_to_string(path.join("contract.json")).unwrap(),
                    ),
                    format: if options["raw"] == true {
                        ExportFormat::Raw
                    } else {
                        ExportFormat::Json
                    },
                }
            } else {
                let config: Json =
                    serde_json::from_slice(&std::fs::read(path.join("outline.json")).unwrap())
                        .unwrap();
                let mut outline = OutlineOptions::default();
                outline.select = config["select"].as_str().map(str::to_owned);
                if let Some(depth) = config["depth"].as_u64() {
                    outline.depth = depth as usize;
                }
                if let Some(lines) = config["max_lines"].as_u64() {
                    outline.max_lines = lines as usize;
                }
                outline.attrs = config["attrs"] == true;
                Operation::Outline(outline)
            };
            let result = completed(Job::new(None).unwrap().execute(Request {
                operation,
                input: Input::Bytes(std::fs::read(path.join("page.html")).unwrap()),
                encoding: options["encoding"].as_str().map(str::to_owned),
                base_url: options["base_url"].as_str().map(str::to_owned),
                limits: limits.clone(),
                destination: Destination::Return,
            }));
            if path.join("error.json").exists() {
                let error = result.unwrap_err();
                let mut actual = serde_json::to_value(error).unwrap();
                let mut expected: Json =
                    serde_json::from_slice(&std::fs::read(path.join("error.json")).unwrap())
                        .unwrap();
                actual.as_object_mut().unwrap().remove("message");
                expected.as_object_mut().unwrap().remove("message");
                if options["partial_error_facts"] == true {
                    for (key, value) in expected.as_object().unwrap() {
                        assert_eq!(actual.get(key), Some(value), "{path:?}: {key}");
                    }
                } else {
                    assert_eq!(actual, expected, "{path:?}");
                }
            } else {
                let Outcome::Returned(result) = result.unwrap() else {
                    panic!("{path:?}")
                };
                if kind == "contract" {
                    let expected: Json =
                        serde_json::from_slice(&std::fs::read(path.join("expected.json")).unwrap())
                            .unwrap();
                    assert_eq!(
                        serde_json::to_value(result.value().unwrap()).unwrap(),
                        expected,
                        "{path:?}"
                    );
                }
                let destination = output.path().join("result");
                completed(Job::new(None).unwrap().save(
                    &result,
                    Destination::File(destination.clone()),
                    limits,
                ))
                .unwrap();
                let bytes = std::fs::read(destination).unwrap();
                if kind == "outline" {
                    assert_eq!(
                        bytes,
                        std::fs::read(path.join("expected.txt")).unwrap(),
                        "{path:?}"
                    );
                } else if path.join("expected.raw").exists() {
                    assert_eq!(
                        bytes,
                        std::fs::read(path.join("expected.raw")).unwrap(),
                        "{path:?}"
                    );
                }
            }
            checked += 1;
        }
    }
    assert!(checked > 0);
    println!("HTMLCUT_RUNTIME_CONFORMANCE_CASES={checked}");
}
