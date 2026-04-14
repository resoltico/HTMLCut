use std::fs;
use std::path::{Path, PathBuf};

use htmlcut_core::interop::v1::{
    HtmlInput, INTEROP_V1_PROFILE, InteropError, InteropResult, PLAN_SCHEMA_NAME,
    PLAN_SCHEMA_VERSION, Plan, execute_plan,
};
use url::Url;

enum ExpectedDocumentKind {
    Result,
    Error,
}

struct AcceptanceCase {
    name: &'static str,
    label: &'static str,
    input_base_url: Option<&'static str>,
    expected_kind: ExpectedDocumentKind,
}

const ACCEPTANCE_CASES: &[AcceptanceCase] = &[
    AcceptanceCase {
        name: "css_selector_ok",
        label: "target-story",
        input_base_url: Some("https://example.com/docs/start.html"),
        expected_kind: ExpectedDocumentKind::Result,
    },
    AcceptanceCase {
        name: "delimiter_pair_literal_ok",
        label: "target-fragment",
        input_base_url: None,
        expected_kind: ExpectedDocumentKind::Result,
    },
    AcceptanceCase {
        name: "delimiter_pair_regex_ok",
        label: "target-article",
        input_base_url: None,
        expected_kind: ExpectedDocumentKind::Result,
    },
    AcceptanceCase {
        name: "ambiguous_single_error",
        label: "target-news",
        input_base_url: None,
        expected_kind: ExpectedDocumentKind::Error,
    },
    AcceptanceCase {
        name: "no_match_error",
        label: "target-empty",
        input_base_url: None,
        expected_kind: ExpectedDocumentKind::Error,
    },
    AcceptanceCase {
        name: "effective_base_url_unresolved_warning",
        label: "target-relative",
        input_base_url: None,
        expected_kind: ExpectedDocumentKind::Result,
    },
];

fn fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("htmlcut-v1")
}

fn fixture_text(path: &Path) -> String {
    let mut contents = fs::read_to_string(path).expect("fixture file");
    if contents.ends_with('\n') {
        contents.pop();
        if contents.ends_with('\r') {
            contents.pop();
        }
    }
    contents
}

fn fixture_dir(case: &AcceptanceCase) -> PathBuf {
    fixture_root().join(case.name)
}

fn load_source(case: &AcceptanceCase) -> HtmlInput {
    let html = fixture_text(&fixture_dir(case).join("source.html"));
    let source = HtmlInput::new(case.label, html).expect("fixture source input");
    if let Some(input_base_url) = case.input_base_url {
        source.with_input_base_url(Url::parse(input_base_url).expect("fixture input base url"))
    } else {
        source
    }
}

fn load_plan(case: &AcceptanceCase) -> (Plan, String) {
    let path = fixture_dir(case).join("plan.json");
    let stable_json = fixture_text(&path);
    let plan: Plan = serde_json::from_str(&stable_json).expect("fixture plan");
    assert_eq!(
        plan.stable_json().expect("plan stable json"),
        stable_json,
        "{} plan fixture must already be canonical stable JSON",
        case.name
    );
    (plan, stable_json)
}

