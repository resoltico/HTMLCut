use super::*;

#[test]
fn deny_check_command_denies_warnings() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root");

    assert_eq!(
        deny_check_command(repo_root).expect("deny command"),
        CommandSpec::new(
            "cargo",
            [
                "deny",
                "--target",
                "aarch64-apple-darwin",
                "--target",
                "x86_64-apple-darwin",
                "--target",
                "x86_64-unknown-linux-musl",
                "--target",
                "x86_64-pc-windows-msvc",
                "check",
                "-D",
                "warnings",
                "advisories",
                "bans",
                "licenses",
                "sources",
            ],
            false,
            false,
        )
    );
}

#[test]
fn parse_deny_graph_targets_supports_inline_and_multiline_lists() {
    assert_eq!(
        crate::policy::parse_deny_graph_targets_for_tests(
            r#"
[graph]
targets = ["aarch64-apple-darwin", "x86_64-pc-windows-msvc"]
"#
        ),
        Some(vec![
            "aarch64-apple-darwin".to_owned(),
            "x86_64-pc-windows-msvc".to_owned(),
        ])
    );
    assert_eq!(
        crate::policy::parse_deny_graph_targets_for_tests(
            r#"
[graph]
all-features = true
targets = [
    "aarch64-apple-darwin",
    "x86_64-apple-darwin",
]
"#
        ),
        Some(vec![
            "aarch64-apple-darwin".to_owned(),
            "x86_64-apple-darwin".to_owned(),
        ])
    );
}

#[test]
fn deny_graph_targets_reports_missing_graph_targets_and_malformed_quotes() {
    let repo_root = tempdir().expect("tempdir");
    fs::write(
        repo_root.path().join("deny.toml"),
        r#"
[graph]
all-features = true
"#,
    )
    .expect("write deny.toml");

    let error = deny_graph_targets(repo_root.path()).expect_err("missing targets should fail");
    assert!(
        error
            .to_string()
            .contains("deny.toml is missing [graph] targets")
    );

    assert_eq!(
        crate::policy::parse_deny_graph_targets_for_tests(
            r#"
[graph]
targets = ["aarch64-apple-darwin
[licenses]
allow = ["MIT"]
"#
        ),
        None
    );

    assert_eq!(
        crate::policy::parse_deny_graph_targets_for_tests(
            r#"
[graph
targets = ["aarch64-apple-darwin"]
"#
        ),
        None
    );
}

#[test]
fn deny_graph_targets_track_the_release_target_registry() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root");

    assert_eq!(
        deny_graph_targets(repo_root).expect("deny targets"),
        release_target_triples(repo_root).expect("release targets")
    );
}
