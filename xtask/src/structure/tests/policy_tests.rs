use std::collections::BTreeSet;

use super::super::metrics::Metrics;
use super::super::policy::Policy;
use super::RULE;

#[test]
fn policy_rejects_invalid_versions_metadata_and_duplicate_rules() {
    for source in [
        RULE.replacen("version = 1", "version = 2", 1),
        "version = 1\n".to_owned(),
        RULE.replacen("path = \"crates/htmlcut-core/src/\"", "path = \"\"", 1),
        RULE.replacen("role = \"test role\"", "role = \"\"", 1),
        RULE.replacen("owner = \"test owner\"", "owner = \"\"", 1),
        RULE.replacen("rationale = \"test rationale\"", "rationale = \"\"", 1),
        RULE.replacen(
            "split_trigger = \"test split trigger\"",
            "split_trigger = \"\"",
            1,
        ),
        format!("{RULE}\n{RULE}"),
    ] {
        assert!(Policy::parse(&source).is_err());
    }
}

#[test]
fn policy_rejects_duplicate_exact_rules_with_an_exactly_classified_error() {
    let exact_rule = RULE
        .replacen(
            "path = \"crates/htmlcut-core/src/\"",
            "path = \"crates/htmlcut-core/src/model.rs\"",
            1,
        )
        .replacen("match = \"prefix\"", "match = \"exact\"", 1);
    let error = Policy::parse(&format!("{exact_rule}\n{exact_rule}"))
        .expect_err("duplicate exact rules must be rejected");
    assert!(
        error
            .to_string()
            .contains("duplicate Rust source-shape rule for exact")
    );
}

#[test]
fn policy_rejects_invalid_review_expirations_and_reports_expired_reviews() {
    let invalid = RULE.replacen(
        "max_physical_lines = 20",
        "review_expires_on = \"not-a-date\"\nmax_physical_lines = 20",
        1,
    );
    assert!(Policy::parse(&invalid).is_err());

    let expired = Policy::parse(&RULE.replacen(
        "max_physical_lines = 20",
        "review_expires_on = \"2000-01-01\"\nmax_physical_lines = 20",
        1,
    ))
    .expect("policy");
    assert!(
        expired
            .expired_rule_findings()
            .expect("expiry findings")
            .iter()
            .any(|finding| finding.contains("expired"))
    );

    let future = Policy::parse(&RULE.replacen(
        "max_physical_lines = 20",
        "review_expires_on = \"2100-01-01\"\nmax_physical_lines = 20",
        1,
    ))
    .expect("policy");
    assert!(
        future
            .expired_rule_findings()
            .expect("future expiry findings")
            .is_empty()
    );

    let today = time::OffsetDateTime::now_utc().date();
    let current = Policy::parse(&RULE.replacen(
        "max_physical_lines = 20",
        &format!("review_expires_on = \"{today}\"\nmax_physical_lines = 20"),
        1,
    ))
    .expect("policy");
    assert!(
        current
            .expired_rule_findings()
            .expect("current-day expiry findings")
            .is_empty(),
        "a rule remains reviewable throughout its stated expiry date"
    );
}

#[test]
fn policy_uses_the_most_specific_matching_rule_and_reports_breaches() {
    let source = format!(
        "{RULE}\n[[rules]]\npath = \"crates/htmlcut-core/src/document/\"\nmatch = \"prefix\"\nrole = \"document role\"\nowner = \"document owner\"\nrationale = \"document rationale\"\nsplit_trigger = \"document split\"\nmax_physical_lines = 1\nmax_items = 1\nmax_public_items = 1\nmax_imports = 1\nmax_functions = 1\nmax_decision_points = 1\nmax_match_arms = 1\nallowed_internal_dependencies = [\"crate\"]\n"
    );
    let policy = Policy::parse(&source).expect("policy");
    let rule = policy
        .rule_for("crates/htmlcut-core/src/document/file.rs")
        .expect("specific rule");
    assert_eq!(rule.role(), "document role");
    assert_eq!(rule.path(), "crates/htmlcut-core/src/document/");

    let metrics =
        Metrics::from_source("use crate::document::Thing;\nfn example() { if true { } }\n")
            .expect("metrics");
    assert!(
        rule.budget_findings("file.rs", &metrics)
            .iter()
            .any(|finding| finding.contains("physical lines"))
    );
    assert!(
        rule.dependency_findings("file.rs", &BTreeSet::from(["document".to_owned()]))
            .iter()
            .any(|finding| finding.contains("forbidden"))
    );
}

#[test]
fn policy_rejects_malformed_rule_paths() {
    for replacement in [
        "path = \"/absolute/\"",
        "path = \"bad\\path/\"",
        "path = \"bad//path/\"",
        "path = \"bad/../path/\"",
        "path = \"file\"",
    ] {
        let source = RULE.replacen("path = \"crates/htmlcut-core/src/\"", replacement, 1);
        assert!(Policy::parse(&source).is_err());
    }

    let exact_non_rust = RULE
        .replacen(
            "path = \"crates/htmlcut-core/src/\"",
            "path = \"crates/htmlcut-core/src/model\"",
            1,
        )
        .replacen("match = \"prefix\"", "match = \"exact\"", 1);
    assert!(Policy::parse(&exact_non_rust).is_err());

    let escaped_separator = RULE.replacen(
        "path = \"crates/htmlcut-core/src/\"",
        "path = \"bad\\\\\\\\path/\"",
        1,
    );
    assert!(Policy::parse(&escaped_separator).is_err());
    let dot_component = RULE.replacen(
        "path = \"crates/htmlcut-core/src/\"",
        "path = \"bad/./path/\"",
        1,
    );
    assert!(Policy::parse(&dot_component).is_err());
}
