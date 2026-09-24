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
                return Some(Ok(b"nightly-2026-08-25-aarch64-apple-darwin\n".to_vec()));
            }
            None
        },
        || {
            let error = ensure_miri_prerequisites(repo_root).expect_err("stale nightly");
            assert!(
                error
                    .to_string()
                    .contains("rustup toolchain install nightly-2026-08-25")
            );
            assert!(
                error
                    .to_string()
                    .contains("Required workspace floor: `1.98.1`")
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
            .then(|| Ok(b"nightly-2026-08-25-aarch64-apple-darwin\n".to_vec()))
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
                    .contains("rustup toolchain install nightly-2026-08-25")
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
                return Some(Ok(b"nightly-2026-08-25-aarch64-apple-darwin\n".to_vec()));
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

#[test]
fn coverage_miri_and_fuzz_preflight_helpers_report_missing_prerequisites() {
    let missing_nightly = crate::preflight::coverage_preflight_error_for_tests("", "", |_| true)
        .expect("missing nightly");
    assert!(missing_nightly.contains("Install the nightly coverage toolchain"));

    let missing_clang = crate::preflight::coverage_preflight_error_for_tests(
        "nightly-2026-08-25-x86_64-unknown-linux-gnu\n",
        "llvm-tools-x86_64-unknown-linux-gnu (installed)\n",
        |_| false,
    )
    .expect("missing clang");
    assert!(missing_clang.contains("clang, clang++"));

    let missing_llvm_tools = crate::preflight::coverage_preflight_error_for_tests(
        "nightly-2026-08-25-x86_64-unknown-linux-gnu\n",
        "",
        |_| true,
    )
    .expect("missing llvm-tools");
    assert!(missing_llvm_tools.contains("llvm-tools-preview"));

    let missing_miri_toolchain = crate::preflight::miri_preflight_error_for_tests("", "", false)
        .expect("missing nightly Miri toolchain");
    assert!(missing_miri_toolchain.contains("Install the nightly Miri toolchain first"));

    let missing_miri_components = crate::preflight::miri_preflight_error_for_tests(
        "nightly-2026-08-25-x86_64-unknown-linux-gnu\n",
        "",
        false,
    )
    .expect("missing nightly Miri components");
    assert!(
        missing_miri_components
            .contains("rustup component add miri rust-src --toolchain nightly-2026-08-25")
    );

    let broken_miri = crate::preflight::miri_preflight_error_for_tests(
        "nightly-2026-08-25-x86_64-unknown-linux-gnu\n",
        "miri-x86_64-unknown-linux-gnu (installed)\nrust-src (installed)\n",
        false,
    )
    .expect("broken nightly Miri binary");
    assert!(broken_miri.contains("cargo +nightly-2026-08-25 miri --version"));

    let missing_cargo_fuzz = crate::preflight::fuzz_smoke_preflight_error_for_tests(
        "nightly-2026-08-25-x86_64-unknown-linux-gnu\n",
        false,
        |_| true,
    )
    .expect("missing cargo-fuzz");
    assert!(missing_cargo_fuzz.contains("install-contributor-cargo-tools.sh cargo-fuzz"));

    let missing_fuzz_clang = crate::preflight::fuzz_smoke_preflight_error_for_tests(
        "nightly-2026-08-25-x86_64-unknown-linux-gnu\n",
        true,
        |_| false,
    )
    .expect("missing clang");
    assert!(missing_fuzz_clang.contains("fuzz-smoke preflight failed"));

    let missing_fuzz_nightly =
        crate::preflight::fuzz_smoke_preflight_error_for_tests("", true, |_| true)
            .expect("missing nightly");
    assert!(missing_fuzz_nightly.contains("nightly-2026-08-25"));

    let clang_only =
        crate::preflight::clang_toolchain_preflight_error_for_tests("coverage", |_| false)
            .expect("clang tool message");
    assert!(clang_only.contains("coverage preflight failed"));
}
