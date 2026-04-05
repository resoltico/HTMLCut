use std::fs;
use std::path::{Path, PathBuf};

use htmlcut_core::interop::ffhn_v1::{
    FfhnError, FfhnPlan, FfhnResult, FfhnSourceInput, execute_ffhn_plan,
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
        .join("ffhn-htmlcut-v1")
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

fn load_source(case: &AcceptanceCase) -> FfhnSourceInput {
    let html = fixture_text(&fixture_dir(case).join("source.html"));
    let source = FfhnSourceInput::new(case.label, html).expect("fixture source input");
    if let Some(input_base_url) = case.input_base_url {
        source.with_input_base_url(Url::parse(input_base_url).expect("fixture input base url"))
    } else {
        source
    }
}

fn load_plan(case: &AcceptanceCase) -> (FfhnPlan, String) {
    let path = fixture_dir(case).join("plan.json");
    let stable_json = fixture_text(&path);
    let plan: FfhnPlan = serde_json::from_str(&stable_json).expect("fixture plan");
    assert_eq!(
        plan.stable_json().expect("plan stable json"),
        stable_json,
        "{} plan fixture must already be canonical stable JSON",
        case.name
    );
    (plan, stable_json)
}

#[test]
fn ffhn_htmlcut_v1_acceptance_fixtures_are_canonical_and_deterministic() {
    for case in ACCEPTANCE_CASES {
        let (plan, plan_json) = load_plan(case);
        let source = load_source(case);

        match case.expected_kind {
            ExpectedDocumentKind::Result => {
                let expected_path = fixture_dir(case).join("expected_result.json");
                let expected_json = fixture_text(&expected_path);
                let expected: FfhnResult =
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

                let actual = execute_ffhn_plan(&source, &plan).expect("fixture result");
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
                let expected: FfhnError =
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

                let actual = execute_ffhn_plan(&source, &plan).expect_err("fixture error");
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
