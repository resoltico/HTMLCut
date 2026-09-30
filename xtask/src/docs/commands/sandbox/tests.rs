use super::*;

#[test]
fn fixture_seed_write_failure_and_missing_working_directory_are_reported() {
    let sandbox = ExampleSandbox::new().unwrap();
    let plan = sandbox.root.path().join("amount.plan.json");
    fs::remove_file(&plan).unwrap();
    fs::create_dir(&plan).unwrap();
    assert!(sandbox.seed().is_err());
    fs::remove_dir_all(sandbox.root.path()).unwrap();
    let tokens = vec!["htmlcut".into(), "describe".into()];
    let error = sandbox
        .command_runtime_error("fixture.md", "htmlcut describe", &tokens)
        .unwrap();
    assert!(error.contains("failed to capture CLI output"), "{error}");
}
