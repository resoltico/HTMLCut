use super::*;
use crate::interop::v1::{DomCanonicalization, execute_plan, prepare_plan};

fn dom_canonicalization(
    ignored_attributes: &[&str],
    strip_whitespace_nodes: bool,
) -> DomCanonicalization {
    DomCanonicalization::new(
        ignored_attributes
            .iter()
            .map(|name| AttributeName::new(*name).expect("canonicalized attribute name")),
        strip_whitespace_nodes,
    )
}

fn selector_text_plan() -> Plan {
    Plan::new(
        PlanStrategy::css_selector(css_selector("article.measurement")),
        Selection::single(),
        Output::text(),
        Rendering::new(TextWhitespace::Normalize, false),
    )
}

fn source() -> HtmlInput {
    HtmlInput::new(
        "canonicalization-fixture",
        concat!(
            "<article class=\"measurement\" data-nonce=\"volatile\">",
            "<!-- transient -->",
            "<a href=\"/offer\" data-nonce=\"volatile\">Guide</a>",
            "  \n",
            "</article>"
        ),
    )
    .expect("fixture source")
}

#[test]
fn dom_canonicalization_selects_original_candidates_and_preserves_raw_evidence() {
    let source = source();
    let baseline_plan = selector_text_plan();
    let canonicalization = dom_canonicalization(&["data-nonce"], true);
    let canonical_plan = selector_text_plan().with_dom_canonicalization(canonicalization);

    let baseline = execute_plan(&source, &baseline_plan).expect("baseline extraction");
    let canonical = execute_plan(&source, &canonical_plan).expect("canonical extraction");
    let baseline_match = baseline
        .selected_matches
        .first()
        .expect("baseline selected match");
    let canonical_match = canonical
        .selected_matches
        .first()
        .expect("canonical selected match");

    assert_eq!(baseline.candidate_count, canonical.candidate_count);
    assert_eq!(baseline.candidate_count, 1);
    assert_eq!(canonical_match.text_output, baseline_match.text_output);
    assert_eq!(
        canonical_match.comparison_text_output.as_deref(),
        Some(baseline_match.text_output.as_str())
    );
    assert_eq!(canonical_match.output_value, baseline_match.output_value);
    assert!(
        canonical_match
            .outer_html_output
            .contains("data-nonce=\"volatile\"")
    );
    assert!(
        canonical_match
            .outer_html_output
            .contains("<!-- transient -->")
    );
    let SelectedMatchMetadata::CssSelector { attributes, .. } = &canonical_match.metadata else {
        unreachable!("CSS plan must return CSS metadata");
    };
    assert_eq!(
        attributes.get("data-nonce").map(String::as_str),
        Some("volatile")
    );
    assert_ne!(
        baseline_plan.digest_sha256().expect("baseline plan digest"),
        canonical_plan
            .digest_sha256()
            .expect("canonical plan digest")
    );
}

#[test]
fn dom_canonicalization_changes_only_the_clone_rendered_text_projection() {
    let source = source();
    let canonical_plan = Plan::new(
        PlanStrategy::css_selector(css_selector("article.measurement a")),
        Selection::single(),
        Output::text(),
        Rendering::new(TextWhitespace::Normalize, false),
    )
    .with_dom_canonicalization(dom_canonicalization(&["href"], false));

    let result = execute_plan(&source, &canonical_plan).expect("canonical extraction");
    let selected = result.selected_matches.first().expect("selected match");

    assert_eq!(result.candidate_count, 1);
    assert_eq!(selected.text_output, "Guide [/offer]");
    assert_eq!(selected.comparison_text_output.as_deref(), Some("Guide"));
    assert_eq!(selected.output_value, serde_json::json!("Guide"));
    assert!(selected.outer_html_output.contains("href=\"/offer\""));
    let SelectedMatchMetadata::CssSelector { attributes, .. } = &selected.metadata else {
        unreachable!("CSS plan must return CSS metadata");
    };
    assert_eq!(attributes.get("href").map(String::as_str), Some("/offer"));
}

