// SPDX-License-Identifier: MPL-2.0
//! Literal corpus through only the shared application facade.
use htmlcut_runtime::*;
use std::path::Path;
fn completed<T>(result: Completion<T>) -> Result<T, Error> {
    match result {
        Completion::Finished(result) => result,
        Completion::Supervised(_) => panic!("unsupervised consumer entered executable enforcement"),
    }
}
#[path = "../../../conformance/fixture.rs"]
mod fixture;
#[test]
fn every_literal_case_uses_the_runtime_preflight_compute_and_delivery_boundary() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../conformance");
    let cases = fixture::discover(&root);
    for case in &cases {
        run_case(case);
        println!("HTMLCUT_CONFORMANCE_ID={}", case.identity());
    }
    println!("HTMLCUT_RUNTIME_CONFORMANCE_CASES={}", cases.len());
}
fn run_case(case: &fixture::Case) {
    let options = &case.options;
    let mut limits = Limits::default();
    if let Some(n) = options.max_input_bytes {
        limits.max_input_bytes = n;
    }
    if let Some(n) = options.max_nodes {
        limits.max_nodes = n;
    }
    if let Some(n) = options.max_reading_bytes {
        limits.max_reading_bytes = n;
    }
    if let Some(n) = options.max_output_bytes {
        limits.max_output_bytes = n;
    }
    let operation = if let Some(text) = &case.contract {
        Operation::Extract {
            contract: ContractInput::Json(text.clone()),
            format: if options.raw {
                ExportFormat::Raw
            } else {
                ExportFormat::Json
            },
        }
    } else {
        let config = case.outline.as_ref().unwrap();
        let mut outline = OutlineOptions::default();
        outline.select = config.select.clone();
        if let Some(n) = config.depth {
            outline.depth = n;
        }
        if let Some(n) = config.max_lines {
            outline.max_lines = n;
        }
        outline.attrs = config.attrs;
        Operation::Outline(outline)
    };
    let result = completed(Job::new(None).unwrap().execute(Request {
        operation,
        input: Input::Bytes(case.page.clone()),
        encoding: options.encoding.clone(),
        base_url: options.base_url.clone(),
        limits: limits.clone(),
        destination: Destination::Return,
    }));
    if case.error.is_some() {
        case.assert_error(serde_json::to_value(result.unwrap_err()).unwrap());
    } else {
        let Outcome::Returned(result) = result.unwrap_or_else(|e| panic!("{:?}: {e}", case.path))
        else {
            panic!("{:?}", case.path)
        };
        if let Some(expected) = &case.expected {
            assert_eq!(
                serde_json::to_value(result.value().unwrap()).unwrap(),
                *expected,
                "{:?}",
                case.path
            );
        }
        case.assert_bytes(
            &completed(Job::new(None).unwrap().export(&result, limits.clone())).unwrap(),
        );
        let output = tempfile::tempdir().unwrap();
        let destination = output.path().join("result");
        completed(Job::new(None).unwrap().save(
            &result,
            Destination::File(destination.clone()),
            limits,
        ))
        .unwrap();
        case.assert_bytes(&std::fs::read(destination).unwrap());
    }
}
#[test]
fn runtime_adapter_rejects_wrong_json_oracle() {
    let root =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../conformance/contract/01-value-one");
    let dir = tempfile::tempdir().unwrap();
    for entry in std::fs::read_dir(root).unwrap() {
        let path = entry.unwrap().path();
        std::fs::copy(&path, dir.path().join(path.file_name().unwrap())).unwrap();
    }
    let mut case = fixture::load(dir.path(), "contract");
    // Keep the semantic oracle correct: the export-byte assertion must detect this.
    case.bytes = b"\"wrong\"\n".to_vec();
    assert!(std::panic::catch_unwind(|| run_case(&case)).is_err());
}

#[test]
fn non_stdin_carriers_preserve_independent_unicode_bytes() {
    let directory = tempfile::tempdir().unwrap();
    let page = directory.path().join("page.html");
    let contract = directory.path().join("contract.json");
    std::fs::write(&page, "<p>Rīga 雪</p>").unwrap();
    std::fs::write(&contract, b"\"p\"").unwrap();
    let mut checked = 0;
    for input in [
        Input::File(page),
        Input::Bytes("<p>Rīga 雪</p>".as_bytes().to_vec()),
    ] {
        for contract in [
            ContractInput::Json("\"p\"".into()),
            ContractInput::File(contract.clone()),
            ContractInput::Quick(QueryRequest {
                select: "p".into(),
                read: "text".into(),
                cardinality: Match::One,
            }),
        ] {
            let request = Request {
                operation: Operation::Extract {
                    contract,
                    format: ExportFormat::Json,
                },
                input: input.clone(),
                encoding: None,
                base_url: None,
                limits: Limits::default(),
                destination: Destination::Return,
            };
            let Outcome::Returned(result) =
                completed(Job::new(None).unwrap().execute(request)).unwrap()
            else {
                panic!()
            };
            assert_eq!(
                completed(Job::new(None).unwrap().export(&result, Limits::default())).unwrap(),
                "\"Rīga 雪\"\n".as_bytes()
            );
            checked += 1;
        }
    }
    assert_eq!(checked, 6);
}

#[test]
fn prepared_metadata_survives_source_removal_failed_save_and_fresh_retry() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../conformance/contract/authored-offers-encoding");
    let case = fixture::load(&root, "contract");
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("offers.html");
    std::fs::write(&source, &case.page).unwrap();
    let document = completed(Job::new(None).unwrap().prepare(
        Input::File(source.clone()),
        case.options.encoding.clone(),
        case.options.base_url.clone(),
        Limits::default(),
    ))
    .unwrap();
    std::fs::remove_file(source).unwrap();
    let mut outline = OutlineOptions::default();
    outline.select = Some("h1".into());
    let Outcome::Returned(report) = completed(Job::new(None).unwrap().execute_prepared(
        &document,
        Operation::Outline(outline),
        Limits::default(),
        Destination::Return,
    ))
    .unwrap() else {
        panic!()
    };
    assert_eq!(
        completed(Job::new(None).unwrap().export(&report, Limits::default())).unwrap(),
        "1 match for h1\nh1 \"Offres café\"\n".as_bytes()
    );
    let Outcome::Returned(result) = completed(Job::new(None).unwrap().execute_prepared(
        &document,
        Operation::Extract {
            contract: ContractInput::Json(case.contract.clone().unwrap()),
            format: ExportFormat::Json,
        },
        Limits::default(),
        Destination::Return,
    ))
    .unwrap() else {
        panic!()
    };
    let destination = directory.path().join("result");
    std::fs::write(&destination, b"old destination").unwrap();
    let inventory = || {
        let mut entries: Vec<_> = std::fs::read_dir(directory.path())
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        entries.sort();
        entries
    };
    let before = inventory();
    let mut limits = Limits::default();
    limits.max_output_bytes = case.bytes.len() - 1;
    let error = completed(Job::new(None).unwrap().save(
        &result,
        Destination::File(destination.clone()),
        limits,
    ))
    .unwrap_err();
    assert_eq!(
        serde_json::to_value(&error).unwrap()["limit"].as_str(),
        Some("output_bytes")
    );
    assert_eq!(std::fs::read(&destination).unwrap(), b"old destination");
    assert_eq!(inventory(), before);
    completed(Job::new(None).unwrap().save(
        &result,
        Destination::File(destination.clone()),
        Limits::default(),
    ))
    .unwrap();
    case.assert_bytes(&std::fs::read(destination).unwrap());
}
