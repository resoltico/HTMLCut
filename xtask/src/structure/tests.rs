use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use htmlcut_tempdir::tempdir;

use super::metrics::{Metrics, measured_internal_dependencies};
use super::policy::Policy;
use super::{
    check_source_structure, collect_findings, collect_rust_files, maintained_sources,
    report_source_structure, source_from_path,
};

const RULE: &str = r#"
version = 1

[[rules]]
path = "crates/htmlcut-core/src/"
match = "prefix"
role = "test role"
owner = "test owner"
rationale = "test rationale"
split_trigger = "test split trigger"
max_physical_lines = 20
max_items = 20
max_public_items = 20
max_imports = 20
max_functions = 20
max_decision_points = 20
max_match_arms = 20
allowed_internal_dependencies = ["crate", "document"]
"#;

#[test]
fn metrics_measure_items_functions_and_decisions_from_rust_syntax() {
    let metrics = Metrics::from_source(
        "use crate::document::Thing;\npub struct Visible;\nfn example() { if true && false { match 1 { 1 => (), _ => () } } }\n",
    )
    .expect("metrics");

    assert_eq!(metrics.physical_lines, 3);
    assert_eq!(metrics.item_count, 3);
    assert_eq!(metrics.public_item_count, 1);
    assert_eq!(metrics.import_count, 1);
    assert_eq!(metrics.function_count, 1);
    assert_eq!(metrics.decision_points, 4);
    assert_eq!(metrics.match_arms, 2);
}

#[test]
fn dependency_measurement_records_named_crate_modules_not_relative_imports() {
    let dependencies = measured_internal_dependencies(
        "use crate::{PublicType, document::{self, Thing}, *};\nuse crate::renamed as Alias;\nuse crate::*;\nuse {super::sibling::Thing as RelativeThing, std::fmt};\n",
    )
    .expect("dependencies");

    assert_eq!(
        dependencies,
        BTreeSet::from(["crate".to_owned(), "document".to_owned()])
    );
}

#[test]
fn metrics_cover_public_item_kinds_all_function_forms_and_control_flow() {
    let public_items = Metrics::from_source(
        "pub const CONSTANT: u8 = 1;\npub enum Choice { One }\npub extern crate alloc;\npub fn public_function() {}\npub mod nested {}\npub static STATIC: u8 = 1;\npub struct Shape;\npub trait Contract { fn required(&self); }\npub trait Alias = Contract;\npub type NamedShape = Shape;\npub union Packed { field: u8 }\npub use self::Shape as ExportedShape;\nmacro_rules! private_macro { () => {}; }\nimpl Shape { fn method(&self) {} }\n",
    )
    .expect("public-item metrics");
    assert_eq!(public_items.item_count, 14);
    assert_eq!(public_items.public_item_count, 12);
    assert_eq!(public_items.function_count, 3);

    let control_flow = Metrics::from_source(
        "fn control_flow() {\n    if true || false {}\n    for _ in 0..1 {}\n    loop { break; }\n    while false {}\n    match 1 { 1 => (), _ => () };\n    let _ = 1 + 2;\n}\n",
    )
    .expect("control-flow metrics");
    assert_eq!(control_flow.decision_points, 7);
    assert_eq!(control_flow.match_arms, 2);

    assert!(Metrics::from_source("pub fn broken( {").is_err());
    assert!(measured_internal_dependencies("use crate::{;").is_err());
}

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

#[test]
fn structure_commands_report_and_enforce_owned_and_unowned_sources() {
    let root = tempdir().expect("temporary repository");
    write(root.path(), "tooling/rust-source-shape-policy.toml", RULE);
    write(
        root.path(),
        "crates/htmlcut-core/src/owned.rs",
        "pub fn owned() {}\n",
    );

    check_source_structure(root.path()).expect("owned source passes structure check");
    report_source_structure(root.path()).expect("owned source is reportable");

    let unowned_root = tempdir().expect("temporary repository");
    let unused_rule = RULE
        .replacen(
            "path = \"crates/htmlcut-core/src/\"",
            "path = \"crates/htmlcut-core/src/declared.rs\"",
            1,
        )
        .replacen("match = \"prefix\"", "match = \"exact\"", 1);
    write(
        unowned_root.path(),
        "tooling/rust-source-shape-policy.toml",
        &unused_rule,
    );
    write(
        unowned_root.path(),
        "crates/htmlcut-core/src/actual.rs",
        "fn actual() {}\n",
    );
    report_source_structure(unowned_root.path()).expect("unowned source remains reportable");
    let error = check_source_structure(unowned_root.path()).expect_err("unowned source fails");
    assert!(
        error
            .to_string()
            .contains("source-structure contract failed")
    );

    let missing_policy_root = tempdir().expect("temporary repository");
    let error = check_source_structure(missing_policy_root.path())
        .expect_err("missing source-shape policy must fail");
    assert!(
        error
            .to_string()
            .contains("cannot read Rust source-shape policy")
    );
}