#[test]
fn dom_canonicalization_keeps_structured_output_raw() {
    let plan = Plan::new(
        PlanStrategy::css_selector(css_selector("article.measurement a")),
        Selection::single(),
        Output::structured(),
        Rendering::new(TextWhitespace::Normalize, false),
    )
    .with_dom_canonicalization(dom_canonicalization(&["href"], false));

    let result = execute_plan(&source(), &plan).expect("canonical structured extraction");
    let selected = result.selected_matches.first().expect("selected match");
    let structured = selected
        .output_value
        .as_object()
        .expect("structured output value");

    assert_eq!(selected.comparison_text_output.as_deref(), Some("Guide"));
    assert_eq!(
        structured.get("textOutput"),
        Some(&serde_json::json!("Guide [/offer]"))
    );
    assert_eq!(
        structured
            .get("attributes")
            .and_then(serde_json::Value::as_object)
            .and_then(|attributes| attributes.get("href")),
        Some(&serde_json::json!("/offer"))
    );
    assert!(!structured.contains_key("comparisonTextOutput"));
}

#[test]
fn dom_canonicalization_has_no_implicit_attribute_denylist() {
    let plan = Plan::new(
        PlanStrategy::css_selector(css_selector("article.measurement a")),
        Selection::single(),
        Output::text(),
        Rendering::new(TextWhitespace::Normalize, false),
    )
    .with_dom_canonicalization(dom_canonicalization(&[], false));

    let result = execute_plan(&source(), &plan).expect("canonical extraction");
    let selected = result.selected_matches.first().expect("selected match");

    assert_eq!(selected.text_output, "Guide [/offer]");
    assert_eq!(
        selected.comparison_text_output.as_deref(),
        Some("Guide [/offer]")
    );
    assert_eq!(selected.output_value, serde_json::json!("Guide [/offer]"));
    assert!(selected.outer_html_output.contains("href=\"/offer\""));
    assert!(
        selected
            .outer_html_output
            .contains("data-nonce=\"volatile\"")
    );
}

#[test]
fn dom_canonicalization_rejects_raw_outputs_without_a_comparison_projection() {
    for output in [
        Output::inner_html(),
        Output::outer_html(),
        Output::attribute(AttributeName::new("href").expect("href attribute")),
    ] {
        let plan = Plan::new(
            PlanStrategy::css_selector(css_selector("article.measurement a")),
            Selection::single(),
            output.clone(),
            Rendering::new(TextWhitespace::Normalize, false),
        )
        .with_dom_canonicalization(dom_canonicalization(&["data-nonce"], true));

        assert!(matches!(
            plan.validate(),
            Err(ContractError::DomCanonicalizationRequiresComparisonTextOutput { output_kind })
                if output_kind == output.kind()
        ));
        let error = prepare_plan(&plan).expect_err("raw output canonicalization must fail");
        assert_eq!(error.error_code, ErrorCode::PlanInvalid);
    }
}

