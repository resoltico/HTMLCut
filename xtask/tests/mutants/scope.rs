// SPDX-License-Identifier: MPL-2.0
//! Exact mutation inventory and drift rejection.
use super::*;

#[test]
fn mutation_scope_verifier_tracks_cargo_default_members_and_rejects_drift() {
    let root = tempdir().expect("scope verifier fixture");
    let valid_path = root.path().join("valid-mutants.json");
    let invalid_path = root.path().join("invalid-mutants.json");
    let mut members = default_runtime_members();
    members.extend(mutation_tooling_members());
    let mut mutants = members
        .iter()
        .map(|(package, member_path)| {
            json!({
                "package": package,
                "file": format!("{member_path}/src/lib.rs"),
            })
        })
        .collect::<Vec<_>>();
    mutants.extend([
        json!({"package": "htmlcut-servo-arc", "file": "patches/rust/servo_arc/tagged_union.rs"}),
        json!({"package": "htmlcut-selectors", "file": "patches/rust/selectors/relative_selector/filter.rs"}),
        json!({"package": "htmlcut-selectors", "file": "patches/rust/selectors/matching.rs"}),
        json!({"package": "htmlcut-selectors", "file": "patches/rust/selectors/context.rs"}),

        json!({"package": "htmlcut-selectors", "file": "patches/rust/selectors/work_budget.rs"}),
        json!({"package": "htmlcut-scraper", "file": "patches/rust/scraper/src/html/clone.rs"}),
        json!({"package": "htmlcut-scraper", "file": "patches/rust/scraper/src/selector/budget.rs"}),
        json!({"package": "htmlcut-scraper", "file": "patches/rust/scraper/src/html/bounded.rs"}),
        json!({"package": "htmlcut-scraper", "file": "patches/rust/scraper/src/element_ref/filtered.rs"}),
    ]);
    fs::write(
        &valid_path,
        serde_json::to_vec(&mutants).expect("serialize valid mutation fixture"),
    )
    .expect("write valid mutation fixture");
    let mut invalid_mutants = mutants;
    invalid_mutants.pop();
    fs::write(
        &invalid_path,
        serde_json::to_vec(&invalid_mutants).expect("serialize invalid mutation fixture"),
    )
    .expect("write invalid mutation fixture");

    for (path, expected_success) in [(valid_path, true), (invalid_path, false)] {
        let output = Command::new("bash")
            .arg(repo_root().join("scripts").join("verify-mutation-scope.sh"))
            .arg(&path)
            .output()
            .expect("run mutation scope verifier");
        assert_eq!(
            output.status.success(),
            expected_success,
            "scope verifier stderr:\n{}\nfixture:\n{}",
            String::from_utf8_lossy(&output.stderr),
            // The fixture is included only when this regression fails, to make metadata/path drift actionable.
            fs::read_to_string(path).expect("read scope verifier fixture")
        );
    }
}
