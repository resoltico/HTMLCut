use super::*;
use std::fs;
use std::path::Path;
use std::process::Command;

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
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root");
    let scripts_dir = repo_root.join("scripts");
    let version = workspace_version(repo_root).expect("workspace version");
    let script = format!(
        r#"set -euo pipefail
script_dir="{scripts_dir}"
. "$script_dir/common.sh"
. "$script_dir/release-tag.sh"

script_dir="$(htmlcut_resolve_script_dir "$script_dir/release-tag.sh")"
readonly script_dir
repo_root="{repo_root}"
readonly repo_root
tag_name="v{version}"
readonly tag_name

resolved_root="$(htmlcut_repo_root_from_script_dir "$script_dir")"
[[ "$resolved_root" == "$repo_root" ]]

resolved_version="$(htmlcut_workspace_version "$script_dir" "$repo_root")"
[[ "$resolved_version" == "{version}" ]]

resolved_tag="$(htmlcut_resolve_release_tag "$tag_name")"
[[ "$resolved_tag" == "$tag_name" ]]

htmlcut_assert_release_tag_matches_workspace_version "$resolved_tag" "$resolved_version"
"#,
        scripts_dir = scripts_dir.display(),
        repo_root = repo_root.display(),
        version = version,
    );

    let output = Command::new("bash")
        .arg("-c")
        .arg(script)
        .current_dir(repo_root)
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

    let output = Command::new("bash")
        .arg("-c")
        .arg(format!(
            r#"set -euo pipefail
PATH="{fake_bin}:$PATH"
export OS=Windows_NT
export RUNNER_TEMP='D:\a\_temp'
source "{repo_root}/scripts/common.sh"
[[ "$(htmlcut_temp_root)" == "/d/a/_temp" ]]
"#,
            fake_bin = fake_bin.display(),
            repo_root = repo_root.display(),
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
fn windows_release_smoke_prefers_bash_native_unzip_before_powershell() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root");
    let temp = tempdir().expect("tempdir");
    let fake_bin = temp.path().join("bin");
    fs::create_dir_all(&fake_bin).expect("create fake bin");
    let log_path = temp.path().join("extractor.log");

    fs::write(
        fake_bin.join("unzip"),
        format!(
            "#!/usr/bin/env bash\nset -euo pipefail\nprintf 'unzip\\n' > \"{}\"\n",
            log_path.display()
        ),
    )
    .expect("write fake unzip");
    fs::write(
        fake_bin.join("powershell.exe"),
        format!(
            "#!/usr/bin/env bash\nset -euo pipefail\nprintf 'powershell\\n' > \"{}\"\n",
            log_path.display()
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

    let output = Command::new("bash")
        .arg("-c")
        .arg(format!(
            r#"set -euo pipefail
PATH="{fake_bin}:$PATH"
export OS=Windows_NT
source "{repo_root}/scripts/smoke-release-artifact.sh"
extract_release_archive "/tmp/archive.zip" "zip" "/tmp/extract-root"
"#,
            fake_bin = fake_bin.display(),
            repo_root = repo_root.display(),
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
fn release_smoke_script_checks_the_canonical_htmlcut_version_banner() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root");
    let script = fs::read_to_string(repo_root.join("scripts").join("smoke-release-artifact.sh"))
        .expect("read smoke-release-artifact.sh");

    assert!(script.contains("grep \"^HTMLCut ${version}$\""));
    assert!(!script.contains("grep \"^htmlcut ${version}$\""));
}
