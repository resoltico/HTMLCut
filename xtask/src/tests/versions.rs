use super::*;

#[test]
fn workspace_version_from_manifest_extracts_workspace_package_version() {
    let version = workspace_version_from_manifest(
        "[workspace.package]\nversion = \"3.1.4\"\nedition = \"2024\"\n",
    )
    .expect("workspace version");

    assert_eq!(version, "3.1.4");
}

#[test]
fn workspace_version_from_manifest_ignores_versions_outside_workspace_package() {
    let version = workspace_version_from_manifest(
        "[package]\nversion = \"0.1.0\"\n\n[workspace.package]\nversion = \"3.1.4\"\n\n[dependencies]\nserde = { version = \"1.0.228\" }\n",
    )
    .expect("workspace version");

    assert_eq!(version, "3.1.4");
}

#[test]
fn workspace_version_from_manifest_skips_blank_and_comment_lines_inside_section() {
    let version = workspace_version_from_manifest(
        "[workspace.package]\n\n# maintained here\nversion = \"3.1.4\"\n",
    )
    .expect("workspace version");

    assert_eq!(version, "3.1.4");
}

#[test]
fn workspace_version_from_manifest_requires_a_version_line() {
    let error = workspace_version_from_manifest("[workspace.package]\nedition = \"2024\"\n")
        .expect_err("missing version should fail");

    assert_eq!(
        error.to_string(),
        "workspace version not found in Cargo.toml"
    );
}

#[test]
fn workspace_rust_version_from_manifest_extracts_workspace_package_floor() {
    let version = workspace_rust_version_from_manifest(
        "[workspace.package]\nversion = \"3.1.4\"\nrust-version = \"1.95.0\"\nedition = \"2024\"\n",
    )
    .expect("workspace rust-version");

    assert_eq!(version, "1.95.0");
}

#[test]
fn workspace_rust_version_from_manifest_requires_a_rust_version_line() {
    let error = workspace_rust_version_from_manifest("[workspace.package]\nversion = \"3.1.4\"\n")
        .expect_err("missing rust-version should fail");

    assert_eq!(
        error.to_string(),
        "workspace rust-version not found in Cargo.toml"
    );
}

#[test]
fn package_version_from_manifest_extracts_package_version() {
    let version = package_version_from_manifest(
        "[package]\nname = \"htmlcut-core\"\nversion = \"2.7.0\"\n\n[workspace]\n",
    )
    .expect("package version");

    assert_eq!(version, "2.7.0");
}

#[test]
fn package_version_from_manifest_requires_a_version_line() {
    let error = package_version_from_manifest("[package]\nname = \"htmlcut-core\"\n")
        .expect_err("missing version should fail");

    assert_eq!(error.to_string(), "package version not found in Cargo.toml");
}

#[test]
fn manifest_version_parsers_skip_malformed_headers_and_assignments() {
    let workspace_version = workspace_version_from_manifest(
        "[workspace.package\nversion = \"0.1.0\"\n[workspace.package]\nversion = 3.1.4\nversion = \"3.1.4\nversion = \"3.1.4\"\n",
    )
    .expect("workspace version");
    let package_version = package_version_from_manifest(
        "[package\nversion = \"0.1.0\"\n[package]\nversion = 2.7.0\nversion = \"2.7.0\nversion = \"2.7.0\"\n",
    )
    .expect("package version");

    assert_eq!(workspace_version, "3.1.4");
    assert_eq!(package_version, "2.7.0");
}

#[test]
fn package_version_from_manifest_ignores_workspace_package_versions() {
    let version = package_version_from_manifest(
        "[workspace.package]\nversion = \"9.9.9\"\n\n[package]\nname = \"htmlcut-core\"\nversion = \"2.7.0\"\n",
    )
    .expect("package version");

    assert_eq!(version, "2.7.0");
}

#[test]
fn workspace_version_reads_from_repo_manifest() {
    let repo_root = tempdir().expect("tempdir");
    fs::write(
        repo_root.path().join("Cargo.toml"),
        "[workspace.package]\nversion = \"9.9.9\"\nrust-version = \"1.95.0\"\n",
    )
    .expect("write Cargo.toml");

    let version = workspace_version(repo_root.path()).expect("workspace version");

    assert_eq!(version, "9.9.9");
}

#[test]
fn workspace_rust_version_reads_from_repo_manifest() {
    let repo_root = tempdir().expect("tempdir");
    fs::write(
        repo_root.path().join("Cargo.toml"),
        "[workspace.package]\nversion = \"9.9.9\"\nrust-version = \"1.95.0\"\n",
    )
    .expect("write Cargo.toml");

    let version = workspace_rust_version(repo_root.path()).expect("workspace rust-version");

    assert_eq!(version, "1.95.0");
}

