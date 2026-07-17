use super::*;

#[test]
fn main_entry_with_runs_the_contract_miri_proof() {
    let repo_root = tempdir().expect("repo tempdir");
    with_isolated_target_dir(repo_root.path(), || {
        write_repo_scaffold(repo_root.path());
        write_toolchain_contract(repo_root.path());
        let calls = Rc::new(RefCell::new(Vec::new()));
        let calls_for_override = Rc::clone(&calls);

        with_ready_preflight(|| {
            crate::command_exec::with_run_spec_override(
                move |_, spec| {
                    calls_for_override.borrow_mut().push(spec.clone());
                    Some(Ok(()))
                },
                || main_entry_with(repo_root.path(), ["xtask", "miri"]),
            )
        })
        .expect("xtask miri should pass");

        assert_eq!(calls.borrow().as_slice(), &[miri_contract_command()]);
    });
}

#[test]
fn main_entry_with_runs_the_dependency_freshness_check() {
    let repo_root = tempdir().expect("repo tempdir");
    with_isolated_target_dir(repo_root.path(), || {
        write_outdated_fixture_repo(repo_root.path());
        let calls = Rc::new(RefCell::new(Vec::new()));
        let calls_for_override = Rc::clone(&calls);

        crate::command_exec::with_run_spec_override(
            move |_, spec| {
                if spec.program == Path::new("cargo")
                    && spec.args.first().map(String::as_str) == Some("outdated")
                {
                    let manifest_path = spec
                        .args
                        .windows(2)
                        .find(|window| window[0] == "--manifest-path")
                        .map(|window| PathBuf::from(&window[1]))
                        .expect("manifest path");
                    let manifest_text =
                        fs::read_to_string(&manifest_path).expect("read sanitized manifest");
                    assert!(!manifest_text.contains("[patch.crates-io]"));
                }
                calls_for_override.borrow_mut().push(spec.clone());
                Some(Ok(()))
            },
            || main_entry_with(repo_root.path(), ["xtask", "outdated-check"]),
        )
        .expect("xtask outdated-check should pass");

        let calls = calls.borrow();
        assert_eq!(calls.len(), 1);
        assert_eq!(
            calls[0].args[0..5],
            [
                "outdated",
                "--workspace",
                "--root-deps-only",
                "--exit-code",
                "1"
            ]
        );
    });
}

#[test]
fn main_entry_with_runs_one_targeted_fuzz_smoke_command() {
    let repo_root = tempdir().expect("repo tempdir");
    with_isolated_target_dir(repo_root.path(), || {
        let checked_in_corpus = repo_root.path().join("fuzz/corpus/selector_parsing");
        fs::create_dir_all(&checked_in_corpus).expect("create corpus dir");
        fs::write(checked_in_corpus.join("seed"), "alpha").expect("write seed");

        let calls = Rc::new(RefCell::new(Vec::new()));
        let calls_for_override = Rc::clone(&calls);

        with_ready_preflight(|| {
            crate::command_exec::with_run_spec_override(
                move |_, spec| {
                    calls_for_override.borrow_mut().push(spec.clone());
                    Some(Ok(()))
                },
                || {
                    main_entry_with(
                        repo_root.path(),
                        [
                            "xtask",
                            "fuzz-smoke",
                            "--target",
                            "selector_parsing",
                            "--runs",
                            "13",
                        ],
                    )
                },
            )
        })
        .expect("targeted fuzz-smoke should pass");

        let calls = calls.borrow();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].program, PathBuf::from("cargo"));
        assert!(calls[0].args.iter().any(|arg| arg == "selector_parsing"));
        assert!(calls[0].args.iter().any(|arg| arg == "-runs=13"));
    });
}

#[test]
fn fuzz_smoke_propagates_a_target_execution_failure() {
    let repo_root = tempdir().expect("repo tempdir");
    with_isolated_target_dir(repo_root.path(), || {
        let checked_in_corpus = repo_root.path().join("fuzz/corpus/selector_parsing");
        fs::create_dir_all(&checked_in_corpus).expect("create corpus dir");
        fs::write(checked_in_corpus.join("seed"), "alpha").expect("write seed");

        let error = with_ready_preflight(|| {
            crate::command_exec::with_run_spec_override(
                |_, _| Some(Err("fuzz fixture failed".into())),
                || {
                    main_entry_with(
                        repo_root.path(),
                        ["xtask", "fuzz-smoke", "--target", "selector_parsing"],
                    )
                },
            )
        })
        .expect_err("failed fuzz target should stop the gate");

        assert!(error.to_string().contains("fuzz fixture failed"));
    });
}

