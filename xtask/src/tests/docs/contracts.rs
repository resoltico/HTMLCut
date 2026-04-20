use super::*;

#[test]
fn docs_contract_lint_accepts_current_repo_style_docs() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("repo root");

    let errors = markdown_contract_errors(repo_root).expect("markdown contract errors");

    assert!(
        errors.is_empty(),
        "unexpected docs contract errors: {errors:#?}"
    );
}
