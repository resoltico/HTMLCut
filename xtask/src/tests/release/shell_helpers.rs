use super::*;

#[test]
fn release_helpers_read_the_canonical_shell_registry() {
    let repo_root = tempdir().expect("tempdir");
    let scripts_dir = repo_root.path().join("scripts");
    fs::create_dir_all(&scripts_dir).expect("create scripts dir");
    fs::write(
        scripts_dir.join("release-targets.sh"),
        r#"#!/usr/bin/env bash
release_target_triples() {
    cat <<'EOF'
aarch64-apple-darwin
x86_64-pc-windows-msvc
EOF
}

release_matrix_json() {
    cat <<'EOF'
{"include":[{"id":"macos-arm64","runs_on":"macos-15","target_triple":"aarch64-apple-darwin","artifact_bundle_name":"standalone-macos-arm64","needs_musl_tools":false}]}
EOF
}

release_asset_names_for_version() {
    local release_version="$1"
    printf 'htmlcut-source-%s.tar.gz\n' "${release_version}"
    printf 'htmlcut-%s-checksums.txt\n' "${release_version}"
}

macos_deployment_target_for_target() {
    local requested_target="$1"
    case "${requested_target}" in
        aarch64-apple-darwin) printf '12.0\n' ;;
        *) printf '\n' ;;
    esac
}

case "${1:-}" in
    triples)
        release_target_triples
        ;;
    matrix-json)
        release_matrix_json
        ;;
    assets)
        [[ "${2:-}" == "--version" ]] || exit 64
        release_asset_names_for_version "${3:-}"
        ;;
    macos-deployment-target)
        [[ "${2:-}" == "--target" ]] || exit 64
        macos_deployment_target_for_target "${3:-}"
        ;;
esac
"#,
    )
    .expect("write release-targets.sh");

    assert_eq!(
        release_target_triples(repo_root.path()).expect("target triples"),
        vec![
            "aarch64-apple-darwin".to_owned(),
            "x86_64-pc-windows-msvc".to_owned(),
        ]
    );
    assert_eq!(
        release_asset_names(repo_root.path(), "9.9.9").expect("asset names"),
        vec![
            "htmlcut-source-9.9.9.tar.gz".to_owned(),
            "htmlcut-9.9.9-checksums.txt".to_owned(),
        ]
    );
    assert_eq!(
        release_matrix(repo_root.path()).expect("release matrix"),
        vec![ReleaseMatrixEntry {
            id: "macos-arm64".to_owned(),
            runs_on: "macos-15".to_owned(),
            target_triple: "aarch64-apple-darwin".to_owned(),
            artifact_bundle_name: "standalone-macos-arm64".to_owned(),
            needs_musl_tools: false,
        }]
    );
    assert_eq!(
        macos_deployment_target(repo_root.path(), "aarch64-apple-darwin")
            .expect("macos deployment target"),
        Some("12.0".to_owned())
    );
    assert_eq!(
        macos_deployment_target(repo_root.path(), "x86_64-pc-windows-msvc")
            .expect("windows deployment target"),
        None
    );
}

