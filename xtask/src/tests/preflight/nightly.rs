use super::*;

#[test]
fn nightly_toolchain_preflight_rejects_a_compiler_below_the_workspace_floor() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root");
    let expected_probe = nightly_toolchain_probe_command();

    crate::command_exec::with_capture_command_output_override(
        move |_repo_root, spec| {
            if command_signature(spec) == command_signature(&expected_probe) {
                return Some(Ok(b"rustc 1.97.0-nightly (hash 2026-05-11)\n".to_vec()));
            }
            if spec.program == Path::new("rustup")
                && spec
                    .args
                    .iter()
                    .map(String::as_str)
                    .eq(["toolchain", "list"])
            {
                return Some(Ok(b"nightly-aarch64-apple-darwin\n".to_vec()));
            }
            None
        },
        || {
            let error = ensure_miri_prerequisites(repo_root).expect_err("stale nightly");
            assert!(error.to_string().contains("rustup update nightly"));
            assert!(
                error
                    .to_string()
                    .contains("Required workspace floor: `1.98`")
            );
        },
    );
}

#[test]
fn nightly_toolchain_preflight_reports_missing_workspace_floor_toolchain_and_compiler() {
    let missing_floor_root = tempdir().expect("missing workspace manifest");
    crate::command_exec::with_capture_command_output_override(
        |_repo_root, spec| {
            (spec.program == Path::new("rustup")
                && spec
                    .args
                    .iter()
                    .map(String::as_str)
                    .eq(["toolchain", "list"]))
            .then(|| Ok(b"nightly-aarch64-apple-darwin\n".to_vec()))
        },
        || {
            let error = ensure_miri_prerequisites(missing_floor_root.path())
                .expect_err("missing workspace floor");
            assert!(
                error
                    .to_string()
                    .contains("could not read the workspace Rust floor")
            );
        },
    );

    let repo_root = tempdir().expect("workspace manifest");
    fs::write(
        repo_root.path().join("Cargo.toml"),
        "[workspace.package]\nrust-version = \"1.98\"\n",
    )
    .expect("write workspace manifest");
    crate::command_exec::with_capture_command_output_override(
        |_repo_root, spec| {
            (spec.program == Path::new("rustup")
                && spec
                    .args
                    .iter()
                    .map(String::as_str)
                    .eq(["toolchain", "list"]))
            .then(|| Ok(b"stable-aarch64-apple-darwin\n".to_vec()))
        },
        || {
            let error = ensure_miri_prerequisites(repo_root.path()).expect_err("missing nightly");
            assert!(
                error
                    .to_string()
                    .contains("rustup toolchain install nightly")
            );
        },
    );

    let expected_probe = nightly_toolchain_probe_command();
    crate::command_exec::with_capture_command_output_override(
        move |_repo_root, spec| {
            if spec.program == Path::new("rustup")
                && spec
                    .args
                    .iter()
                    .map(String::as_str)
                    .eq(["toolchain", "list"])
            {
                return Some(Ok(b"nightly-aarch64-apple-darwin\n".to_vec()));
            }
            (command_signature(spec) == command_signature(&expected_probe))
                .then(|| Err("nightly compiler unavailable".into()))
        },
        || {
            let error = ensure_miri_prerequisites(repo_root.path())
                .expect_err("unavailable nightly compiler");
            assert!(
                error
                    .to_string()
                    .contains("could not run the nightly compiler")
            );
        },
    );
}
