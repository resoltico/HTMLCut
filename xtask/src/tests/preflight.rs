use std::collections::BTreeMap;

use super::*;

#[test]
fn repo_toolchain_preflight_error_covers_missing_toolchain_and_broken_component() {
    let toolchain = RepoToolchain {
        channel: "1.95.0".to_owned(),
        components: vec!["clippy".to_owned(), "rustfmt".to_owned()],
    };

    let missing_toolchain = crate::preflight::repo_toolchain_preflight_error_for_tests(
        &toolchain,
        "stable-x86_64-unknown-linux-gnu\n",
        "",
        |_| true,
    )
    .expect("missing toolchain message");
    assert!(missing_toolchain.contains("Install the pinned toolchain first"));

    let broken_component = crate::preflight::repo_toolchain_preflight_error_for_tests(
        &toolchain,
        "1.95.0-x86_64-unknown-linux-gnu\n",
        "clippy-x86_64-unknown-linux-gnu (installed)\nrustfmt-x86_64-unknown-linux-gnu (installed)\n",
        |spec| !spec.args.contains(&"cargo-clippy".to_owned()),
    )
    .expect("broken component message");
    assert!(broken_component.contains("still do not run: clippy"));
}

#[test]
fn coverage_and_fuzz_preflight_helpers_report_missing_prerequisites() {
    let missing_nightly = crate::preflight::coverage_preflight_error_for_tests("", "", |_| true)
        .expect("missing nightly");
    assert!(missing_nightly.contains("Install the nightly coverage toolchain"));

    let missing_clang = crate::preflight::coverage_preflight_error_for_tests(
        "nightly-x86_64-unknown-linux-gnu\n",
        "llvm-tools-x86_64-unknown-linux-gnu (installed)\n",
        |_| false,
    )
    .expect("missing clang");
    assert!(missing_clang.contains("clang, clang++"));

    let missing_llvm_tools = crate::preflight::coverage_preflight_error_for_tests(
        "nightly-x86_64-unknown-linux-gnu\n",
        "",
        |_| true,
    )
    .expect("missing llvm-tools");
    assert!(missing_llvm_tools.contains("llvm-tools-preview"));

    let missing_cargo_fuzz = crate::preflight::fuzz_smoke_preflight_error_for_tests(
        "nightly-x86_64-unknown-linux-gnu\n",
        false,
        |_| true,
    )
    .expect("missing cargo-fuzz");
    assert!(missing_cargo_fuzz.contains("cargo install cargo-fuzz --locked"));

    let missing_fuzz_clang = crate::preflight::fuzz_smoke_preflight_error_for_tests(
        "nightly-x86_64-unknown-linux-gnu\n",
        true,
        |_| false,
    )
    .expect("missing clang");
    assert!(missing_fuzz_clang.contains("fuzz-smoke preflight failed"));

    let missing_fuzz_nightly =
        crate::preflight::fuzz_smoke_preflight_error_for_tests("", true, |_| true)
            .expect("missing nightly");
    assert!(missing_fuzz_nightly.contains("nightly"));

    let clang_only =
        crate::preflight::clang_toolchain_preflight_error_for_tests("coverage", |_| false)
            .expect("clang tool message");
    assert!(clang_only.contains("coverage preflight failed"));
}

#[test]
fn public_preflight_wrappers_use_the_capture_override_surface() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root");
    let toolchain = repo_toolchain(repo_root).expect("repo toolchain");
    let toolchain_list = format!(
        "{}-x86_64-apple-darwin\nnightly-x86_64-apple-darwin\n",
        toolchain.channel
    );
    let component_list = toolchain
        .components
        .iter()
        .map(|component| format!("{component}-x86_64-apple-darwin (installed)\n"))
        .collect::<String>();

    crate::command_exec::with_capture_command_output_override(
        capture_override_fixture(
            toolchain.clone(),
            toolchain_list.clone(),
            component_list.clone(),
        ),
        || {
            ensure_repo_toolchain_prerequisites(repo_root).expect("repo toolchain preflight");
            ensure_coverage_prerequisites(repo_root).expect("coverage preflight");
            ensure_fuzz_smoke_prerequisites(repo_root).expect("fuzz preflight");
        },
    );

    crate::command_exec::with_capture_command_output_override(
        capture_override_fixture(toolchain, String::new(), component_list),
        || {
            let error =
                ensure_repo_toolchain_prerequisites(repo_root).expect_err("repo toolchain failure");
            assert!(
                error
                    .to_string()
                    .contains("Install the pinned toolchain first")
            );
        },
    );
}

