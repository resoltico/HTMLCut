use super::*;

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
[[ "$2" == "--quiet" ]]
[[ "$3" == "-p" ]]
[[ "$4" == "xtask" ]]
[[ "$5" == "--locked" ]]
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
        "build --quiet -p xtask --locked\n"
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
        include_str!("../../../../.gitignore"),
    )
    .expect("write gitignore");
    fs::write(
        repo_root.path().join(".gitattributes"),
        include_str!("../../../../.gitattributes"),
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
