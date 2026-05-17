use super::*;
use std::cell::RefCell;
use std::rc::Rc;

#[cfg(unix)]
use std::os::unix::fs::symlink;

fn write_outdated_fixture_repo(repo_root: &Path) {
    fs::create_dir_all(repo_root.join("crates").join("htmlcut-core")).expect("create core dir");
    fs::create_dir_all(repo_root.join("xtask")).expect("create xtask dir");
    fs::create_dir_all(repo_root.join("crates").join("htmlcut-core").join("src"))
        .expect("create core src dir");
    fs::create_dir_all(repo_root.join("xtask").join("src")).expect("create xtask src dir");
    fs::write(
        repo_root.join("Cargo.toml"),
        r#"[workspace]
members = ["crates/htmlcut-core", "xtask"]
resolver = "3"

[workspace.package]
version = "10.0.0"

[patch.crates-io]
servo_arc = { path = "patches/rust/servo_arc" }
tendril = { path = "patches/rust/tendril" }
"#,
    )
    .expect("write root Cargo.toml");
    fs::write(
        repo_root
            .join("crates")
            .join("htmlcut-core")
            .join("Cargo.toml"),
        "[package]\nname = \"htmlcut-core\"\nversion = \"10.0.0\"\nedition = \"2024\"\n",
    )
    .expect("write htmlcut-core Cargo.toml");
    fs::write(
        repo_root.join("xtask").join("Cargo.toml"),
        "[package]\nname = \"xtask\"\nversion = \"10.0.0\"\nedition = \"2024\"\n",
    )
    .expect("write xtask Cargo.toml");
    fs::write(
        repo_root
            .join("crates")
            .join("htmlcut-core")
            .join("src")
            .join("lib.rs"),
        "pub fn placeholder() {}\n",
    )
    .expect("write htmlcut-core lib.rs");
    fs::write(
        repo_root.join("xtask").join("src").join("main.rs"),
        "fn main() {}\n",
    )
    .expect("write xtask main.rs");
}

fn write_member_with_full_layout(member_root: &Path) {
    for directory in [
        member_root.join("src").join("nested"),
        member_root.join("tests"),
        member_root.join("examples"),
        member_root.join("benches"),
        member_root.join("fuzz_targets"),
    ] {
        fs::create_dir_all(&directory).expect("create member directory");
    }

    fs::write(
        member_root.join("Cargo.toml"),
        "[package]\nname = \"demo\"\nversion = \"1.0.0\"\n",
    )
    .expect("write member Cargo.toml");
    fs::write(
        member_root.join("src").join("lib.rs"),
        "pub fn top_level() {}\n",
    )
    .expect("write member lib.rs");
    fs::write(
        member_root.join("src").join("nested").join("mod.rs"),
        "pub fn nested() {}\n",
    )
    .expect("write nested mod.rs");
    fs::write(
        member_root.join("tests").join("smoke.rs"),
        "#[test]\nfn smoke() {}\n",
    )
    .expect("write tests");
    fs::write(
        member_root.join("examples").join("demo.rs"),
        "fn main() {}\n",
    )
    .expect("write examples");
    fs::write(
        member_root.join("benches").join("bench.rs"),
        "fn main() {}\n",
    )
    .expect("write benches");
    fs::write(
        member_root.join("fuzz_targets").join("fuzz.rs"),
        "fn main() {}\n",
    )
    .expect("write fuzz target");
    fs::write(member_root.join("build.rs"), "fn main() {}\n").expect("write build script");
}

#[test]
fn outdated_check_command_uses_the_repo_owned_subcommand_surface() {
    let spec = crate::outdated_check_command();

    assert_eq!(spec.program, PathBuf::from("cargo"));
    assert_eq!(
        spec.args,
        vec!["run", "-p", "xtask", "--", "outdated-check"]
    );
    assert!(!command_is_quiet(&spec));
    assert!(command_uses_managed_workspace_artifacts(&spec));
}

#[test]
fn strip_patch_crates_io_for_tests_removes_only_the_crates_io_patch_table() {
    let sanitized = crate::outdated::strip_patch_crates_io_for_tests(
        r#"[workspace]
members = ["crates/htmlcut-core"]

[patch.crates-io]
servo_arc = { path = "patches/rust/servo_arc" }

[patch."https://example.com/index"]
custom = { path = "vendor/custom" }
"#,
    )
    .expect("sanitize manifest");
    let parsed = toml::from_str::<toml::Value>(&sanitized).expect("parse sanitized manifest");
    let patch = parsed
        .get("patch")
        .and_then(toml::Value::as_table)
        .expect("patch table");

    assert!(!patch.contains_key("crates-io"));
    assert!(patch.contains_key("https://example.com/index"));
}

#[test]
fn strip_patch_crates_io_for_tests_leaves_manifests_without_patch_tables_untouched() {
    let manifest = "[workspace]\nmembers = [\"crates/htmlcut-core\"]\n";

    let sanitized =
        crate::outdated::strip_patch_crates_io_for_tests(manifest).expect("sanitize manifest");

    let parsed_original = toml::from_str::<toml::Value>(manifest).expect("parse original");
    let parsed_sanitized = toml::from_str::<toml::Value>(&sanitized).expect("parse sanitized");
    assert_eq!(parsed_sanitized, parsed_original);
}

