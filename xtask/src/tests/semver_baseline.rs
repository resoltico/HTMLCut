use super::*;

#[test]
fn materialized_semver_baseline_restores_inert_vendored_manifests_only_in_scratch() {
    let repo_root = tempdir().expect("repo tempdir");
    let baseline = repo_root.path().join("semver-baseline/htmlcut-core");
    let vendor = baseline.join("vendor/htmlcut-scraper");
    fs::create_dir_all(&vendor).expect("create frozen vendor directory");
    fs::write(
        baseline.join("Cargo.toml"),
        "[package]\nname = \"htmlcut-core\"\nversion = \"12.0.0\"\n",
    )
    .expect("write baseline manifest");
    fs::write(vendor.join("lib.rs"), "pub fn frozen() {}\n").expect("write frozen source");
    fs::write(
        vendor.join("Cargo.toml.htmlcut-baseline"),
        "[package]\nname = \"htmlcut-scraper\"\nversion = \"0.27.0-htmlcut.1\"\n",
    )
    .expect("write inert vendor manifest");

    let scratch = repo_root.path().join("scratch/baseline");
    let materialized = crate::plan::materialize_semver_baseline(repo_root.path(), &scratch)
        .expect("materialize semver baseline");

    assert_eq!(materialized, scratch);
    assert!(vendor.join("Cargo.toml.htmlcut-baseline").exists());
    assert!(!vendor.join("Cargo.toml").exists());
    assert_eq!(
        fs::read_to_string(scratch.join("vendor/htmlcut-scraper/Cargo.toml"))
            .expect("read restored vendor manifest"),
        "[package]\nname = \"htmlcut-scraper\"\nversion = \"0.27.0-htmlcut.1\"\n"
    );
    assert!(
        !scratch
            .join("vendor/htmlcut-scraper/Cargo.toml.htmlcut-baseline")
            .exists()
    );
}

#[cfg(unix)]
#[test]
fn materialized_semver_baseline_rejects_symbolic_links() {
    use std::os::unix::fs::symlink;

    let repo_root = tempdir().expect("repo tempdir");
    let baseline = repo_root.path().join("semver-baseline/htmlcut-core");
    fs::create_dir_all(&baseline).expect("create baseline");
    let target = baseline.join("target");
    fs::write(&target, "frozen source").expect("write target");
    symlink(&target, baseline.join("unexpected-link")).expect("write symbolic link");

    let error = crate::plan::materialize_semver_baseline(
        repo_root.path(),
        &repo_root.path().join("scratch/baseline"),
    )
    .expect_err("symbolic link should be rejected");

    assert!(
        error
            .to_string()
            .contains("semver baseline contains a non-regular entry")
    );
}

#[test]
fn semver_gate_replaces_its_baseline_with_the_materialized_snapshot() {
    let spec = CommandSpec::new(
        "cargo",
        ["semver-checks", "--baseline-root", "checked-in-baseline"],
        CommandStdout::Quiet,
        CommandToolchainEnv::Inherit,
    );
    let materialized = Path::new("/tmp/htmlcut-semver-baseline");

    let rewritten = semver_spec_with_materialized_baseline_for_tests(spec, materialized)
        .expect("rewrite baseline root");

    assert_eq!(
        rewritten.args,
        vec![
            "semver-checks".to_owned(),
            "--baseline-root".to_owned(),
            "/tmp/htmlcut-semver-baseline".to_owned(),
        ]
    );
}

#[test]
fn semver_gate_requires_a_baseline_root_flag() {
    let spec = CommandSpec::new(
        "cargo",
        ["semver-checks"],
        CommandStdout::Quiet,
        CommandToolchainEnv::Inherit,
    );

    let error = semver_spec_with_materialized_baseline_for_tests(spec, Path::new("/tmp/baseline"))
        .expect_err("missing baseline flag should fail");

    assert_eq!(
        error.to_string(),
        "semver gate command is missing --baseline-root"
    );
}

#[test]
fn semver_gate_requires_a_baseline_root_value() {
    let spec = CommandSpec::new(
        "cargo",
        ["semver-checks", "--baseline-root"],
        CommandStdout::Quiet,
        CommandToolchainEnv::Inherit,
    );

    let error = semver_spec_with_materialized_baseline_for_tests(spec, Path::new("/tmp/baseline"))
        .expect_err("missing baseline value should fail");

    assert_eq!(
        error.to_string(),
        "semver gate command is missing the --baseline-root value"
    );
}