#[test]
fn repo_manifests_publish_the_verified_rust_version_floor() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("repo root");
    let workspace_manifest =
        fs::read_to_string(repo_root.join("Cargo.toml")).expect("workspace manifest");
    let core_manifest = fs::read_to_string(repo_root.join("crates/htmlcut-core/Cargo.toml"))
        .expect("core manifest");
    let cli_manifest =
        fs::read_to_string(repo_root.join("crates/htmlcut-cli/Cargo.toml")).expect("cli manifest");
    let xtask_manifest =
        fs::read_to_string(repo_root.join("xtask/Cargo.toml")).expect("xtask manifest");
    let fuzz_manifest =
        fs::read_to_string(repo_root.join("fuzz/Cargo.toml")).expect("fuzz manifest");
    let toolchain_manifest =
        fs::read_to_string(repo_root.join("rust-toolchain.toml")).expect("rust-toolchain manifest");
    let repo_toolchain = repo_toolchain_from_manifest(&toolchain_manifest).expect("repo toolchain");

    assert_eq!(
        workspace_rust_version_from_manifest(&workspace_manifest).expect("workspace rust-version"),
        "1.95.0"
    );
    assert_eq!(repo_toolchain.channel, "1.95.0");
    assert_eq!(repo_toolchain.components, vec!["clippy", "rustfmt"]);
    assert!(core_manifest.contains("rust-version.workspace = true"));
    assert!(cli_manifest.contains("rust-version.workspace = true"));
    assert!(xtask_manifest.contains("rust-version.workspace = true"));
    assert!(fuzz_manifest.contains("rust-version = \"1.95.0\""));
    assert!(fuzz_manifest.contains("unsafe_code = \"warn\""));
    assert!(fuzz_manifest.contains("all = \"warn\""));
}

#[test]
fn semver_release_type_uses_major_until_the_baseline_catches_up() {
    assert_eq!(semver_release_type_from_versions("3.0.0", "2.0.0"), "major");
    assert_eq!(semver_release_type_from_versions("3.0.0", "3.0.0"), "minor");
}

#[test]
fn semver_release_type_reads_versions_from_the_repo_layout() {
    let repo_root = tempdir().expect("tempdir");
    write_repo_scaffold(repo_root.path());

    assert_eq!(
        semver_release_type(repo_root.path()).expect("major semver release type"),
        "major"
    );

    fs::write(
        repo_root
            .path()
            .join("semver-baseline")
            .join("htmlcut-core")
            .join("Cargo.toml"),
        "[package]\nname = \"htmlcut-core\"\nversion = \"3.0.0\"\n",
    )
    .expect("write updated baseline Cargo.toml");

    assert_eq!(
        semver_release_type(repo_root.path()).expect("minor semver release type"),
        "minor"
    );
}

#[test]
fn with_workspace_stub_appends_once() {
    let updated = with_workspace_stub("[package]\nname = \"htmlcut-core\"\n");
    let unchanged = with_workspace_stub("[package]\nname = \"htmlcut-core\"\n\n[workspace]\n");

    assert_eq!(
        updated,
        "[package]\nname = \"htmlcut-core\"\n\n[workspace]\n"
    );
    assert_eq!(
        unchanged,
        "[package]\nname = \"htmlcut-core\"\n\n[workspace]\n"
    );
}

#[test]
fn strip_dev_dependency_tables_drops_root_and_target_specific_dev_dependencies() {
    let manifest = "\
[package]
name = \"htmlcut-core\"

[dependencies]
serde = \"1\"

[dev-dependencies]
htmlcut-tempdir = { path = \"../htmlcut-tempdir\" }

[target.'cfg(unix)'.dev-dependencies]
tempfile = \"3\"

[features]
default = []
";

    assert_eq!(
        strip_dev_dependency_tables(manifest),
        "\
[package]
name = \"htmlcut-core\"

[dependencies]
serde = \"1\"

[features]
default = []
"
    );
}

#[test]
fn strip_dev_dependency_tables_preserves_manifests_without_dev_dependencies() {
    let manifest = "[package]\nname = \"htmlcut-core\"";

    assert_eq!(strip_dev_dependency_tables(manifest), manifest);
}

#[test]
fn strip_dev_dependency_tables_ignores_malformed_headers() {
    let manifest = "[package\nname = \"htmlcut-core\"\n[dev-dependencies\nhtmlcut-tempdir = \"1\"";

    assert_eq!(strip_dev_dependency_tables(manifest), manifest);
}