#[test]
fn materialize_outdated_workspace_copies_member_manifests_and_sanitizes_root() {
    let repo_root = tempdir().expect("tempdir");
    let snapshot_root = tempdir().expect("tempdir");
    write_outdated_fixture_repo(repo_root.path());

    crate::outdated::materialize_outdated_workspace_for_tests(
        repo_root.path(),
        snapshot_root.path(),
    )
    .expect("materialize workspace");

    let root_manifest =
        fs::read_to_string(snapshot_root.path().join("Cargo.toml")).expect("read root manifest");
    assert!(!root_manifest.contains("[patch.crates-io]"));
    assert!(
        snapshot_root
            .path()
            .join("crates")
            .join("htmlcut-core")
            .join("Cargo.toml")
            .exists()
    );
    assert!(
        snapshot_root
            .path()
            .join("xtask")
            .join("Cargo.toml")
            .exists()
    );
}

#[test]
fn materialize_outdated_workspace_reports_root_manifest_write_failures() {
    let repo_root = tempdir().expect("tempdir");
    let snapshot_root = tempdir().expect("tempdir");
    write_outdated_fixture_repo(repo_root.path());
    fs::create_dir_all(snapshot_root.path().join("Cargo.toml"))
        .expect("block root manifest with directory");

    let error = crate::outdated::materialize_outdated_workspace_for_tests(
        repo_root.path(),
        snapshot_root.path(),
    )
    .expect_err("root manifest write should fail");

    assert!(error.to_string().contains("Is a directory"));
}

#[test]
fn copy_member_package_layout_for_tests_copies_nested_directories_and_build_scripts() {
    let source_root = tempdir().expect("tempdir");
    let destination_root = tempdir().expect("tempdir");
    let source_member = source_root.path().join("member");
    write_member_with_full_layout(&source_member);

    crate::outdated::copy_member_package_layout_for_tests(
        &source_member,
        &destination_root.path().join("member"),
    )
    .expect("copy member package layout");

    for relative_path in [
        PathBuf::from("Cargo.toml"),
        PathBuf::from("src/lib.rs"),
        PathBuf::from("src/nested/mod.rs"),
        PathBuf::from("tests/smoke.rs"),
        PathBuf::from("examples/demo.rs"),
        PathBuf::from("benches/bench.rs"),
        PathBuf::from("fuzz_targets/fuzz.rs"),
        PathBuf::from("build.rs"),
    ] {
        assert!(
            destination_root
                .path()
                .join("member")
                .join(&relative_path)
                .exists(),
            "expected {} to be copied",
            relative_path.display()
        );
    }
}

#[test]
fn copy_member_package_layout_for_tests_reports_manifest_copy_failures() {
    let source_root = tempdir().expect("tempdir");
    let destination_root = tempdir().expect("tempdir");
    let source_member = source_root.path().join("member");
    let destination_member = destination_root.path().join("member");
    write_member_with_full_layout(&source_member);
    fs::create_dir_all(destination_member.join("Cargo.toml"))
        .expect("block destination manifest with directory");

    let error =
        crate::outdated::copy_member_package_layout_for_tests(&source_member, &destination_member)
            .expect_err("manifest copy should fail");

    assert!(error.to_string().contains("Is a directory"));
}

#[test]
fn copy_member_package_layout_for_tests_reports_recursive_file_copy_failures() {
    let source_root = tempdir().expect("tempdir");
    let destination_root = tempdir().expect("tempdir");
    let source_member = source_root.path().join("member");
    let destination_member = destination_root.path().join("member");
    write_member_with_full_layout(&source_member);
    fs::create_dir_all(destination_member.join("src").join("nested").join("mod.rs"))
        .expect("block nested file with directory");

    let error =
        crate::outdated::copy_member_package_layout_for_tests(&source_member, &destination_member)
            .expect_err("recursive file copy should fail");

    assert!(error.to_string().contains("Is a directory"));
}

#[test]
#[cfg(unix)]
fn copy_member_package_layout_for_tests_ignores_symlink_entries_in_recursive_directories() {
    let source_root = tempdir().expect("tempdir");
    let destination_root = tempdir().expect("tempdir");
    let source_member = source_root.path().join("member");
    let destination_member = destination_root.path().join("member");
    write_member_with_full_layout(&source_member);
    symlink(
        source_member.join("src").join("lib.rs"),
        source_member.join("src").join("linked-lib.rs"),
    )
    .expect("create symlink");

    crate::outdated::copy_member_package_layout_for_tests(&source_member, &destination_member)
        .expect("copy member package layout");

    assert!(
        !destination_member
            .join("src")
            .join("linked-lib.rs")
            .exists()
    );
    assert!(destination_member.join("src").join("lib.rs").exists());
}

#[test]
fn copy_file_for_tests_rejects_destinations_without_a_parent_directory() {
    let source_root = tempdir().expect("tempdir");
    let source_file = source_root.path().join("source.txt");
    fs::write(&source_file, "payload").expect("write source");

    let error =
        crate::outdated::copy_file_for_tests(&source_file, Path::new("/")).expect_err("copy");

    assert!(
        error
            .to_string()
            .contains("manifest destination has no parent: /")
    );
}

#[test]
fn run_outdated_check_builds_a_sanitized_snapshot_before_invoking_cargo_outdated() {
    let repo_root = tempdir().expect("tempdir");
    write_outdated_fixture_repo(repo_root.path());

    let observed = Rc::new(RefCell::new(None::<CommandSpec>));
    let observed_clone = Rc::clone(&observed);
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
                assert!(manifest_path.ends_with(Path::new("workspace").join("Cargo.toml")));
                *observed_clone.borrow_mut() = Some(spec.clone());
                return Some(Ok(()));
            }
            None
        },
        || crate::outdated::run_outdated_check(repo_root.path()),
    )
    .expect("outdated check");

    let spec = observed
        .borrow()
        .clone()
        .expect("captured outdated command");
    assert_eq!(
        spec.args[0..5],
        [
            "outdated",
            "--workspace",
            "--root-deps-only",
            "--exit-code",
            "1"
        ]
    );
}
