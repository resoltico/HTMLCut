use super::*;

#[test]
fn refresh_semver_baseline_for_tests_bootstraps_missing_baseline_dirs() {
    let repo_root = tempdir().expect("repo tempdir");
    fs::write(
        repo_root.path().join("Cargo.toml"),
        "[workspace.package]\nversion = \"3.0.0\"\n",
    )
    .expect("write Cargo.toml");
    let packaged_manifest = Rc::new(RefCell::new(String::new()));
    let packaged_manifest_for_override = Rc::clone(&packaged_manifest);
    let repo_root_path = repo_root.path().to_path_buf();

    crate::command_exec::with_run_spec_override(
        move |current_root, spec| {
            let args = spec.args.iter().map(String::as_str).collect::<Vec<_>>();

            if spec.program == Path::new("tar")
                && args.first() == Some(&"-xf")
                && args.get(2) == Some(&"-C")
            {
                let snapshot_root = PathBuf::from(args[3]);
                fs::create_dir_all(snapshot_root.join("crates/htmlcut-core"))
                    .expect("create snapshot crate dir");
                fs::write(
                    snapshot_root.join("Cargo.toml"),
                    "[workspace.package]\nversion = \"4.2.0\"\n",
                )
                .expect("write snapshot workspace Cargo.toml");
                fs::write(
                    snapshot_root.join("crates/htmlcut-core/Cargo.toml"),
                    "[package]\nname = \"htmlcut-core\"\nversion = \"4.2.0\"\n",
                )
                .expect("write snapshot crate manifest");
                return Some(Ok(()));
            }

            if spec.program == Path::new("cargo")
                && args[..5]
                    == [
                        "package",
                        "--allow-dirty",
                        "--no-verify",
                        "-p",
                        "htmlcut-core",
                    ]
            {
                *packaged_manifest_for_override.borrow_mut() =
                    fs::read_to_string(current_root.join("crates/htmlcut-core/Cargo.toml"))
                        .expect("read packaged manifest");
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
                    packaged_manifest_for_override.borrow().as_str(),
                )
                .expect("write extracted manifest");
                return Some(Ok(()));
            }

            Some(Ok(()))
        },
        || refresh_semver_baseline_for_tests(repo_root.path(), "v4.2.0"),
    )
    .expect("refresh-semver-baseline should create missing baseline dirs");

    let refreshed_manifest = fs::read_to_string(
        repo_root
            .path()
            .join("semver-baseline/htmlcut-core/Cargo.toml"),
    )
    .expect("read refreshed baseline manifest");
    let refreshed_provenance = fs::read_to_string(
        repo_root
            .path()
            .join("semver-baseline/htmlcut-core/BASELINE.toml"),
    )
    .expect("read refreshed baseline provenance");
    assert!(refreshed_manifest.contains("version = \"4.2.0\""));
    assert!(refreshed_manifest.contains("\n[workspace]\n"));
    assert!(refreshed_provenance.contains("package_version = \"4.2.0\""));
    assert!(refreshed_provenance.contains("source_git_ref = \"v4.2.0\""));
}

#[test]
fn refresh_semver_baseline_for_tests_overrides_snapshot_cargo_target_layout() {
    let repo_root = tempdir().expect("repo tempdir");
    fs::write(
        repo_root.path().join("Cargo.toml"),
        "[workspace.package]\nversion = \"3.0.0\"\n",
    )
    .expect("write Cargo.toml");
    let packaged_manifest = Rc::new(RefCell::new(String::new()));
    let packaged_manifest_for_override = Rc::clone(&packaged_manifest);
    let observed_target_dir = Rc::new(RefCell::new(None::<PathBuf>));
    let observed_target_dir_for_override = Rc::clone(&observed_target_dir);
    let repo_root_path = repo_root.path().to_path_buf();
    let repo_root_path_for_override = repo_root_path.clone();

    crate::command_exec::with_run_spec_override(
        move |current_root, spec| {
            let args = spec.args.iter().map(String::as_str).collect::<Vec<_>>();

            if spec.program == Path::new("tar")
                && args.first() == Some(&"-xf")
                && args.get(2) == Some(&"-C")
            {
                let snapshot_root = PathBuf::from(args[3]);
                fs::create_dir_all(snapshot_root.join(".cargo")).expect("create snapshot .cargo dir");
                fs::create_dir_all(snapshot_root.join("crates/htmlcut-core"))
                    .expect("create snapshot crate dir");
                fs::write(
                    snapshot_root.join(".cargo/config.toml"),
                    "[build]\ntarget-dir = \"../published-artifacts/target\"\nbuild-dir = \"../published-artifacts/build\"\n",
                )
                .expect("write snapshot cargo config");
                fs::write(
                    snapshot_root.join("Cargo.toml"),
                    "[workspace.package]\nversion = \"4.2.0\"\n",
                )
                .expect("write snapshot workspace Cargo.toml");
                fs::write(
                    snapshot_root.join("crates/htmlcut-core/Cargo.toml"),
                    "[package]\nname = \"htmlcut-core\"\nversion = \"4.2.0\"\n",
                )
                .expect("write snapshot crate manifest");
                return Some(Ok(()));
            }

            if spec.program == Path::new("cargo")
                && args[..5]
                    == [
                        "package",
                        "--allow-dirty",
                        "--no-verify",
                        "-p",
                        "htmlcut-core",
                    ]
            {
                let target_dir = PathBuf::from(command_env_value(spec, "CARGO_TARGET_DIR"));
                assert!(
                    !target_dir.ends_with("published-artifacts/target"),
                    "refresh packaging should not inherit the published snapshot target-dir"
                );
                *observed_target_dir_for_override.borrow_mut() = Some(target_dir.clone());
                *packaged_manifest_for_override.borrow_mut() =
                    fs::read_to_string(current_root.join("crates/htmlcut-core/Cargo.toml"))
                        .expect("read packaged manifest");
                let archive = target_dir.join("package/htmlcut-core-4.2.0.crate");
                fs::create_dir_all(archive.parent().expect("archive parent"))
                    .expect("create archive parent");
                fs::write(&archive, "crate archive").expect("write crate archive");
                return Some(Ok(()));
            }

            if spec.program == Path::new("tar")
                && args.first() == Some(&"-xzf")
                && args.get(2) == Some(&"-C")
            {
                let extracted_dir =
                    repo_root_path_for_override.join("semver-baseline/htmlcut-core-4.2.0");
                fs::create_dir_all(&extracted_dir).expect("create extracted dir");
                fs::write(
                    extracted_dir.join("Cargo.toml"),
                    packaged_manifest_for_override.borrow().as_str(),
                )
                .expect("write extracted manifest");
                return Some(Ok(()));
            }

            Some(Ok(()))
        },
        || refresh_semver_baseline_for_tests(repo_root.path(), "v4.2.0"),
    )
    .expect("refresh-semver-baseline should override the snapshot cargo target layout");

    let observed_target_dir = observed_target_dir
        .borrow()
        .clone()
        .expect("refresh packaging should set an explicit target dir");
    assert!(
        observed_target_dir.ends_with("cargo-target"),
        "refresh packaging should use the temp-owned cargo-target root"
    );
    assert!(
        repo_root_path
            .join("semver-baseline/htmlcut-core/BASELINE.toml")
            .exists(),
        "refresh flow should still materialize the baseline provenance"
    );
}