#[test]
fn public_preflight_wrappers_report_missing_manifests_and_command_failures() {
    let missing_repo = tempdir().expect("tempdir");
    let missing_manifest = ensure_repo_toolchain_prerequisites(missing_repo.path())
        .expect_err("missing rust-toolchain should fail");
    assert!(
        missing_manifest
            .to_string()
            .contains("toolchain preflight could not read rust-toolchain.toml")
    );

    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root");
    let toolchain = repo_toolchain(repo_root).expect("repo toolchain");

    crate::command_exec::with_capture_command_output_override(
        move |_repo_root, spec| {
            (command_signature(spec)
                == command_signature(&CommandSpec::new(
                    "rustup",
                    ["toolchain", "list"],
                    false,
                    false,
                )))
            .then(|| Err("boom".into()))
        },
        || {
            let error =
                ensure_repo_toolchain_prerequisites(repo_root).expect_err("toolchain list failure");
            assert!(
                error
                    .to_string()
                    .contains("toolchain preflight could not query rustup toolchains")
            );
        },
    );

    let toolchain_list = format!("{}-x86_64-apple-darwin\n", toolchain.channel);
    crate::command_exec::with_capture_command_output_override(
        move |_repo_root, spec| {
            if command_signature(spec)
                == command_signature(&CommandSpec::new(
                    "rustup",
                    ["toolchain", "list"],
                    false,
                    false,
                ))
            {
                return Some(Ok(toolchain_list.as_bytes().to_vec()));
            }
            (command_signature(spec)
                == command_signature(&CommandSpec::new(
                    "rustup",
                    [
                        "component",
                        "list",
                        "--toolchain",
                        toolchain.channel.as_str(),
                        "--installed",
                    ],
                    false,
                    false,
                )))
            .then(|| Ok(vec![0xFF]))
        },
        || {
            let error = ensure_repo_toolchain_prerequisites(repo_root)
                .expect_err("component decode failure");
            assert!(
                error
                    .to_string()
                    .contains("toolchain preflight received invalid component output")
            );
        },
    );

    crate::command_exec::with_capture_command_output_override(
        move |_repo_root, spec| {
            if command_signature(spec)
                == command_signature(&CommandSpec::new(
                    "rustup",
                    ["toolchain", "list"],
                    false,
                    false,
                ))
            {
                return Some(Ok(b"nightly-x86_64-apple-darwin\n".to_vec()));
            }
            if command_signature(spec)
                == command_signature(&CommandSpec::new(
                    "rustup",
                    ["component", "list", "--toolchain", "nightly", "--installed"],
                    false,
                    false,
                ))
            {
                return Some(Ok(Vec::new()));
            }
            None
        },
        || {
            let error = ensure_coverage_prerequisites(repo_root)
                .expect_err("coverage should fail without llvm-tools");
            assert!(error.to_string().contains("llvm-tools-preview"));
        },
    );

    crate::command_exec::with_capture_command_output_override(
        move |_repo_root, spec| {
            (command_signature(spec)
                == command_signature(&CommandSpec::new(
                    "rustup",
                    ["toolchain", "list"],
                    false,
                    false,
                )))
            .then(|| Err("missing nightly".into()))
        },
        || {
            let error = ensure_coverage_prerequisites(repo_root)
                .expect_err("coverage toolchain query failure");
            assert!(
                error
                    .to_string()
                    .contains("coverage preflight could not query rustup toolchains")
            );
        },
    );

    crate::command_exec::with_capture_command_output_override(
        move |_repo_root, spec| {
            if command_signature(spec)
                == command_signature(&CommandSpec::new(
                    "rustup",
                    ["toolchain", "list"],
                    false,
                    false,
                ))
            {
                return Some(Ok(b"nightly-x86_64-apple-darwin\n".to_vec()));
            }
            (command_signature(spec)
                == command_signature(&CommandSpec::new(
                    "rustup",
                    ["component", "list", "--toolchain", "nightly", "--installed"],
                    false,
                    false,
                )))
            .then(|| Ok(vec![0xFF]))
        },
        || {
            let error = ensure_coverage_prerequisites(repo_root)
                .expect_err("coverage component decode failure");
            assert!(
                error
                    .to_string()
                    .contains("coverage preflight received invalid component output")
            );
        },
    );

    crate::command_exec::with_capture_command_output_override(
        move |_repo_root, spec| {
            if command_signature(spec)
                == command_signature(&CommandSpec::new(
                    "rustup",
                    ["toolchain", "list"],
                    false,
                    false,
                ))
            {
                return Some(Ok(b"nightly-x86_64-apple-darwin\n".to_vec()));
            }
            (command_signature(spec) == command_signature(&cargo_fuzz_probe_command()))
                .then(|| Err("missing cargo-fuzz".into()))
        },
        || {
            let error = ensure_fuzz_smoke_prerequisites(repo_root)
                .expect_err("fuzz smoke should fail without cargo-fuzz");
            assert!(
                error
                    .to_string()
                    .contains("cargo install cargo-fuzz --locked")
            );
        },
    );

    crate::command_exec::with_capture_command_output_override(
        move |_repo_root, spec| {
            (command_signature(spec)
                == command_signature(&CommandSpec::new(
                    "rustup",
                    ["toolchain", "list"],
                    false,
                    false,
                )))
            .then(|| Ok(vec![0xFF]))
        },
        || {
            let error = ensure_fuzz_smoke_prerequisites(repo_root)
                .expect_err("fuzz toolchain decode failure");
            assert!(
                error
                    .to_string()
                    .contains("fuzz-smoke preflight received invalid rustup output")
            );
        },
    );
}