#[test]
fn dom_canonicalization_rejects_removed_controls_non_css_strategies_and_ignored_measured_attributes()
 {
    for removed_control in ["sort_attributes", "strip_comments"] {
        let mut canonicalization = serde_json::json!({
            "ignore_attributes": [],
            "strip_whitespace_nodes": false
        });
        canonicalization
            .as_object_mut()
            .expect("canonicalization object")
            .insert(removed_control.to_owned(), serde_json::json!(true));
        let removed_control_error = serde_json::from_value::<Plan>(serde_json::json!({
            "schema_name": "htmlcut.plan",
            "schema_version": 8,
            "interop_profile": "htmlcut-v1",
            "strategy": {"kind": "css_selector", "selector": "article.measurement"},
            "selection": {"mode": "single"},
            "output": {"kind": "text"},
            "rendering": {"whitespace": "normalize", "rewrite_urls": false},
            "dom_canonicalization": canonicalization
        }))
        .expect_err("removed canonicalization controls must be rejected, not ignored");
        assert!(removed_control_error.to_string().contains(removed_control));
    }

    let non_css = Plan::new(
        PlanStrategy::delimiter_pair(
            delimiter_boundary("<article>"),
            delimiter_boundary("</article>"),
            DelimiterMode::Literal,
            DelimiterBoundaryRetention::ExcludeBoth,
            Vec::new(),
        ),
        Selection::single(),
        Output::text(),
        Rendering::new(TextWhitespace::Normalize, false),
    )
    .with_dom_canonicalization(dom_canonicalization(&["data-nonce"], false));
    assert!(matches!(
        non_css.validate(),
        Err(ContractError::DomCanonicalizationRequiresCssSelector)
    ));
    let non_css_error = prepare_plan(&non_css).expect_err("non-CSS canonicalization must fail");
    assert_eq!(non_css_error.error_code, ErrorCode::PlanInvalid);
    assert_eq!(
        non_css_error.details.get("contract_error"),
        Some(&serde_json::json!(
            "dom_canonicalization is only valid for css_selector strategies"
        ))
    );

    let ignored_measured_attribute = Plan::new(
        PlanStrategy::css_selector(css_selector("article.measurement a")),
        Selection::single(),
        Output::attribute(AttributeName::new("href").expect("href attribute")),
        Rendering::new(TextWhitespace::Normalize, false),
    )
    .with_dom_canonicalization(dom_canonicalization(&["HREF"], false));
    assert!(matches!(
        ignored_measured_attribute.validate(),
        Err(ContractError::DomCanonicalizationIgnoresMeasuredAttribute { .. })
    ));
    let ignored_measured_attribute_error = prepare_plan(&ignored_measured_attribute)
        .expect_err("ignore-and-measure canonicalization must fail");
    assert_eq!(
        ignored_measured_attribute_error.error_code,
        ErrorCode::PlanInvalid
    );
}

#[test]
fn dom_canonicalization_strips_whitespace_nodes_only_from_comparison_text() {
    let source = HtmlInput::new(
        "canonical-whitespace-fixture",
        "<pre>Alpha<span></span>  \n  <span></span>Beta</pre>",
    )
    .expect("fixture source");
    let plan = Plan::new(
        PlanStrategy::css_selector(css_selector("pre")),
        Selection::single(),
        Output::text(),
        Rendering::new(TextWhitespace::Rendered, false),
    )
    .with_dom_canonicalization(dom_canonicalization(&[], true));

    let result = execute_plan(&source, &plan).expect("canonical whitespace extraction");
    let selected = result.selected_matches.first().expect("selected match");
    let comparison = selected
        .comparison_text_output
        .as_deref()
        .expect("comparison text");

    assert_ne!(comparison, selected.text_output);
    assert_eq!(selected.output_value, serde_json::json!(comparison));
    assert!(selected.text_output.contains("Beta"));
    assert!(comparison.contains("Beta"));
}

#[test]
fn dom_canonicalization_all_selection_keeps_each_raw_candidate_independent() {
    let source = HtmlInput::new(
        "canonical-all-fixture",
        concat!(
            "<article class=\"measurement\" data-nonce=\"one\"><a href=\"/one\">One</a>  \n</article>",
            "<article class=\"measurement\" data-nonce=\"two\"><a href=\"/two\">Two</a>  \n</article>"
        ),
    )
    .expect("fixture source");
    let plan = Plan::new(
        PlanStrategy::css_selector(css_selector("article.measurement")),
        Selection::all(),
        Output::text(),
        Rendering::new(TextWhitespace::Normalize, false),
    )
    .with_dom_canonicalization(dom_canonicalization(&["data-nonce", "href"], true));

    let result = execute_plan(&source, &plan).expect("canonical all extraction");

    assert_eq!(result.candidate_count, 2);
    assert_eq!(result.selected_matches.len(), 2);
    for (selected, expected) in result
        .selected_matches
        .iter()
        .zip([("One [/one]", "One", "/one"), ("Two [/two]", "Two", "/two")])
    {
        assert_eq!(selected.text_output, expected.0);
        assert_eq!(selected.comparison_text_output.as_deref(), Some(expected.1));
        assert_eq!(selected.output_value, serde_json::json!(expected.1));
        assert!(selected.outer_html_output.contains("data-nonce"));
        assert!(selected.outer_html_output.contains(expected.2));
        let SelectedMatchMetadata::CssSelector { attributes, .. } = &selected.metadata else {
            unreachable!("CSS plan must return CSS metadata");
        };
        assert!(attributes.contains_key("data-nonce"));
    }
}