#[test]
fn release_shell_helpers_survive_readonly_caller_names() {
    let actual_repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root");
    let repo_root = tempdir().expect("release-tag repository");
    fs::write(
        repo_root.path().join("Cargo.toml"),
        "[workspace.package]\nversion = \"9.8.7\"\n",
    )
    .expect("write tagged manifest");
    init_git_repo(repo_root.path());
    git(repo_root.path(), &["add", "Cargo.toml"]);
    git(repo_root.path(), &["commit", "-m", "release candidate"]);
    git(
        repo_root.path(),
        &["tag", "-a", "v9.8.7", "-m", "HTMLCut 9.8.7"],
    );
    fs::write(
        repo_root.path().join("Cargo.toml"),
        "[workspace.package]\nversion = \"9.9.9\"\n",
    )
    .expect("write newer main manifest");

    let scripts_dir = actual_repo_root.join("scripts");
    let scripts_dir = crate::release::bash_source_argument_for_tests(&scripts_dir);
    let helper_repo_root_for_bash =
        crate::release::bash_source_argument_for_tests(actual_repo_root);
    let repo_root_for_bash = crate::release::bash_source_argument_for_tests(repo_root.path());
    let script = format!(
        r#"set -euo pipefail
script_dir="{scripts_dir}"
. "$script_dir/common.sh"
. "$script_dir/release-tag.sh"

script_dir="$(htmlcut_resolve_script_dir "$script_dir/release-tag.sh")"
readonly script_dir
helper_repo_root="{helper_repo_root}"
readonly helper_repo_root
release_repo_root="{release_repo_root}"
readonly release_repo_root
tag_name="v9.8.7"
readonly tag_name

resolved_root="$(htmlcut_repo_root_from_script_dir "$script_dir")"
[[ "$resolved_root" == "$helper_repo_root" ]]

resolved_tag="$(htmlcut_resolve_release_tag "$tag_name")"
[[ "$resolved_tag" == "$tag_name" ]]

resolved_version="$(htmlcut_release_version_for_tag "$script_dir" "$release_repo_root" "$resolved_tag")"
[[ "$resolved_version" == "9.8.7" ]]

htmlcut_assert_release_tag_matches_workspace_version "$resolved_tag" "$resolved_version"
"#,
        scripts_dir = scripts_dir,
        helper_repo_root = helper_repo_root_for_bash,
        release_repo_root = repo_root_for_bash,
    );

    let output = bash_command()
        .arg("-c")
        .arg(script)
        .current_dir(repo_root.path())
        .output()
        .expect("run helper smoke");

    assert!(
        output.status.success(),
        "release shell helper smoke failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn release_shell_helpers_normalize_windows_temp_roots() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root");
    let temp = tempdir().expect("tempdir");
    let fake_bin = temp.path().join("bin");
    fs::create_dir_all(&fake_bin).expect("create fake bin");

    fs::write(
        fake_bin.join("cygpath"),
        "#!/usr/bin/env bash\nset -euo pipefail\n[[ \"$1\" == \"-u\" ]]\nprintf '/d/a/_temp\\n'\n",
    )
    .expect("write fake cygpath");

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        fs::set_permissions(fake_bin.join("cygpath"), fs::Permissions::from_mode(0o755))
            .expect("chmod fake cygpath");
    }

    let fake_bin_for_bash = crate::release::bash_source_argument_for_tests(&fake_bin);
    let repo_root_for_bash = crate::release::bash_source_argument_for_tests(repo_root);
    let output = bash_command()
        .arg("-c")
        .arg(format!(
            r#"set -euo pipefail
PATH="{fake_bin}:$PATH"
export OS=Windows_NT
export RUNNER_TEMP='D:\a\_temp'
source "{repo_root}/scripts/common.sh"
[[ "$(htmlcut_temp_root)" == "/d/a/_temp" ]]
"#,
            fake_bin = fake_bin_for_bash,
            repo_root = repo_root_for_bash,
        ))
        .current_dir(repo_root)
        .output()
        .expect("run temp-root helper smoke");

    assert!(
        output.status.success(),
        "temp-root helper smoke failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn release_shell_helpers_resolve_the_configured_cargo_target_dir() {
    let actual_repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root");
    let repo_root = tempdir().expect("tempdir");
    fs::create_dir_all(repo_root.path().join(".cargo")).expect("create .cargo dir");
    fs::create_dir_all(repo_root.path().join("src")).expect("create src dir");
    fs::write(
        repo_root.path().join("Cargo.toml"),
        "[package]\nname = \"htmlcut-release-helper-smoke\"\nversion = \"0.0.0\"\nedition = \"2024\"\n",
    )
    .expect("write cargo manifest");
    fs::write(
        repo_root.path().join("src").join("main.rs"),
        "fn main() {}\n",
    )
    .expect("write main.rs");
    fs::write(
        repo_root.path().join(".cargo").join("config.toml"),
        "[build]\ntarget-dir = \"../.managed-artifacts/target\"\n",
    )
    .expect("write cargo config");

    let helper_repo_root_for_bash =
        crate::release::bash_source_argument_for_tests(actual_repo_root);
    let repo_root_for_bash = crate::release::bash_source_argument_for_tests(repo_root.path());
    let canonical_repo_root = fs::canonicalize(repo_root.path()).expect("canonicalize temp repo");
    let output = bash_command()
        .arg("-c")
        .arg(format!(
            r#"set -euo pipefail
unset CARGO_TARGET_DIR
source "{helper_repo_root}/scripts/common.sh"
[[ "$(htmlcut_cargo_target_dir "{repo_root}")" == "{expected_target}" ]]
[[ "$(htmlcut_cargo_compiled_binary_path "{repo_root}" "aarch64-apple-darwin" "dist" "htmlcut")" == "{expected_binary}" ]]
"#,
            helper_repo_root = helper_repo_root_for_bash,
            repo_root = repo_root_for_bash,
            expected_target = crate::release::bash_source_argument_for_tests(
                &canonical_repo_root.join("../.managed-artifacts/target")
            ),
            expected_binary = crate::release::bash_source_argument_for_tests(
                &canonical_repo_root
                    .join("../.managed-artifacts/target")
                    .join("aarch64-apple-darwin")
                    .join("dist")
                    .join("htmlcut"),
            ),
        ))
        .current_dir(repo_root.path())
        .output()
        .expect("run cargo target helper smoke");

    assert!(
        output.status.success(),
        "cargo target helper smoke failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn release_shell_helpers_prefer_cargo_target_dir_environment_overrides() {
    let actual_repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root");
    let repo_root = tempdir().expect("tempdir");
    fs::create_dir_all(repo_root.path().join(".cargo")).expect("create .cargo dir");
    fs::create_dir_all(repo_root.path().join("src")).expect("create src dir");
    fs::write(
        repo_root.path().join("Cargo.toml"),
        "[package]\nname = \"htmlcut-release-helper-smoke\"\nversion = \"0.0.0\"\nedition = \"2024\"\n",
    )
    .expect("write cargo manifest");
    fs::write(
        repo_root.path().join("src").join("main.rs"),
        "fn main() {}\n",
    )
    .expect("write main.rs");
    fs::write(
        repo_root.path().join(".cargo").join("config.toml"),
        "[build]\ntarget-dir = \"../.managed-artifacts/target\"\n",
    )
    .expect("write cargo config");

    let helper_repo_root_for_bash =
        crate::release::bash_source_argument_for_tests(actual_repo_root);
    let repo_root_for_bash = crate::release::bash_source_argument_for_tests(repo_root.path());
    let output = bash_command()
        .arg("-c")
        .arg(format!(
            r#"set -euo pipefail
export CARGO_TARGET_DIR="./tmp/override-target"
source "{helper_repo_root}/scripts/common.sh"
[[ "$(htmlcut_cargo_target_dir "{repo_root}")" == "{expected_target}" ]]
"#,
            helper_repo_root = helper_repo_root_for_bash,
            repo_root = repo_root_for_bash,
            expected_target = crate::release::bash_source_argument_for_tests(
                &repo_root.path().join("./tmp/override-target")
            ),
        ))
        .current_dir(repo_root.path())
        .output()
        .expect("run cargo target env-override smoke");

    assert!(
        output.status.success(),
        "cargo target env-override smoke failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn release_shell_helpers_resolve_host_binary_paths_for_unix_and_windows() {
    let actual_repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root");
    let repo_root = tempdir().expect("tempdir");
    fs::create_dir_all(repo_root.path().join(".cargo")).expect("create .cargo dir");
    fs::create_dir_all(repo_root.path().join("src")).expect("create src dir");
    fs::write(
        repo_root.path().join("Cargo.toml"),
        "[package]\nname = \"htmlcut-release-helper-smoke\"\nversion = \"0.0.0\"\nedition = \"2024\"\n",
    )
    .expect("write cargo manifest");
    fs::write(
        repo_root.path().join("src").join("main.rs"),
        "fn main() {}\n",
    )
    .expect("write main.rs");
    fs::write(
        repo_root.path().join(".cargo").join("config.toml"),
        "[build]\ntarget-dir = \"../.managed-artifacts/target\"\n",
    )
    .expect("write cargo config");

    let helper_repo_root_for_bash =
        crate::release::bash_source_argument_for_tests(actual_repo_root);
    let repo_root_for_bash = crate::release::bash_source_argument_for_tests(repo_root.path());
    let canonical_repo_root = fs::canonicalize(repo_root.path()).expect("canonicalize temp repo");
    let unix_binary = canonical_repo_root
        .join("../.managed-artifacts/target")
        .join("debug")
        .join("xtask");
    let windows_binary = canonical_repo_root
        .join("../.managed-artifacts/target")
        .join("debug")
        .join("xtask.exe");
    let output = bash_command()
        .arg("-c")
        .arg(format!(
            r#"set -euo pipefail
source "{helper_repo_root}/scripts/common.sh"
unset CARGO_TARGET_DIR
unset OS
[[ "$(htmlcut_cargo_host_binary_path "{repo_root}" "debug" "xtask")" == "{expected_unix}" ]]
export OS=Windows_NT
[[ "$(htmlcut_cargo_host_binary_path "{repo_root}" "debug" "xtask")" == "{expected_windows}" ]]
"#,
            helper_repo_root = helper_repo_root_for_bash,
            repo_root = repo_root_for_bash,
            expected_unix = crate::release::bash_source_argument_for_tests(&unix_binary),
            expected_windows = crate::release::bash_source_argument_for_tests(&windows_binary),
        ))
        .current_dir(repo_root.path())
        .output()
        .expect("run host-binary helper smoke");

    assert!(
        output.status.success(),
        "host-binary helper smoke failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