/// Regenerates every fixture JSON file from the live stack.
///
/// Run once after any schema-identity or digest-chain change:
///
/// ```bash
/// UPDATE_FIXTURES=1 cargo test -p htmlcut-core -- --ignored update_fixtures
/// ```
///
/// Inspect the diff before committing. The acceptance test must pass afterwards.
#[test]
#[ignore = "run with UPDATE_FIXTURES=1 to regenerate fixture JSON files"]
fn update_fixtures() {
    assert_eq!(
        std::env::var("UPDATE_FIXTURES").as_deref().unwrap_or(""),
        "1",
        "set UPDATE_FIXTURES=1 to run this test"
    );

    for case in ACCEPTANCE_CASES {
        let source = load_source(case);

        // Read the existing plan as a raw JSON Value so we can patch stale identity
        // fields without failing the schema-identity check inside stable_json().
        let plan_path = fixture_dir(case).join("plan.json");
        let raw = fs::read_to_string(&plan_path).expect("plan json");
        let mut plan_value: serde_json::Value =
            serde_json::from_str(raw.trim_end()).expect("plan value");
        plan_value["schema_name"] = serde_json::Value::String(PLAN_SCHEMA_NAME.to_owned());
        plan_value["schema_version"] =
            serde_json::Value::Number(serde_json::Number::from(PLAN_SCHEMA_VERSION));
        plan_value["interop_profile"] = serde_json::Value::String(INTEROP_V1_PROFILE.to_owned());
        let plan: Plan = serde_json::from_value(plan_value).expect("plan from patched value");
        let new_plan_json = plan.stable_json().expect("plan stable json");
        fs::write(&plan_path, format!("{new_plan_json}\n")).expect("write plan");

        match case.expected_kind {
            ExpectedDocumentKind::Result => {
                let result = execute_plan(&source, &plan).expect("execute plan");
                let result_json = result.stable_json().expect("result stable json");
                let result_path = fixture_dir(case).join("expected_result.json");
                fs::write(&result_path, format!("{result_json}\n")).expect("write result");
            }
            ExpectedDocumentKind::Error => {
                let err = execute_plan(&source, &plan).expect_err("execute plan error");
                let err_json = err.stable_json().expect("error stable json");
                let err_path = fixture_dir(case).join("expected_error.json");
                fs::write(&err_path, format!("{err_json}\n")).expect("write error");
            }
        }
    }
}

#[test]
fn htmlcut_v1_acceptance_fixtures_are_canonical_and_deterministic() {
    for case in ACCEPTANCE_CASES {
        let (plan, plan_json) = load_plan(case);
        let source = load_source(case);

        match case.expected_kind {
            ExpectedDocumentKind::Result => {
                let expected_path = fixture_dir(case).join("expected_result.json");
                let expected_json = fixture_text(&expected_path);
                let expected: InteropResult =
                    serde_json::from_str(&expected_json).expect("fixture result document");
                assert_eq!(
                    expected.stable_json().expect("result stable json"),
                    expected_json,
                    "{} result fixture must already be canonical stable JSON",
                    case.name
                );
                assert_eq!(
                    plan.digest_sha256().expect("plan digest"),
                    expected.plan_digest_sha256,
                    "{} plan digest must stay frozen",
                    case.name
                );

                let actual = execute_plan(&source, &plan).expect("fixture result");
                assert_eq!(actual, expected, "{} result document mismatch", case.name);
                assert_eq!(
                    actual.stable_json().expect("actual result stable json"),
                    expected_json,
                    "{} result JSON mismatch",
                    case.name
                );
                assert_eq!(
                    actual.digest_sha256().expect("actual result digest"),
                    expected.result_digest_sha256,
                    "{} result digest mismatch",
                    case.name
                );
                assert_eq!(
                    plan.stable_json().expect("plan stable json"),
                    plan_json,
                    "{} plan JSON mismatch",
                    case.name
                );
            }
            ExpectedDocumentKind::Error => {
                let expected_path = fixture_dir(case).join("expected_error.json");
                let expected_json = fixture_text(&expected_path);
                let expected: InteropError =
                    serde_json::from_str(&expected_json).expect("fixture error document");
                assert_eq!(
                    expected.stable_json().expect("error stable json"),
                    expected_json,
                    "{} error fixture must already be canonical stable JSON",
                    case.name
                );
                assert_eq!(
                    plan.digest_sha256().expect("plan digest"),
                    expected.plan_digest_sha256,
                    "{} plan digest must stay frozen",
                    case.name
                );

                let actual = execute_plan(&source, &plan).expect_err("fixture error");
                assert_eq!(*actual, expected, "{} error document mismatch", case.name);
                assert_eq!(
                    actual.stable_json().expect("actual error stable json"),
                    expected_json,
                    "{} error JSON mismatch",
                    case.name
                );
                assert_eq!(
                    actual.digest_sha256().expect("actual error digest"),
                    expected.error_digest_sha256,
                    "{} error digest mismatch",
                    case.name
                );
                assert_eq!(
                    plan.stable_json().expect("plan stable json"),
                    plan_json,
                    "{} plan JSON mismatch",
                    case.name
                );
            }
        }
    }
}
