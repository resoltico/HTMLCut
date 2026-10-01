mod support;
use support::invoke;

#[test]
fn generated_file_run_is_reusable_and_destinations_do_not_clobber_source() {
    let root = htmlcut_tempdir::tempdir().unwrap();
    let source = root.path().join("source.html");
    let run = root.path().join("run.json");
    let output = root.path().join("result.json");
    std::fs::write(&source, "<p id='amount'>180</p>").unwrap();
    let generated = invoke(
        &[
            "extract",
            "--file",
            source.to_str().unwrap(),
            "--css",
            "#amount",
            "--save-run",
            run.to_str().unwrap(),
        ],
        b"",
    );
    assert!(generated.status.success());
    assert!(run.is_file());
    let replay = invoke(&["run", run.to_str().unwrap()], b"");
    assert!(replay.status.success());
    assert_eq!(generated.stdout, replay.stdout);
    let published = invoke(
        &[
            "run",
            run.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
        ],
        b"",
    );
    assert!(published.status.success());
    assert!(published.stdout.is_empty());
    assert_eq!(std::fs::read(&output).unwrap(), generated.stdout);
    let collision = invoke(
        &[
            "run",
            run.to_str().unwrap(),
            "--output",
            source.to_str().unwrap(),
            "--overwrite",
        ],
        b"",
    );
    assert_eq!(collision.status.code(), Some(2));
    assert!(collision.stdout.is_empty());
    assert_eq!(
        std::fs::read_to_string(&source).unwrap(),
        "<p id='amount'>180</p>"
    );
}

#[test]
fn authored_run_paths_are_relative_to_the_run_and_unknown_or_duplicate_fields_fail() {
    let root = htmlcut_tempdir::tempdir().unwrap();
    let run = root.path().join("run.json");
    std::fs::write(root.path().join("source.html"), "<p>180</p>").unwrap();
    let plan = htmlcut_core::ExtractionPlan::css("p").unwrap();
    let valid = serde_json::json!({"schema":"htmlcut.run","version":2,"source":{"kind":"file","path":"source.html"},"plan":plan});
    std::fs::write(&run, serde_json::to_vec(&valid).unwrap()).unwrap();
    let output = invoke(&["run", run.to_str().unwrap(), "--raw"], b"");
    assert!(output.status.success());
    assert_eq!(output.stdout, b"180");
    for text in [
        serde_json::to_string(&valid)
            .unwrap()
            .replace("\"version\":2", "\"version\":2,\"version\":2"),
        serde_json::to_string(&valid).unwrap().replace(
            "\"kind\":\"file\"",
            "\"kind\":\"file\",\"authentication\":\"secret\"",
        ),
    ] {
        std::fs::write(&run, text).unwrap();
        let output = invoke(&["run", run.to_str().unwrap()], b"");
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
    }
}
