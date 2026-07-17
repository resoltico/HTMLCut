use super::*;

#[test]
fn tracked_files_skip_missing_roots_non_rust_entries_and_explicit_exclusions() {
    let repo_root = tempdir().expect("tempdir");
    let cli_src = repo_root.path().join("crates/htmlcut-cli/src");
    fs::create_dir_all(cli_src.join("nested")).expect("create nested cli src");
    fs::create_dir_all(cli_src.join("tests")).expect("create cli test dir");
    fs::create_dir_all(cli_src.join("model")).expect("create cli model dir");

    fs::write(cli_src.join("lookup.rs"), "// tracked\n").expect("write lookup");
    fs::write(cli_src.join("nested/report.rs"), "// tracked\n").expect("write nested report");
    fs::write(cli_src.join("main.rs"), "// skipped main\n").expect("write main");
    fs::write(cli_src.join("notes.txt"), "ignore").expect("write note");
    fs::write(cli_src.join("tests/helper.rs"), "// skipped test module")
        .expect("write test helper");
    fs::write(cli_src.join("tests.rs"), "// skipped test module root")
        .expect("write test module root");
    fs::write(cli_src.join("model/catalog.rs"), "// skipped declarative")
        .expect("write excluded catalog model");

    let tracked = tracked_files(repo_root.path()).expect("tracked files");
    let tracked_paths = tracked
        .values()
        .map(|tracked_file| tracked_file.display_path.clone())
        .collect::<Vec<_>>();

    assert!(tracked_paths.contains(&"crates/htmlcut-cli/src/lookup.rs".to_owned()));
    assert!(tracked_paths.contains(&"crates/htmlcut-cli/src/nested/report.rs".to_owned()));
    assert!(!tracked_paths.contains(&"crates/htmlcut-cli/src/main.rs".to_owned()));
    assert!(!tracked_paths.contains(&"crates/htmlcut-cli/src/tests/helper.rs".to_owned()));
    assert!(!tracked_paths.contains(&"crates/htmlcut-cli/src/tests.rs".to_owned()));
    assert!(!tracked_paths.contains(&"crates/htmlcut-cli/src/model/catalog.rs".to_owned()));
}

#[test]
fn tracked_files_use_git_inventory_when_available() {
    let repo_root = tempdir().expect("tempdir");
    fs::write(repo_root.path().join(".git"), "gitdir: /tmp/htmlcut.git\n").expect("write .git");
    let tracked_file = repo_root.path().join("crates/htmlcut-cli/src/execute.rs");
    let ignored_local_file = repo_root
        .path()
        .join("crates/htmlcut-cli/src/local_only.rs");
    let skipped_main = repo_root.path().join("xtask/src/main.rs");
    let outside_root_rs = repo_root.path().join("scripts/helper.rs");
    let non_rust_file = repo_root.path().join("Cargo.toml");
    fs::create_dir_all(tracked_file.parent().expect("parent")).expect("create tracked parent");
    fs::write(&tracked_file, "// tracked\n").expect("write tracked file");
    fs::write(&ignored_local_file, "// local only\n").expect("write local-only file");
    fs::create_dir_all(skipped_main.parent().expect("parent")).expect("create xtask parent");
    fs::write(&skipped_main, "// main\n").expect("write skipped main");
    fs::create_dir_all(outside_root_rs.parent().expect("parent")).expect("create scripts dir");
    fs::write(&outside_root_rs, "// helper\n").expect("write outside-root helper");
    fs::write(&non_rust_file, "[workspace]\n").expect("write Cargo.toml");

    let tracked = crate::command_exec::with_capture_command_output_override(
        |_, spec| {
            (spec.program == Path::new("git"))
                .then(|| {
                    Ok(
                        b"Cargo.toml\0crates/htmlcut-cli/src/execute.rs\0scripts/helper.rs\0xtask/src/main.rs\0".to_vec(),
                    )
                })
        },
        || tracked_files(repo_root.path()),
    )
    .expect("tracked files");
    let tracked_paths = tracked
        .values()
        .map(|tracked_file| tracked_file.display_path.clone())
        .collect::<Vec<_>>();

    assert_eq!(
        tracked_paths,
        vec!["crates/htmlcut-cli/src/execute.rs".to_owned()]
    );
    assert!(crate::coverage::is_under_coverage_root_for_tests(
        "xtask/src"
    ));
    assert!(crate::coverage::is_under_coverage_root_for_tests(
        "xtask/src/lib.rs"
    ));
    assert!(!crate::coverage::is_under_coverage_root_for_tests(
        "scripts/helper.rs"
    ));
}

#[cfg(unix)]
#[test]
fn tracked_files_reject_entries_that_resolve_outside_the_repo_root() {
    let repo_root = tempdir().expect("repo tempdir");
    let outside_root = tempdir().expect("outside tempdir");
    let cli_src = repo_root.path().join("crates/htmlcut-cli/src");
    fs::create_dir_all(&cli_src).expect("create cli src");
    let outside_file = outside_root.path().join("escaped.rs");
    fs::write(&outside_file, "// outside\n").expect("write outside file");
    symlink_file(&outside_file, &cli_src.join("escaped.rs"));

    let error = tracked_files(repo_root.path()).expect_err("symlink should escape the repo");

    assert!(error.to_string().contains("does not live under repo root"));
}

#[test]
fn repo_relative_source_path_rejects_paths_outside_the_repo_root() {
    let repo_root = tempdir().expect("repo tempdir");
    let outside_root = tempdir().expect("outside tempdir");
    let outside_file = outside_root.path().join("lookup.rs");
    fs::write(&outside_file, "// outside\n").expect("write outside file");
    let absolute_outside_file =
        normalize_path(repo_root.path(), &outside_file).expect("normalize outside file");

    let error = crate::coverage::repo_relative_source_path_for_tests(
        repo_root.path(),
        &absolute_outside_file,
    )
    .expect_err("outside paths should fail");

    assert!(error.to_string().contains("does not live under repo root"));
}
