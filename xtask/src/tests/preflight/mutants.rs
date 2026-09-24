//! Mutation-preflight scenarios separated from the shared preflight fixture matrix.

use super::*;

#[test]
fn mutation_preflight_accepts_an_available_tool_and_supported_nightly() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root");
    let mutant_probe = cargo_mutants_probe_command();
    let toolchain_list = test_command_spec("rustup", ["toolchain", "list"], false, false);
    let nightly_probe = nightly_toolchain_probe_command();

    crate::command_exec::with_capture_command_output_override(
        move |_repo_root, spec| {
            if command_signature(spec) == command_signature(&mutant_probe) {
                return Some(Ok(b"cargo-mutants 27.1.0\n".to_vec()));
            }
            if command_signature(spec) == command_signature(&toolchain_list) {
                return Some(Ok(b"nightly-aarch64-apple-darwin\n".to_vec()));
            }
            if command_signature(spec) == command_signature(&nightly_probe) {
                return Some(Ok(b"rustc 1.100.0-nightly (hash 2026-08-23)\n".to_vec()));
            }
            None
        },
        || ensure_mutants_prerequisites(repo_root).expect("mutation preflight"),
    );
}
