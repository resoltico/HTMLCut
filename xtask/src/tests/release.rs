use super::*;
use std::fs;
use std::path::Path;
use std::process::Command;

use serde_json::Value;

fn bash_command() -> Command {
    Command::new(crate::release::bash_program_for_tests())
}

fn release_script_argument(repo_root: &Path, script_name: &str) -> String {
    crate::release::bash_source_argument_for_tests(&repo_root.join("scripts").join(script_name))
}

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

#[test]
fn stable_xtask_launcher_runs_a_temp_copy_outside_the_managed_target_root() {
    let actual_repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root");
    let repo_root = tempdir().expect("tempdir");
    let scripts_dir = repo_root.path().join("scripts");
    fs::create_dir_all(&scripts_dir).expect("create scripts dir");
    fs::create_dir_all(repo_root.path().join(".cargo")).expect("create .cargo dir");
    fs::write(
        repo_root.path().join(".cargo").join("config.toml"),
        "[build]\ntarget-dir = \"../.managed-artifacts/target\"\n",
    )
    .expect("write cargo config");
    fs::copy(
        actual_repo_root.join("scripts").join("common.sh"),
        scripts_dir.join("common.sh"),
    )
    .expect("copy common.sh");
    fs::copy(
        actual_repo_root.join("scripts").join("xtask.sh"),
        scripts_dir.join("xtask.sh"),
    )
    .expect("copy xtask.sh");

    let fake_bin = repo_root.path().join("fake-bin");
    fs::create_dir_all(&fake_bin).expect("create fake-bin");
    let log_path = repo_root.path().join("xtask-wrapper.log");
    let args_path = repo_root.path().join("cargo-build.log");
    let managed_target_root = fs::canonicalize(repo_root.path())
        .expect("canonicalize repo root")
        .join("../.managed-artifacts/target");
    let managed_target_root_for_bash =
        crate::release::bash_source_argument_for_tests(&managed_target_root);
    let log_path_for_bash = crate::release::bash_source_argument_for_tests(&log_path);
    let args_path_for_bash = crate::release::bash_source_argument_for_tests(&args_path);

    fs::write(
        fake_bin.join("cargo"),
        format!(
            r#"#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "$*" > "{args_path}"
[[ "$1" == "build" ]]
[[ "$2" == "-p" ]]
[[ "$3" == "xtask" ]]
[[ "$4" == "--locked" ]]
target_root="{managed_target_root}"
mkdir -p "${{target_root}}/debug"
cat > "${{target_root}}/debug/xtask.exe" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
[[ "$1" == "ci-rust-gate" ]]
case "$0" in
  "{managed_target_root}"/*) exit 42 ;;
esac
printf '%s\n' "$0" > "{log_path}"
EOF
chmod +x "${{target_root}}/debug/xtask.exe"
"#,
            args_path = args_path_for_bash,
            managed_target_root = managed_target_root_for_bash,
            log_path = log_path_for_bash,
        ),
    )
    .expect("write fake cargo");

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        fs::set_permissions(fake_bin.join("cargo"), fs::Permissions::from_mode(0o755))
            .expect("chmod fake cargo");
    }

    let repo_root_for_bash = crate::release::bash_source_argument_for_tests(repo_root.path());
    let fake_bin_for_bash = crate::release::bash_source_argument_for_tests(&fake_bin);
    let output = bash_command()
        .arg("-c")
        .arg(format!(
            r#"set -euo pipefail
export PATH="{fake_bin}:$PATH"
export OS=Windows_NT
export CARGO_TARGET_DIR="{managed_target_root}"
cd "{repo_root}"
bash ./scripts/xtask.sh ci-rust-gate
"#,
            fake_bin = fake_bin_for_bash,
            managed_target_root = managed_target_root_for_bash,
            repo_root = repo_root_for_bash,
        ))
        .current_dir(repo_root.path())
        .output()
        .expect("run stable xtask launcher smoke");

    assert!(
        output.status.success(),
        "stable xtask launcher smoke failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        fs::read_to_string(&args_path).expect("read cargo args"),
        "build -p xtask --locked\n"
    );
    let executed_path = fs::read_to_string(&log_path)
        .expect("read executed path")
        .trim()
        .to_owned();
    assert!(
        !executed_path.starts_with(&format!("{}/", managed_target_root_for_bash)),
        "stable xtask launcher must execute a temp copy outside the managed target root: {executed_path}"
    );
}

#[test]
fn release_shell_helpers_normalize_windows_source_paths() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root");
    let repo_root_for_bash = crate::release::bash_source_argument_for_tests(repo_root);

    let output = bash_command()
        .arg("-c")
        .arg(format!(
            r#"set -euo pipefail
source "{repo_root}/scripts/common.sh"
[[ "$(htmlcut_normalize_bash_path 'D:\a\HTMLCut\scripts\release-targets.sh')" == '/d/a/HTMLCut/scripts/release-targets.sh' ]]
[[ "$(htmlcut_normalize_bash_path 'scripts\release-targets.sh')" == 'scripts/release-targets.sh' ]]
"#,
            repo_root = repo_root_for_bash,
        ))
        .current_dir(repo_root)
        .output()
        .expect("run path-normalization smoke");

    assert!(
        output.status.success(),
        "path-normalization smoke failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn maintained_gate_surfaces_use_the_stable_xtask_launcher() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root");
    let root_check = fs::read_to_string(repo_root.join("check.sh")).expect("read check.sh");
    let workflow = fs::read_to_string(repo_root.join(".github").join("workflows").join("ci.yml"))
        .expect("read ci.yml");

    assert!(root_check.contains("./scripts/xtask.sh check"));
    assert!(workflow.contains("./scripts/xtask.sh ci-rust-gate"));
    assert!(workflow.contains("'scripts/xtask.sh'"));
    assert!(workflow.contains("..\\.htmlcut-artifacts"));
    assert!(workflow.contains("Join-Path $artifactRoot \"target\""));
    assert!(workflow.contains("Join-Path $artifactRoot \"build\""));
}

#[test]
fn windows_release_smoke_prefers_bash_native_unzip_before_powershell() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root");
    let temp = tempdir().expect("tempdir");
    let fake_bin = temp.path().join("bin");
    let fake_archive = temp.path().join("archive.zip");
    let fake_extract_root = temp.path().join("extract-root");
    fs::create_dir_all(&fake_bin).expect("create fake bin");
    let log_path = temp.path().join("extractor.log");
    let log_path_for_bash = crate::release::bash_source_argument_for_tests(&log_path);

    fs::write(
        fake_bin.join("unzip"),
        format!(
            "#!/usr/bin/env bash\nset -euo pipefail\nprintf 'unzip\\n' > \"{}\"\n",
            log_path_for_bash
        ),
    )
    .expect("write fake unzip");
    fs::write(
        fake_bin.join("powershell.exe"),
        format!(
            "#!/usr/bin/env bash\nset -euo pipefail\nprintf 'powershell\\n' > \"{}\"\n",
            log_path_for_bash
        ),
    )
    .expect("write fake powershell");

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let executable = fs::Permissions::from_mode(0o755);
        fs::set_permissions(fake_bin.join("unzip"), executable.clone()).expect("chmod fake unzip");
        fs::set_permissions(fake_bin.join("powershell.exe"), executable)
            .expect("chmod fake powershell");
    }

    let fake_bin_for_bash = crate::release::bash_source_argument_for_tests(&fake_bin);
    let repo_root_for_bash = crate::release::bash_source_argument_for_tests(repo_root);
    let fake_archive_for_bash = crate::release::bash_source_argument_for_tests(&fake_archive);
    let fake_extract_root_for_bash =
        crate::release::bash_source_argument_for_tests(&fake_extract_root);
    let output = bash_command()
        .arg("-c")
        .arg(format!(
            r#"set -euo pipefail
PATH="{fake_bin}:$PATH"
export OS=Windows_NT
source "{repo_root}/scripts/smoke-release-artifact.sh"
extract_release_archive "{fake_archive}" "zip" "{fake_extract_root}"
"#,
            fake_bin = fake_bin_for_bash,
            repo_root = repo_root_for_bash,
            fake_archive = fake_archive_for_bash,
            fake_extract_root = fake_extract_root_for_bash,
        ))
        .current_dir(repo_root)
        .output()
        .expect("run extractor selection smoke");

    assert!(
        output.status.success(),
        "extractor selection smoke failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        fs::read_to_string(&log_path).expect("read extractor log"),
        "unzip\n"
    );
}

#[test]
fn maintainer_protocol_paths_are_tracked_but_excluded_from_source_archives() {
    let repo_root = tempdir().expect("tempdir");
    fs::write(
        repo_root.path().join(".gitignore"),
        include_str!("../../../.gitignore"),
    )
    .expect("write gitignore");
    fs::write(
        repo_root.path().join(".gitattributes"),
        include_str!("../../../.gitattributes"),
    )
    .expect("write gitattributes");
    fs::write(repo_root.path().join("README.md"), "# Fixture\n").expect("write README");
    fs::write(repo_root.path().join("AGENTS.md"), "# Agent Entry\n").expect("write AGENTS");
    let codex_dir = repo_root.path().join(".codex");
    fs::create_dir_all(&codex_dir).expect("create .codex");
    fs::write(codex_dir.join("PROTOCOL_AFAD.md"), "# Protocol\n").expect("write protocol");

    init_git_repo(repo_root.path());
    assert_visible_as_untracked(repo_root.path(), &["AGENTS.md", ".codex/PROTOCOL_AFAD.md"]);
    git(repo_root.path(), &["add", "."]);
    git(repo_root.path(), &["commit", "-m", "fixture"]);

    let archive = git_output(repo_root.path(), &["archive", "--format=tar", "HEAD"]);
    let entries = tar_entry_names(&archive);

    assert!(entries.iter().any(|entry| entry == "README.md"));
    assert!(!entries.iter().any(|entry| entry == "AGENTS.md"));
    assert!(
        !entries
            .iter()
            .any(|entry| entry == ".codex/PROTOCOL_AFAD.md")
    );
}

fn init_git_repo(repo_root: &Path) {
    git(repo_root, &["init", "--initial-branch=main"]);
    git(repo_root, &["config", "user.name", "HTMLCut Tests"]);
    git(
        repo_root,
        &["config", "user.email", "htmlcut-tests@example.invalid"],
    );
}

fn assert_visible_as_untracked(repo_root: &Path, paths: &[&str]) {
    let output = git_output(repo_root, &["ls-files", "--others", "--exclude-standard"]);
    let listed_paths = String::from_utf8(output).expect("utf8 git ls-files output");
    for path in paths {
        assert!(
            listed_paths.lines().any(|listed| listed == *path),
            "expected {path} to stay untracked-and-visible, got:\n{listed_paths}",
        );
    }
}

fn git(repo_root: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(repo_root)
        .output()
        .expect("run git");
    assert!(
        output.status.success(),
        "git {:?} failed:\nstdout:\n{}\nstderr:\n{}",
        args,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn git_output(repo_root: &Path, args: &[&str]) -> Vec<u8> {
    let output = Command::new("git")
        .args(args)
        .current_dir(repo_root)
        .output()
        .expect("run git");
    assert!(
        output.status.success(),
        "git {:?} failed:\nstdout:\n{}\nstderr:\n{}",
        args,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    output.stdout
}

fn tar_entry_names(archive: &[u8]) -> Vec<String> {
    let mut entries = Vec::new();
    let mut offset = 0usize;

    while offset + 512 <= archive.len() {
        let header = &archive[offset..offset + 512];
        if header.iter().all(|byte| *byte == 0) {
            break;
        }

        let name = tar_header_field(&header[..100]);
        let prefix = tar_header_field(&header[345..500]);
        let entry = if prefix.is_empty() {
            name
        } else {
            format!("{prefix}/{name}")
        };
        entries.push(entry);

        let size = tar_octal_field(&header[124..136]);
        let payload_blocks = size.div_ceil(512);
        offset += 512 + (payload_blocks * 512);
    }

    entries
}

fn tar_header_field(field: &[u8]) -> String {
    let end = field
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(field.len());
    String::from_utf8_lossy(&field[..end]).trim().to_owned()
}

fn tar_octal_field(field: &[u8]) -> usize {
    let value = tar_header_field(field);
    usize::from_str_radix(value.trim(), 8).expect("tar size is valid octal")
}

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