#[test]
fn maintained_source_inventory_includes_only_first_party_roots_and_is_sorted() {
    let root = tempdir().expect("temporary repository");
    write(root.path(), "crates/htmlcut-core/src/z.rs", "fn z() {}\n");
    write(root.path(), "crates/htmlcut-core/src/a.rs", "fn a() {}\n");
    write(
        root.path(),
        "patches/rust/scraper/src/error.rs",
        "fn ignored() {}\n",
    );
    write(
        root.path(),
        "semver-baseline/htmlcut-core/src/lib.rs",
        "fn ignored() {}\n",
    );
    let sources = maintained_sources(root.path()).expect("sources");
    let names = sources
        .iter()
        .map(|source| source.relative_path.as_str())
        .collect::<Vec<_>>();

    assert_eq!(
        names,
        vec![
            "crates/htmlcut-core/src/a.rs",
            "crates/htmlcut-core/src/z.rs"
        ]
    );
}

#[test]
fn maintained_source_inventory_uses_git_paths_and_deduplicates_repeated_entries() {
    let root = tempdir().expect("temporary repository");
    fs::write(root.path().join(".git"), "gitdir: /tmp/htmlcut.git\n").expect("git marker");
    write(
        root.path(),
        "crates/htmlcut-core/src/model.rs",
        "fn model() {}\n",
    );
    write(root.path(), "scripts/helper.rs", "fn ignored() {}\n");

    let sources = crate::command_exec::with_capture_command_output_override(
        |_, spec| {
            (spec.program == Path::new("git")).then(|| {
                Ok(b"crates/htmlcut-core/src/model.rs\0crates/htmlcut-core/src/model.rs\0scripts/helper.rs\0".to_vec())
            })
        },
        || maintained_sources(root.path()),
    )
    .expect("Git-backed source inventory");

    assert_eq!(sources.len(), 1);
    assert_eq!(sources[0].relative_path, "crates/htmlcut-core/src/model.rs");
}

#[test]
fn recursive_inventory_handles_missing_non_rust_and_invalid_filesystem_entries() {
    let root = tempdir().expect("temporary repository");
    let mut paths = Vec::new();
    collect_rust_files(&root.path().join("missing"), &mut paths).expect("missing root is empty");
    let note = root.path().join("note.txt");
    fs::write(&note, "not Rust").expect("write note");
    collect_rust_files(&note, &mut paths).expect("non-Rust file is ignored");
    assert!(paths.is_empty());
    assert!(collect_rust_files(Path::new("\0"), &mut paths).is_err());
}

#[cfg(unix)]
#[test]
fn recursive_inventory_rejects_symlinked_and_special_source_paths() {
    let root = tempdir().expect("temporary repository");
    let outside = tempdir().expect("outside repository");
    let symlinked_root = root.path().join("crates/htmlcut-core/src");
    fs::create_dir_all(symlinked_root.parent().expect("core parent")).expect("core parent");
    std::os::unix::fs::symlink(outside.path(), &symlinked_root).expect("source-root symlink");
    let error = maintained_sources(root.path()).expect_err("symlinked source root fails");
    assert!(error.to_string().contains("rejects symlinked source path"));

    let special = root.path().join("special.rs");
    let status = std::process::Command::new("mkfifo")
        .arg(&special)
        .status()
        .expect("run mkfifo");
    assert!(status.success(), "mkfifo succeeds");
    let mut paths = Vec::new();
    let error = collect_rust_files(&special, &mut paths).expect_err("FIFO is not a source file");
    assert!(error.to_string().contains("non-file, non-directory"));
}

#[test]
fn source_paths_resolve_roots_and_reject_outside_or_non_rust_files() {
    let root = tempdir().expect("temporary repository");
    let file = root.path().join("crates/htmlcut-core/src/document/item.rs");
    write_path(&file, "fn item() {}\n");
    let source = source_from_path(root.path(), file)
        .expect("source inventory")
        .expect("source");
    assert_eq!(
        source.relative_path,
        "crates/htmlcut-core/src/document/item.rs"
    );
    assert!(
        source_from_path(root.path(), root.path().join("README.md"))
            .expect("non-Rust inventory")
            .is_none()
    );
    assert!(
        source_from_path(root.path(), root.path().join("patches/rust/file.rs"))
            .expect("unmanaged inventory")
            .is_none()
    );
    let outside_root = tempdir().expect("outside repository");
    let outside_source = outside_root.path().join("outside.rs");
    write_path(&outside_source, "fn outside() {}\n");
    let error = source_from_path(root.path(), outside_source).expect_err("outside source fails");
    assert!(
        error
            .to_string()
            .contains("path outside the repository root")
    );
}