#[test]
fn main_entry_with_rejects_unknown_fuzz_targets_before_tool_preflight() {
    let repo_root = tempdir().expect("repo tempdir");

    with_isolated_target_dir(repo_root.path(), || {
        let error = main_entry_with(
            repo_root.path(),
            ["xtask", "fuzz-smoke", "--target", "not-real"],
        )
        .expect_err("unknown fuzz target should fail before preflight");

        let message = error.to_string();
        assert!(message.contains("unknown fuzz target `not-real`"));
        assert!(
            !message.contains("missing prerequisites"),
            "unknown target should not depend on tool preflight: {message}"
        );
    });
}

#[test]
fn main_entry_with_runs_the_full_fuzz_smoke_inventory() {
    let repo_root = tempdir().expect("repo tempdir");
    with_isolated_target_dir(repo_root.path(), || {
        for target in fuzz_smoke_targets() {
            let checked_in_corpus = repo_root.path().join("fuzz/corpus").join(target);
            fs::create_dir_all(&checked_in_corpus).expect("create corpus dir");
            fs::write(checked_in_corpus.join("seed"), target).expect("write seed");
        }

        let call_count = Rc::new(RefCell::new(0usize));
        let call_count_for_override = Rc::clone(&call_count);

        with_ready_preflight(|| {
            crate::command_exec::with_run_spec_override(
                move |_, _| {
                    *call_count_for_override.borrow_mut() += 1;
                    Some(Ok(()))
                },
                || main_entry_with(repo_root.path(), ["xtask", "fuzz-smoke", "--runs", "5"]),
            )
        })
        .expect("full fuzz-smoke inventory should pass");

        assert_eq!(*call_count.borrow(), fuzz_smoke_targets().len());
    });
}

