use super::*;

#[test]
fn poisoned_cwd_lock_is_reported_without_poisoning_the_process_global_lock() {
    let lock = Mutex::new(());
    std::thread::scope(|scope| {
        assert!(
            scope
                .spawn(|| {
                    let _guard = lock.lock().unwrap();
                    panic!("test-owned lock poison");
                })
                .join()
                .is_err()
        );
    });
    assert!(
        acquire_cwd_lock(&lock)
            .unwrap_err()
            .to_string()
            .contains("cwd mutex poisoned")
    );
}

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