#[cfg(unix)]
#[test]
fn source_inventory_rejects_symlinked_rust_source() {
    let root = tempdir().expect("temporary repository");
    let outside = root.path().join("outside.rs");
    write_path(&outside, "fn outside() {}\n");
    let symlink = root.path().join("crates/htmlcut-core/src/linked.rs");
    fs::create_dir_all(symlink.parent().expect("parent")).expect("source directory");
    std::os::unix::fs::symlink(&outside, &symlink).expect("source symlink");

    let error = source_from_path(root.path(), symlink).expect_err("symlink rejected");
    assert!(error.to_string().contains("regular, non-symlink"));
}

#[cfg(unix)]
#[test]
fn source_inventory_rejects_non_regular_rust_sources() {
    let root = tempdir().expect("temporary repository");
    let special = root.path().join("crates/htmlcut-core/src/special.rs");
    fs::create_dir_all(special.parent().expect("source parent")).expect("source parent");
    let status = std::process::Command::new("mkfifo")
        .arg(&special)
        .status()
        .expect("run mkfifo");
    assert!(status.success(), "mkfifo succeeds");

    let error = source_from_path(root.path(), special).expect_err("FIFO is not a source file");
    assert!(error.to_string().contains("regular, non-symlink"));
}

#[cfg(unix)]
#[test]
fn source_inventory_rejects_a_source_escaping_through_an_ancestor_symlink() {
    let root = tempdir().expect("temporary repository");
    let outside = tempdir().expect("outside repository");
    let outside_source = outside.path().join("src/item.rs");
    write_path(&outside_source, "fn item() {}\n");
    let linked_core = root.path().join("crates/htmlcut-core");
    fs::create_dir_all(linked_core.parent().expect("crates parent")).expect("crates parent");
    std::os::unix::fs::symlink(outside.path(), &linked_core).expect("ancestor symlink");

    let escaped = linked_core.join("src/item.rs");
    let error = source_from_path(root.path(), escaped).expect_err("escaped source rejected");
    assert!(
        error
            .to_string()
            .contains("source escaping the repository root")
    );
}

#[test]
fn findings_cover_unowned_files_unused_rules_budgets_and_dependencies() {
    let root = tempdir().expect("temporary repository");
    write(root.path(), "tooling/rust-source-shape-policy.toml", RULE);
    write(
        root.path(),
        "crates/htmlcut-core/src/model.rs",
        "use crate::forbidden::Thing;\nfn one() {}\nfn two() {}\n",
    );
    let policy = Policy::parse(
        &RULE
            .replacen("max_physical_lines = 20", "max_physical_lines = 1", 1)
            .replacen("max_functions = 20", "max_functions = 1", 1),
    )
    .expect("policy");
    let findings = collect_findings(root.path(), &policy).expect("findings");

    assert!(
        findings
            .iter()
            .any(|finding| finding.contains("physical lines"))
    );
    assert!(findings.iter().any(|finding| finding.contains("functions")));
    assert!(findings.iter().any(|finding| finding.contains("forbidden")));

    let no_rules = Policy::parse(
        "version = 1\n\n[[rules]]\npath = \"crates/htmlcut-core/src/unused.rs\"\nmatch = \"exact\"\nrole = \"unused\"\nowner = \"owner\"\nrationale = \"rationale\"\nsplit_trigger = \"trigger\"\nmax_physical_lines = 10\nmax_items = 10\nmax_public_items = 10\nmax_imports = 10\nmax_functions = 10\nmax_decision_points = 10\nmax_match_arms = 10\n",
    )
    .expect("policy");
    let no_rule_findings = collect_findings(root.path(), &no_rules).expect("findings");
    assert!(
        no_rule_findings
            .iter()
            .any(|finding| finding.contains("no declared ownership rule"))
    );
    assert!(
        no_rule_findings
            .iter()
            .any(|finding| finding.contains("matches no maintained"))
    );
}

fn write(root: &Path, relative: &str, contents: &str) {
    write_path(&root.join(relative), contents);
}

fn write_path(path: &Path, contents: &str) {
    fs::create_dir_all(path.parent().expect("parent")).expect("parent directories");
    fs::write(path, contents).expect("fixture file");
}
