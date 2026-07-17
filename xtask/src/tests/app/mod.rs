use super::*;
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

fn write_toolchain_contract(repo_root: &Path) {
    fs::write(
        repo_root.join("rust-toolchain.toml"),
        "[toolchain]\nchannel = \"stable\"\ncomponents = [\"clippy\", \"rustfmt\"]\n",
    )
    .expect("write rust-toolchain.toml");
}

fn write_tracked_source(repo_root: &Path, relative_path: &str) -> PathBuf {
    let path = repo_root.join(relative_path);
    fs::create_dir_all(path.parent().expect("tracked source parent")).expect("create src dir");
    fs::write(&path, "pub fn covered() {}\n").expect("write tracked source");
    path
}

fn write_coverage_report(
    repo_root: &Path,
    tracked_file: &Path,
    line_count: u64,
    branch_count: u64,
    covered_branches: u64,
    uncovered_branches: u64,
) {
    let coverage_path = coverage_output_path(repo_root);
    fs::create_dir_all(coverage_path.parent().expect("coverage dir")).expect("create coverage dir");
    fs::write(
        coverage_path,
        serde_json::json!({
            "data": [{
                "files": [{
                    "filename": tracked_file,
                    "segments": [[1, 0, line_count, false, true, false]],
                    "branches": [],
                    "summary": {
                        "branches": {
                            "count": branch_count,
                            "covered": covered_branches,
                            "notcovered": uncovered_branches,
                        }
                    }
                }]
            }]
        })
        .to_string(),
    )
    .expect("write coverage report");
}

fn write_outdated_fixture_repo(repo_root: &Path) {
    fs::create_dir_all(repo_root.join("crates").join("htmlcut-core")).expect("create core dir");
    fs::create_dir_all(repo_root.join("xtask")).expect("create xtask dir");
    fs::create_dir_all(repo_root.join("crates").join("htmlcut-core").join("src"))
        .expect("create core src dir");
    fs::create_dir_all(repo_root.join("xtask").join("src")).expect("create xtask src dir");
    fs::write(
        repo_root.join("Cargo.toml"),
        r#"[workspace]
members = ["crates/htmlcut-core", "xtask"]
resolver = "3"

[workspace.package]
version = "10.1.0"

[workspace.dependencies]
scraper = { package = "htmlcut-scraper", path = "patches/rust/scraper", version = "0.27.0-htmlcut.1", default-features = false, features = ["errors"] }
"#,
    )
    .expect("write root Cargo.toml");
    fs::write(
        repo_root
            .join("crates")
            .join("htmlcut-core")
            .join("Cargo.toml"),
        "[package]\nname = \"htmlcut-core\"\nversion = \"10.0.0\"\nedition = \"2024\"\n",
    )
    .expect("write htmlcut-core Cargo.toml");
    fs::write(
        repo_root.join("xtask").join("Cargo.toml"),
        "[package]\nname = \"xtask\"\nversion = \"10.0.0\"\nedition = \"2024\"\n",
    )
    .expect("write xtask Cargo.toml");
    fs::write(
        repo_root
            .join("crates")
            .join("htmlcut-core")
            .join("src")
            .join("lib.rs"),
        "pub fn placeholder() {}\n",
    )
    .expect("write htmlcut-core lib.rs");
    fs::write(
        repo_root.join("xtask").join("src").join("main.rs"),
        "fn main() {}\n",
    )
    .expect("write xtask main.rs");
}

fn with_ready_preflight<T>(operation: impl FnOnce() -> T) -> T {
    crate::command_exec::with_capture_command_output_override(
        |_, spec| {
            let args = spec.args.iter().map(String::as_str).collect::<Vec<_>>();
            if spec.program == Path::new("rustup") && args == ["toolchain", "list"] {
                return Some(Ok(
                    b"stable-aarch64-apple-darwin (default)\nnightly-aarch64-apple-darwin\n"
                        .to_vec(),
                ));
            }
            if spec.program == Path::new("rustup") && args == ["run", "stable", "rustc", "-Vv"] {
                return Some(Ok(b"rustc 1.97.0\n".to_vec()));
            }
            if spec.program == Path::new("rustup")
                && args == ["component", "list", "--toolchain", "stable", "--installed"]
            {
                return Some(Ok(
                    b"clippy-aarch64-apple-darwin\nrustfmt-aarch64-apple-darwin\n".to_vec(),
                ));
            }
            if spec.program == Path::new("rustup")
                && args == ["component", "list", "--toolchain", "nightly", "--installed"]
            {
                return Some(Ok(
                    b"llvm-tools-preview-aarch64-apple-darwin\nmiri-aarch64-apple-darwin\nrust-src\n"
                        .to_vec(),
                ));
            }
            if spec.program == Path::new("rustup")
                && args == ["run", "stable", "cargo-clippy", "-V"]
            {
                return Some(Ok(b"clippy 0.1.0\n".to_vec()));
            }
            if spec.program == Path::new("rustup")
                && args == ["run", "stable", "rustfmt", "--version"]
            {
                return Some(Ok(b"rustfmt 1.0.0\n".to_vec()));
            }
            if spec.program == Path::new("cargo") && args == ["+nightly", "miri", "--version"] {
                return Some(Ok(b"miri 0.1.0\n".to_vec()));
            }
            if spec.program == Path::new("cargo") && args == ["fuzz", "--help"] {
                return Some(Ok(b"cargo-fuzz 0.12.0\n".to_vec()));
            }
            if (spec.program == Path::new("clang") || spec.program == Path::new("clang++"))
                && args == ["--version"]
            {
                return Some(Ok(b"clang version 18.0.0\n".to_vec()));
            }

            None
        },
        operation,
    )
}

fn with_isolated_target_dir<T>(repo_root: &Path, operation: impl FnOnce() -> T) -> T {
    let cargo_config_dir = repo_root.join(".cargo");
    fs::create_dir_all(&cargo_config_dir).expect("create .cargo dir");
    fs::write(
        cargo_config_dir.join("config.toml"),
        "[build]\ntarget-dir = \".htmlcut-artifacts/target\"\nbuild-dir = \".htmlcut-artifacts/build\"\n",
    )
    .expect("write managed test cargo config");
    crate::plan::with_cargo_artifact_dir_overrides_for_tests(
        repo_root.join(".htmlcut-artifacts/target"),
        repo_root.join(".htmlcut-artifacts/build"),
        operation,
    )
}

fn command_env_value<'a>(spec: &'a CommandSpec, key: &str) -> &'a str {
    spec.env
        .get(key)
        .map(String::as_str)
        .expect("command env value")
}

mod check_flow;
mod maintenance_commands;
mod semver_refresh;
mod structure;