#[cfg(unix)]
#[test]
fn refresh_semver_baseline_for_tests_rejects_symbolic_links_in_published_vendored_sources() {
    use std::os::unix::fs::symlink;

    let repo_root = tempdir().expect("repo tempdir");
    fs::write(
        repo_root.path().join("Cargo.toml"),
        "[workspace.package]\nversion = \"3.0.0\"\n",
    )
    .expect("write Cargo.toml");

    let error = crate::command_exec::with_run_spec_override(
        move |_, spec| {
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
                    fs::create_dir_all(snapshot_root.join("patches/rust").join(directory))
                        .expect("create snapshot vendor dir");
                }
                let scraper_dir = snapshot_root.join("patches/rust/scraper");
                symlink(&scraper_dir, scraper_dir.join("unexpected-link"))
                    .expect("write snapshot symbolic link");
                fs::write(
                    snapshot_root.join("Cargo.toml"),
                    "[workspace]\n\n[workspace.package]\nversion = \"4.2.0\"\n\n[workspace.dependencies]\nscraper = { package = \"htmlcut-scraper\", path = \"patches/rust/scraper\" }\n",
                )
                .expect("write snapshot workspace Cargo.toml");
                fs::write(
                    snapshot_root.join("crates/htmlcut-core/Cargo.toml"),
                    "[package]\nname = \"htmlcut-core\"\nversion = \"4.2.0\"\n",
                )
                .expect("write snapshot crate manifest");
                return Some(Ok(()));
            }
            Some(Ok(()))
        },
        || refresh_semver_baseline_for_tests(repo_root.path(), "v4.2.0"),
    )
    .expect_err("symbolic link should fail the vendored baseline copy");

    assert!(error.to_string().contains(
        "published vendored selector-stack entry is neither a regular file nor directory"
    ));
}

#[test]
fn refresh_semver_baseline_for_tests_fails_when_the_captured_vendored_stack_disappears() {
    let repo_root = tempdir().expect("repo tempdir");
    fs::write(
        repo_root.path().join("Cargo.toml"),
        "[workspace.package]\nversion = \"3.0.0\"\n",
    )
    .expect("write Cargo.toml");
    let repo_root_path = repo_root.path().to_path_buf();

    let result = crate::command_exec::with_run_spec_override(
        move |current_root, spec| {
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
                }
                fs::write(
                    snapshot_root.join("Cargo.toml"),
                    "[workspace]\n\n[workspace.package]\nversion = \"4.2.0\"\n\n[workspace.dependencies]\nscraper = { package = \"htmlcut-scraper\", path = \"patches/rust/scraper\" }\n",
                )
                .expect("write snapshot workspace Cargo.toml");
                fs::write(
                    snapshot_root.join("crates/htmlcut-core/Cargo.toml"),
                    "[package]\nname = \"htmlcut-core\"\nversion = \"4.2.0\"\n",
                )
                .expect("write snapshot crate manifest");
                return Some(Ok(()));
            }

            if spec.program == Path::new("cargo")
                && args[..5]
                    == [
                        "package",
                        "--allow-dirty",
                        "--no-verify",
                        "-p",
                        "htmlcut-core",
                    ]
            {
                let captured_stack = current_root
                    .parent()
                    .expect("snapshot parent")
                    .join("published-vendored-selector-stack");
                fs::rename(
                    &captured_stack,
                    current_root.join("moved-vendored-selector-stack"),
                )
                .expect("hide captured stack before extraction");
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
                    "[package]\nname = \"htmlcut-core\"\nversion = \"4.2.0\"\n\n[dependencies.scraper]\nversion = \"0.27.0\"\n",
                )
                .expect("write extracted manifest");
                return Some(Ok(()));
            }

            Some(Ok(()))
        },
        || refresh_semver_baseline_for_tests(repo_root.path(), "v4.2.0"),
    );

    assert!(
        result.is_err(),
        "refresh must fail rather than write a baseline that points to a missing vendored stack"
    );
}