#[test]
fn main_entry_with_refreshes_the_semver_baseline_snapshot() {
    let repo_root = tempdir().expect("repo tempdir");
    fs::write(
        repo_root.path().join("Cargo.toml"),
        "[workspace.package]\nversion = \"3.0.0\"\n",
    )
    .expect("write Cargo.toml");
    let baseline_parent = repo_root.path().join("semver-baseline");
    let baseline_dir = baseline_parent.join("htmlcut-core");
    let stale_extracted_dir = baseline_parent.join("htmlcut-core-4.2.0");
    fs::create_dir_all(&baseline_dir).expect("create baseline dir");
    fs::create_dir_all(&stale_extracted_dir).expect("create stale extracted dir");
    fs::write(baseline_dir.join("Cargo.toml"), "old baseline\n").expect("write old baseline");
    fs::write(stale_extracted_dir.join("Cargo.toml"), "stale\n").expect("write stale manifest");

    let calls = Rc::new(RefCell::new(Vec::new()));
    let calls_for_override = Rc::clone(&calls);
    let packaged_manifest = Rc::new(RefCell::new(String::new()));
    let packaged_manifest_for_override = Rc::clone(&packaged_manifest);
    let repo_root_path = repo_root.path().to_path_buf();

    crate::command_exec::with_run_spec_override(
        move |current_root, spec| {
            calls_for_override.borrow_mut().push(spec.clone());
            let args = spec.args.iter().map(String::as_str).collect::<Vec<_>>();

            if spec.program == Path::new("tar")
                && args.first() == Some(&"-xf")
                && args.get(2) == Some(&"-C")
            {
                let snapshot_root = PathBuf::from(args[3]);
                fs::create_dir_all(snapshot_root.join("crates/htmlcut-core"))
                    .expect("create snapshot crate dir");
                for directory in [
                    "html5ever",
                    "markup5ever",
                    "scraper",
                    "selectors",
                    "servo_arc",
                    "tendril",
                ] {
                    let vendor_dir = snapshot_root.join("patches/rust").join(directory);
                    fs::create_dir_all(&vendor_dir).expect("create snapshot vendor dir");
                    fs::write(vendor_dir.join("source.rs"), directory)
                        .expect("write snapshot vendor source");
                    fs::create_dir_all(vendor_dir.join("nested"))
                        .expect("create nested snapshot vendor dir");
                    fs::write(vendor_dir.join("nested/source.rs"), directory)
                        .expect("write nested snapshot vendor source");
                }
                fs::write(
                    snapshot_root.join("Cargo.toml"),
                    "[workspace]\nresolver = \"3\"\n\n[workspace.package]\nversion = \"4.2.0\"\n\n[workspace.dependencies]\nscraper = { package = \"htmlcut-scraper\", path = \"patches/rust/scraper\", version = \"0.27.0-htmlcut.1\", default-features = false, features = [\"errors\"] }\nselectors = { package = \"htmlcut-selectors\", path = \"patches/rust/selectors\", version = \"0.38.0-htmlcut.1\" }\n",
                )
                .expect("write snapshot workspace Cargo.toml");
                fs::write(
                    snapshot_root.join("crates/htmlcut-core/Cargo.toml"),
                    "[package]\nname = \"htmlcut-core\"\nversion = \"4.2.0\"\n\n[dev-dependencies]\ninsta = \"1\"\n",
                )
                .expect("write snapshot crate manifest");
                return Some(Ok(()));
            }

            if spec.program == Path::new("cargo") && args[..5] == ["package", "--allow-dirty", "--no-verify", "-p", "htmlcut-core"] {
                let workspace_manifest =
                    fs::read_to_string(current_root.join("Cargo.toml"))
                        .expect("read sanitized workspace manifest");
                assert!(
                    !workspace_manifest.contains("htmlcut-scraper"),
                    "refresh packaging should rewrite vendored package aliases back to registry coordinates"
                );
                assert!(
                    !workspace_manifest.contains("patches/rust/scraper"),
                    "refresh packaging should drop repo-owned patch paths from the packaged snapshot workspace"
                );
                assert!(
                    workspace_manifest.contains("version = \"0.27.0\""),
                    "refresh packaging should keep the upstream dependency version in the sanitized workspace manifest"
                );
                let manifest = fs::read_to_string(current_root.join("crates/htmlcut-core/Cargo.toml"))
                    .expect("read stripped snapshot manifest");
                assert!(
                    !manifest.contains("[dev-dependencies]"),
                    "snapshot manifest should be stripped before packaging"
                );
                *packaged_manifest_for_override.borrow_mut() = manifest;
                let archive = PathBuf::from(command_env_value(spec, "CARGO_TARGET_DIR"))
                    .join("package/htmlcut-core-4.2.0.crate");
                fs::create_dir_all(archive.parent().expect("archive parent"))
                    .expect("create archive parent");
                fs::write(&archive, "crate archive").expect("write crate archive");
                return Some(Ok(()));
            }

            if spec.program == Path::new("tar")
                && args.first() == Some(&"-xzf")
                && args.get(2) == Some(&"-C")
            {
                let extracted_dir = repo_root_path.join("semver-baseline/htmlcut-core-4.2.0");
                fs::create_dir_all(&extracted_dir).expect("create extracted dir");
                fs::write(
                    extracted_dir.join("Cargo.toml"),
                    "[package]\nname = \"htmlcut-core\"\nversion = \"4.2.0\"\n\n[dependencies.scraper]\nversion = \"0.27.0\"\ndefault-features = false\nfeatures = [\"errors\"]\n\n[dependencies.selectors]\nversion = \"0.38.0\"\n",
                )
                    .expect("write extracted manifest");
                return Some(Ok(()));
            }

            Some(Ok(()))
        },
        || {
            main_entry_with(
                repo_root.path(),
                ["xtask", "refresh-semver-baseline", "--git-ref", "v4.2.0"],
            )
        },
    )
    .expect("refresh-semver-baseline should pass");

    let refreshed_manifest = fs::read_to_string(baseline_dir.join("Cargo.toml"))
        .expect("read refreshed baseline manifest");
    let refreshed_provenance = fs::read_to_string(baseline_dir.join("BASELINE.toml"))
        .expect("read refreshed baseline provenance");
    assert!(refreshed_manifest.contains("version = \"4.2.0\""));
    assert!(!refreshed_manifest.contains("[dev-dependencies]"));
    assert!(refreshed_manifest.contains("\n[workspace]\n"));
    assert!(refreshed_manifest.contains("package = \"htmlcut-scraper\""));
    assert!(refreshed_manifest.contains("path = \"vendor/scraper\""));
    assert!(refreshed_manifest.contains("package = \"htmlcut-selectors\""));
    assert!(baseline_dir.join("vendor/scraper/source.rs").exists());
    assert!(
        baseline_dir
            .join("vendor/scraper/nested/source.rs")
            .exists()
    );
    assert!(baseline_dir.join("vendor/tendril/source.rs").exists());
    assert!(refreshed_provenance.contains("schema = \"htmlcut.semver_baseline_provenance@1\""));
    assert!(refreshed_provenance.contains("package = \"htmlcut-core\""));
    assert!(refreshed_provenance.contains("package_version = \"4.2.0\""));
    assert!(refreshed_provenance.contains("source_git_ref = \"v4.2.0\""));
    assert!(
        refreshed_provenance
            .contains("refresh_command = \"cargo xtask refresh-semver-baseline --git-ref v4.2.0\"")
    );
    assert!(
        !stale_extracted_dir.exists(),
        "stale extracted dir should be replaced"
    );
    assert!(
        calls
            .borrow()
            .iter()
            .any(|spec| spec.program == Path::new("git")),
        "refresh flow should archive the requested git ref"
    );
    let package_spec = calls
        .borrow()
        .iter()
        .find(|spec| spec.program == Path::new("cargo"))
        .expect("package spec should be recorded")
        .clone();
    assert!(
        command_env_value(&package_spec, "CARGO_TARGET_DIR").contains("cargo-target"),
        "refresh packaging should use the isolated package target root"
    );
    assert!(
        command_env_value(&package_spec, "CARGO_BUILD_BUILD_DIR").contains("cargo-build"),
        "refresh packaging should use the isolated package build root"
    );
}
