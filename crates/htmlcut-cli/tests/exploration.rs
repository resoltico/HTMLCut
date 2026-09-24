mod support;
use support::*;

#[test]
fn elements_cursor_pages_a_static_file_and_rejects_stale_or_malformed_inputs() {
    let dir = tempdir().expect("tempdir");
    let source = dir.path().join("page.html");
    fs::write(&source, "<main>One</main><aside>Two</aside>").expect("write source");
    let path = source.to_string_lossy().into_owned();
    let page = |cursor: Option<&str>, max_elements: &str| {
        let mut command = Command::cargo_bin("htmlcut").expect("binary");
        command.args([
            "inspect",
            "elements",
            path.as_str(),
            "--max-elements",
            max_elements,
        ]);
        if let Some(cursor) = cursor {
            command.args(["--cursor", cursor]);
        }
        let output = command.output().expect("run elements");
        let json: serde_json::Value = serde_json::from_slice(&output.stdout).expect("JSON result");
        (output.status.code(), json)
    };

    let (first_status, first) = page(None, "2");
    assert_eq!(first_status, Some(0));
    let first_cursor = serde_json::to_string(&first["next_cursor"]).expect("cursor JSON");
    let padding = 1_024_usize
        .checked_sub(first_cursor.len())
        .expect("cursor fits the published JSON limit");
    let at_limit = format!("{}{}", " ".repeat(padding), first_cursor);
    assert_eq!(at_limit.len(), 1_024);
    let (at_limit_status, _) = page(Some(&at_limit), "2");
    assert_eq!(at_limit_status, Some(0));
    let over_limit = format!(" {at_limit}");
    let (over_limit_status, over_limit_error) = page(Some(&over_limit), "2");
    assert_eq!(over_limit_status, Some(2));
    assert_eq!(
        over_limit_error["error"]["code"],
        "CLI_EXPLORATION_CURSOR_INVALID"
    );
    assert!(
        over_limit_error["error"]["message"]
            .as_str()
            .expect("cursor error message")
            .contains("exceeds the 1024-byte JSON limit")
    );
    let (second_status, second) = page(Some(&first_cursor), "2");
    assert_eq!(second_status, Some(0));
    let second_cursor = serde_json::to_string(&second["next_cursor"]).expect("cursor JSON");
    let (third_status, third) = page(Some(&second_cursor), "2");
    assert_eq!(third_status, Some(0));
    assert!(third.get("next_cursor").is_none());
    let paths = [&first, &second, &third]
        .into_iter()
        .flat_map(|result| result["elements"].as_array().expect("elements"))
        .map(|element| element["path"].as_str().expect("path"))
        .collect::<Vec<_>>();
    assert_eq!(paths.len(), 5);
    assert_eq!(
        paths
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        5
    );

    let (options_status, options_error) = page(Some(&first_cursor), "3");
    assert_eq!(options_status, Some(2));
    assert_eq!(
        options_error["error"]["code"],
        "CLI_EXPLORATION_CURSOR_OPTIONS_MISMATCH"
    );
    for (field, value) in [
        ("discovery_identity_sha256", serde_json::json!("ABC")),
        ("next_document_ordinal", serde_json::json!(100)),
        ("unknown", serde_json::json!(true)),
    ] {
        let mut invalid = first["next_cursor"].clone();
        invalid[field] = value;
        let cursor = serde_json::to_string(&invalid).expect("invalid cursor JSON");
        let (status, error) = page(Some(&cursor), "2");
        assert_eq!(status, Some(2));
        assert_eq!(error["error"]["code"], "CLI_EXPLORATION_CURSOR_INVALID");
    }
    fs::write(&source, "<main>Changed</main><aside>Two</aside>").expect("change source");
    let (snapshot_status, snapshot_error) = page(Some(&first_cursor), "2");
    assert_eq!(snapshot_status, Some(4));
    assert_eq!(
        snapshot_error["error"]["code"],
        "CLI_EXPLORATION_CURSOR_SNAPSHOT_MISMATCH"
    );
    for cursor in ["{", &"x".repeat(1_025)] {
        let (status, error) = page(Some(cursor), "2");
        assert_eq!(status, Some(2));
        assert_eq!(error["error"]["code"], "CLI_EXPLORATION_CURSOR_INVALID");
    }
}

#[test]
fn propose_accepts_the_published_fingerprint_vector_and_rejects_plain_sha256() {
    let args = [
        "inspect",
        "propose",
        "--input-html",
        "<main>One</main>",
        "--path",
        "html:html[1]/html:body[1]/html:main[1]",
        "--namespace",
        "html",
        "--local-name",
        "main",
        "--text-digest-sha256",
    ];
    let mut valid = Command::cargo_bin("htmlcut").expect("binary");
    valid
        .args(args)
        .arg("e163d4661874af8285350875496da6d42afb63bbe2c9e6f7d390c6c632aadfc6")
        .assert()
        .success()
        .stdout(predicate::str::contains("htmlcut.target_resolution_result"));

    let mut plain = Command::cargo_bin("htmlcut").expect("binary");
    plain
        .args(args)
        .arg("8b12507783d5becacbf2ebe5b01a60024d8728a8f86dcc818bce699e8b3320bc")
        .assert()
        .failure()
        .code(4);
}

