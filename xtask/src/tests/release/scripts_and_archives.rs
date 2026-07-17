use super::*;

#[test]
fn release_smoke_script_checks_the_canonical_version_and_real_extraction_flow() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root");
    let script = fs::read_to_string(repo_root.join("scripts").join("smoke-release-artifact.sh"))
        .expect("read smoke-release-artifact.sh");

    assert!(script.contains("grep \"^HTMLCut ${version}$\""));
    assert!(!script.contains("grep \"^htmlcut ${version}$\""));
    assert!(script.contains("--emit-request-file"));
    assert!(script.contains("packaged README.md leaked source-build instructions"));
    assert!(script.contains("request-file replay drifted"));
}

#[test]
fn release_build_script_generates_a_package_specific_readme() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root");
    let script = fs::read_to_string(repo_root.join("scripts").join("build-release-artifact.sh"))
        .expect("read build-release-artifact.sh");

    assert!(script.contains("write_packaged_readme"));
    assert!(script.contains("htmlcut_cargo_compiled_binary_path"));
    assert!(script.contains("package-specific install and verification guide"));
    assert!(!script.contains("sed '/^<!--$/,/^-->$/d' \"${repo_root}/README.md\""));
    assert!(
        !script.contains(
            "${repo_root}/target/${target_triple}/${cargo_profile}/${compiled_binary_name}"
        )
    );
}

#[test]
fn release_workflow_uses_immutable_tag_identity_for_all_publication_side_effects() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root");
    let workflow = fs::read_to_string(repo_root.join(".github/workflows/release.yml"))
        .expect("read release workflow");
    let checksum_script = fs::read_to_string(repo_root.join("scripts/build-release-checksums.sh"))
        .expect("read checksum script");
    let publish_script = fs::read_to_string(repo_root.join("scripts/publish-github-release.sh"))
        .expect("read publication script");
    let verify_script = fs::read_to_string(repo_root.join("scripts/verify-github-release.sh"))
        .expect("read verification script");

    assert!(workflow.contains("immutable tag manifest"));
    assert!(workflow.contains("RELEASE_TAG: ${{ steps.release.outputs.tag }}"));
    for script in [checksum_script, publish_script, verify_script] {
        assert!(script.contains("htmlcut_release_version_for_tag"));
        assert!(!script.contains("htmlcut_workspace_version \"${script_dir}\" \"${repo_root}\""));
    }
}

#[test]
fn maintained_release_shell_entrypoints_are_self_describing() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root");

    for (script_name, expected_fragment) in [
        (
            "build-release-artifact.sh",
            "Build one maintained standalone HTMLCut release artifact",
        ),
        (
            "build-release-checksums.sh",
            "Write the canonical SHA-256 checksum manifest",
        ),
        (
            "publish-github-release.sh",
            "Publish or converge the GitHub release object",
        ),
        (
            "release-tag.sh",
            "Validate one release tag against the tagged workspace manifest",
        ),
        (
            "release-targets.sh",
            "Inspect the canonical HTMLCut standalone release-target registry",
        ),
        (
            "smoke-release-artifact.sh",
            "Extract one maintained ./dist release archive",
        ),
        (
            "verify-github-release.sh",
            "Verify the published GitHub release object",
        ),
        (
            "workspace-version.sh",
            "Print the [workspace.package] version",
        ),
    ] {
        let output = bash_command()
            .arg(release_script_argument(repo_root, script_name))
            .arg("--help")
            .current_dir(repo_root)
            .output()
            .expect("run script --help");

        assert!(
            output.status.success(),
            "{script_name} --help failed:\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("Usage:"));
        assert!(
            stdout.contains(expected_fragment),
            "{script_name} help missing expected fragment {expected_fragment:?}:\n{stdout}",
        );
    }
}

#[test]
fn release_targets_cli_prints_canonical_registry_views() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root");

    let triples_output = bash_command()
        .arg(release_script_argument(repo_root, "release-targets.sh"))
        .arg("triples")
        .current_dir(repo_root)
        .output()
        .expect("run release-targets triples");
    assert!(triples_output.status.success());
    let triples = String::from_utf8(triples_output.stdout).expect("utf8 triples");
    assert!(triples.lines().any(|line| line == "aarch64-apple-darwin"));
    assert!(triples.lines().any(|line| line == "x86_64-pc-windows-msvc"));

    let matrix_output = bash_command()
        .arg(release_script_argument(repo_root, "release-targets.sh"))
        .arg("matrix-json")
        .current_dir(repo_root)
        .output()
        .expect("run release-targets matrix-json");
    assert!(matrix_output.status.success());
    let matrix: Value = serde_json::from_slice(&matrix_output.stdout).expect("parse matrix json");
    let include = matrix["include"].as_array().expect("matrix include");
    assert!(
        include
            .iter()
            .any(|entry| entry["target_triple"] == "aarch64-apple-darwin")
    );
    assert!(
        include
            .iter()
            .any(|entry| entry["target_triple"] == "x86_64-unknown-linux-musl")
    );

    let assets_output = bash_command()
        .arg(release_script_argument(repo_root, "release-targets.sh"))
        .args(["assets", "--version", "9.9.9"])
        .current_dir(repo_root)
        .output()
        .expect("run release-targets assets");
    assert!(assets_output.status.success());
    let assets = String::from_utf8(assets_output.stdout).expect("utf8 assets");
    assert!(
        assets
            .lines()
            .any(|line| line == "htmlcut-source-9.9.9.zip")
    );
    assert!(
        assets
            .lines()
            .any(|line| line == "htmlcut-9.9.9-x86_64-pc-windows-msvc.zip")
    );
    assert!(
        assets
            .lines()
            .any(|line| line == "htmlcut-9.9.9-checksums.txt")
    );
}

#[cfg(unix)]
#[test]
fn release_targets_script_is_shipped_as_an_executable_entrypoint() {
    use std::os::unix::fs::PermissionsExt;

    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root");
    let metadata = fs::metadata(repo_root.join("scripts").join("release-targets.sh"))
        .expect("release-targets metadata");

    assert_ne!(
        metadata.permissions().mode() & 0o111,
        0,
        "release-targets.sh should be directly runnable for local inspection",
    );
}
