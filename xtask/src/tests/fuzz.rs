use super::*;

#[test]
fn fuzz_smoke_targets_stay_in_the_canonical_inventory_order() {
    assert_eq!(
        fuzz_smoke_targets(),
        &[
            "parse_document_bytes",
            "selector_parsing",
            "slice_boundaries",
            "extraction_request_building",
        ]
    );
}

#[test]
fn assert_known_fuzz_target_rejects_unknown_names() {
    let error = assert_known_fuzz_target("unknown-target").expect_err("unknown target should fail");

    assert!(
        error
            .to_string()
            .contains("unknown fuzz target `unknown-target`")
    );
    assert!(error.to_string().contains("parse_document_bytes"));
}

#[test]
fn fuzz_smoke_command_uses_the_staged_corpus_and_runs_budget() {
    let staged_corpus = Path::new("/tmp/htmlcut-fuzz/selector_parsing");
    let command = fuzz_smoke_command("selector_parsing", staged_corpus, 77).expect("command");

    assert_eq!(command.program, PathBuf::from("cargo"));
    assert_eq!(
        command.args,
        vec![
            "+nightly",
            "fuzz",
            "run",
            "--fuzz-dir",
            "fuzz",
            "selector_parsing",
            "/tmp/htmlcut-fuzz/selector_parsing",
            "--",
            "-runs=77",
        ]
    );
    assert!(!command.quiet_stdout);
    assert!(command.force_clang);
}

#[test]
fn fuzz_smoke_preflight_requires_nightly_and_cargo_fuzz() {
    assert_eq!(
        fuzz_smoke_preflight_failures("", false),
        vec![
            FuzzSmokePreflightFailure::MissingNightlyToolchain,
            FuzzSmokePreflightFailure::MissingCargoFuzz,
        ]
    );
    assert_eq!(
        fuzz_smoke_preflight_failures("nightly-aarch64-apple-darwin", true),
        Vec::new()
    );
}

#[test]
fn fuzz_smoke_preflight_message_lists_every_missing_prerequisite() {
    let message = fuzz_smoke_preflight_message(&[
        FuzzSmokePreflightFailure::MissingNightlyToolchain,
        FuzzSmokePreflightFailure::MissingCargoFuzz,
    ]);

    assert!(message.contains("requires nightly plus the `cargo-fuzz` runner"));
    assert!(message.contains("rustup toolchain install nightly --profile minimal"));
    assert!(message.contains("CC=clang CXX=clang++ cargo install cargo-fuzz --locked"));
}

#[test]
fn fuzz_smoke_preflight_message_lists_only_the_missing_prerequisite() {
    let nightly_only =
        fuzz_smoke_preflight_message(&[FuzzSmokePreflightFailure::MissingNightlyToolchain]);
    assert!(nightly_only.contains("rustup toolchain install nightly --profile minimal"));
    assert!(!nightly_only.contains("cargo install cargo-fuzz"));

    let cargo_fuzz_only =
        fuzz_smoke_preflight_message(&[FuzzSmokePreflightFailure::MissingCargoFuzz]);
    assert!(cargo_fuzz_only.contains("cargo install cargo-fuzz --locked"));
    assert!(!cargo_fuzz_only.contains("rustup toolchain install nightly"));
}

#[test]
fn cargo_fuzz_probe_command_stays_quiet_and_does_not_force_clang() {
    let command = cargo_fuzz_probe_command();

    assert_eq!(command.program, PathBuf::from("cargo"));
    assert_eq!(command.args, vec!["fuzz", "--help"]);
    assert!(command.quiet_stdout);
    assert!(!command.force_clang);
}

#[test]
fn stage_fuzz_corpus_copies_nested_seed_files_into_scratch_space() {
    let repo_root = tempdir().expect("repo tempdir");
    let scratch_root = tempdir().expect("scratch tempdir");
    let checked_in_corpus = repo_root.path().join("fuzz/corpus/selector_parsing");
    fs::create_dir_all(checked_in_corpus.join("nested")).expect("create nested corpus dir");
    fs::write(checked_in_corpus.join("seed-a"), "alpha").expect("write root seed");
    fs::write(checked_in_corpus.join("nested/seed-b"), "beta").expect("write nested seed");

    let staged = stage_fuzz_corpus(repo_root.path(), scratch_root.path(), "selector_parsing")
        .expect("stage fuzz corpus");

    assert_eq!(staged, scratch_root.path().join("selector_parsing"));
    assert_eq!(
        fs::read_to_string(staged.join("seed-a")).expect("read staged root seed"),
        "alpha"
    );
    assert_eq!(
        fs::read_to_string(staged.join("nested/seed-b")).expect("read staged nested seed"),
        "beta"
    );
    assert_eq!(
        fs::read_to_string(checked_in_corpus.join("seed-a")).expect("read original root seed"),
        "alpha"
    );
}

#[test]
fn stage_fuzz_corpus_reports_missing_checked_in_seed_directories() {
    let repo_root = tempdir().expect("repo tempdir");
    let scratch_root = tempdir().expect("scratch tempdir");

    let error = stage_fuzz_corpus(repo_root.path(), scratch_root.path(), "selector_parsing")
        .expect_err("missing corpus should fail");

    assert!(
        error
            .to_string()
            .contains("failed to stage fuzz corpus for `selector_parsing`")
    );
    assert!(error.to_string().contains("fuzz/corpus/selector_parsing"));
}