#[test]
fn elements_command_emits_bounded_exploration_json() {
    let mut command = Command::cargo_bin("htmlcut").expect("binary");
    let output = command
        .args([
            "inspect",
            "elements",
            "--input-html",
            "<main><p>One</p></main>",
            "--max-elements",
            "2",
            "--max-work-units",
            "100",
            "--max-proposals-per-element",
            "2",
            "--preview-bytes",
            "32",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let result: serde_json::Value = serde_json::from_slice(&output).expect("exploration JSON");
    assert_eq!(result["schema_name"], "htmlcut.exploration_result");
    assert_eq!(result["schema_version"], 1);
    assert!(
        result["elements"]
            .as_array()
            .is_some_and(|elements| !elements.is_empty())
    );
}

#[test]
fn elements_command_writes_a_quiet_output_file_after_successful_preparation() {
    let output_dir = tempdir().expect("tempdir");
    let output_path = output_dir.path().join("elements.json");
    let mut command = Command::cargo_bin("htmlcut").expect("binary");
    command
        .args([
            "--quiet",
            "inspect",
            "elements",
            "--input-html",
            "<main><p>One</p></main>",
            "--output-file",
            output_path.to_string_lossy().as_ref(),
        ])
        .assert()
        .success()
        .stderr(predicate::str::is_empty());
    assert!(
        fs::read_to_string(&output_path)
            .expect("elements output")
            .contains("htmlcut.exploration_result")
    );
}

#[test]
fn propose_command_refuses_a_path_only_target() {
    let mut command = Command::cargo_bin("htmlcut").expect("binary");
    command
        .args([
            "inspect",
            "propose",
            "--input-html",
            "<main>One</main>",
            "--path",
            "html:html[1]/html:body[1]/html:main[1]",
            "--namespace",
            "html",
            "--local-name",
            "main",
            "--text-digest-sha256",
            "not-a-digest",
        ])
        .assert()
        .failure()
        .code(4)
        .stdout(predicate::str::contains(
            "current path and text-fingerprint contract",
        ));
}

#[test]
fn exploration_commands_reject_invalid_limits_source_and_target_evidence() {
    for (flag, value) in [
        ("--max-elements", "0"),
        ("--max-work-units", "0"),
        ("--max-proposals-per-element", "0"),
        ("--preview-bytes", "0"),
        ("--max-elements", "1001"),
    ] {
        let mut command = Command::cargo_bin("htmlcut").expect("binary");
        command
            .args([
                "inspect",
                "elements",
                "--input-html",
                "<main><p>One</p></main>",
                flag,
                value,
            ])
            .assert()
            .failure();
    }

    let missing = tempdir().expect("tempdir").path().join("missing.html");
    let mut command = Command::cargo_bin("htmlcut").expect("binary");
    command
        .args(["inspect", "elements", missing.to_string_lossy().as_ref()])
        .assert()
        .failure()
        .code(3);

    let mut command = Command::cargo_bin("htmlcut").expect("binary");
    command
        .args([
            "inspect",
            "propose",
            missing.to_string_lossy().as_ref(),
            "--path",
            "html:html[1]/html:body[1]/html:main[1]",
            "--namespace",
            "html",
            "--local-name",
            "main",
            "--text-digest-sha256",
            "0000000000000000000000000000000000000000000000000000000000000000",
        ])
        .assert()
        .failure()
        .code(3);

    for (flag, value) in [
        ("--namespace", "unknown"),
        ("--attribute", "missing-separator"),
        ("--attribute", "=empty-name"),
        ("--attribute", "name="),
        ("--max-work-units", "0"),
        ("--max-proposals", "0"),
    ] {
        let mut command = Command::cargo_bin("htmlcut").expect("binary");
        command
            .args([
                "inspect",
                "propose",
                "--input-html",
                "<main>One</main>",
                "--path",
                "html:html[1]/html:body[1]/html:main[1]",
                "--namespace",
                "html",
                "--local-name",
                "main",
                "--text-digest-sha256",
                "0000000000000000000000000000000000000000000000000000000000000000",
                flag,
                value,
            ])
            .assert()
            .failure();
    }
}

#[test]
fn propose_command_accepts_each_closed_namespace_before_fail_closed_resolution() {
    for namespace in ["svg", "mathml"] {
        let mut command = Command::cargo_bin("htmlcut").expect("binary");
        command
            .args([
                "inspect",
                "propose",
                "--input-html",
                "<main>One</main>",
                "--path",
                "svg:svg[1]",
                "--namespace",
                namespace,
                "--local-name",
                "svg",
                "--text-digest-sha256",
                "0000000000000000000000000000000000000000000000000000000000000000",
            ])
            .assert()
            .failure()
            .code(4);
    }
}

#[test]
fn propose_command_rejects_an_unknown_namespace_before_target_resolution() {
    let mut command = Command::cargo_bin("htmlcut").expect("binary");
    command
        .args([
            "inspect",
            "propose",
            "--input-html",
            "<main>One</main>",
            "--path",
            "html:html[1]/html:body[1]/html:main[1]",
            "--namespace",
            "unknown",
            "--local-name",
            "main",
            "--text-digest-sha256",
            "0000000000000000000000000000000000000000000000000000000000000000",
        ])
        .assert()
        .failure()
        .code(2);
}

#[test]
fn propose_command_rejects_each_empty_attribute_component_before_source_preparation() {
    for attribute in ["=value", "name="] {
        let mut command = Command::cargo_bin("htmlcut").expect("binary");
        command
            .args([
                "inspect",
                "propose",
                "--input-html",
                "<main>One</main>",
                "--path",
                "html:html[1]/html:body[1]/html:main[1]",
                "--namespace",
                "html",
                "--local-name",
                "main",
                "--text-digest-sha256",
                "0000000000000000000000000000000000000000000000000000000000000000",
                "--attribute",
                attribute,
            ])
            .assert()
            .failure()
            .code(2)
            .stdout(predicate::str::contains(
                "--attribute NAME and VALUE must both be non-empty.",
            ));
    }
}

#[test]
fn propose_command_resolves_a_complete_target_and_writes_json() {
    let digest = htmlcut_core::interop::v2::normalized_dom_text_digest("One");
    let output_dir = tempdir().expect("tempdir");
    let output_path = output_dir.path().join("target.json");
    let mut command = Command::cargo_bin("htmlcut").expect("binary");
    command
        .args([
            "inspect",
            "propose",
            "--input-html",
            "<main data-anchor=\"stable\">One</main>",
            "--path",
            "html:html[1]/html:body[1]/html:main[1]",
            "--namespace",
            "html",
            "--local-name",
            "main",
            "--text-digest-sha256",
            digest.as_str(),
            "--attribute",
            "data-anchor=stable",
            "--output-file",
            output_path.to_string_lossy().as_ref(),
        ])
        .assert()
        .success()
        .stderr(predicate::str::contains("wrote output file"));
    assert!(
        fs::read_to_string(&output_path)
            .expect("target output")
            .contains("htmlcut.target_resolution_result")
    );
}

#[test]
fn exploration_commands_reject_invalid_output_targets_and_source_configuration() {
    let output_dir = tempdir().expect("tempdir");
    let existing_output = output_dir.path().join("already-exists.json");
    fs::write(&existing_output, "existing").expect("existing output");
    for command_name in ["elements", "propose"] {
        let mut command = Command::cargo_bin("htmlcut").expect("binary");
        let mut arguments = vec![
            "inspect".to_owned(),
            command_name.to_owned(),
            "--input-html".to_owned(),
            "<main>One</main>".to_owned(),
            "--output-file".to_owned(),
            existing_output.to_string_lossy().into_owned(),
        ];
        if command_name == "propose" {
            arguments.extend([
                "--path".to_owned(),
                "html:html[1]/html:body[1]/html:main[1]".to_owned(),
                "--namespace".to_owned(),
                "html".to_owned(),
                "--local-name".to_owned(),
                "main".to_owned(),
                "--text-digest-sha256".to_owned(),
                "0000000000000000000000000000000000000000000000000000000000000000".to_owned(),
            ]);
        }
        command.args(arguments).assert().failure().code(5);
    }

    for command_name in ["elements", "propose"] {
        let mut command = Command::cargo_bin("htmlcut").expect("binary");
        let mut arguments = vec![
            "inspect".to_owned(),
            command_name.to_owned(),
            "--input-html".to_owned(),
            "<main>One</main>".to_owned(),
            "--base-url".to_owned(),
            "ftp://example.test".to_owned(),
        ];
        if command_name == "propose" {
            arguments.extend([
                "--path".to_owned(),
                "html:html[1]/html:body[1]/html:main[1]".to_owned(),
                "--namespace".to_owned(),
                "html".to_owned(),
                "--local-name".to_owned(),
                "main".to_owned(),
                "--text-digest-sha256".to_owned(),
                "0000000000000000000000000000000000000000000000000000000000000000".to_owned(),
            ]);
        }
        command.args(arguments).assert().failure().code(2);
    }

    for command_name in ["elements", "propose"] {
        let mut command = Command::cargo_bin("htmlcut").expect("binary");
        let mut arguments = vec![
            "inspect".to_owned(),
            command_name.to_owned(),
            "--input-html".to_owned(),
            "<main>One</main>".to_owned(),
            "--fetch-timeout-ms".to_owned(),
            "0".to_owned(),
        ];
        if command_name == "propose" {
            arguments.extend([
                "--path".to_owned(),
                "html:html[1]/html:body[1]/html:main[1]".to_owned(),
                "--namespace".to_owned(),
                "html".to_owned(),
                "--local-name".to_owned(),
                "main".to_owned(),
                "--text-digest-sha256".to_owned(),
                "0000000000000000000000000000000000000000000000000000000000000000".to_owned(),
            ]);
        }
        command.args(arguments).assert().failure().code(2);
    }
}