#[test]
fn repo_toolchain_preflight_error_reports_missing_components() {
    let toolchain = RepoToolchain {
        channel: "1.95.0".to_owned(),
        components: vec!["clippy".to_owned(), "rustfmt".to_owned()],
    };

    let message = crate::preflight::repo_toolchain_preflight_error_for_tests(
        &toolchain,
        "1.95.0-x86_64-unknown-linux-gnu\n",
        "clippy-x86_64-unknown-linux-gnu (installed)\n",
        |_| true,
    )
    .expect("missing component message");
    assert!(message.contains("Install the missing pinned-toolchain components"));
}

fn capture_override_fixture(
    toolchain: RepoToolchain,
    toolchain_list: String,
    component_list: String,
) -> impl FnMut(&Path, &CommandSpec) -> Option<DynResult<Vec<u8>>> {
    let mut outputs: BTreeMap<(String, Vec<String>), Result<Vec<u8>, String>> = BTreeMap::new();
    outputs.insert(
        command_signature(&CommandSpec::new(
            "rustup",
            ["toolchain", "list"],
            false,
            false,
        )),
        Ok(toolchain_list.into_bytes()),
    );
    outputs.insert(
        command_signature(&CommandSpec::new(
            "rustup",
            [
                "component",
                "list",
                "--toolchain",
                toolchain.channel.as_str(),
                "--installed",
            ],
            false,
            false,
        )),
        Ok(component_list.into_bytes()),
    );
    outputs.insert(
        command_signature(&CommandSpec::new(
            "rustup",
            ["component", "list", "--toolchain", "nightly", "--installed"],
            false,
            false,
        )),
        Ok(b"llvm-tools-x86_64-unknown-linux-gnu (installed)\n".to_vec()),
    );
    outputs.insert(
        command_signature(&cargo_fuzz_probe_command()),
        Ok(b"cargo-fuzz 0.13.1\n".to_vec()),
    );
    outputs.insert(
        command_signature(&host_tool_probe_command("clang")),
        Ok(b"clang version 20.0.0\n".to_vec()),
    );
    outputs.insert(
        command_signature(&host_tool_probe_command("clang++")),
        Ok(b"clang version 20.0.0\n".to_vec()),
    );

    move |_repo_root, spec| {
        outputs
            .get(&command_signature(spec))
            .map(|result| match result {
                Ok(bytes) => Ok(bytes.clone()),
                Err(message) => Err(message.clone().into()),
            })
    }
}

fn command_signature(spec: &CommandSpec) -> (String, Vec<String>) {
    (
        spec.program.to_string_lossy().into_owned(),
        spec.args.clone(),
    )
}
