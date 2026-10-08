// SPDX-License-Identifier: MPL-2.0
use super::*;

#[test]
fn plan_field_failure_reports_only_declared_path_segments() {
    let bad = br#"{"version":7,"select":"p","fields":{"SYNTHETIC_SECRET!":{"select":null}}}"#;
    let error = ExtractionPlan::from_json(bad).unwrap_err();
    assert_eq!(error.code, ErrorCode::InvalidPlan);
    assert_eq!(error.plan_path.as_deref(), Some("$.fields"));
    assert!(
        !serde_json::to_string(&error)
            .unwrap()
            .contains("SYNTHETIC_SECRET")
    );
    let indexed = br#"{"version":7,"select":"p","expect":[{"select":"p"},{"scope":"selected"}]}"#;
    let error = ExtractionPlan::from_json(indexed).unwrap_err();
    assert_eq!(error.plan_path.as_deref(), Some("$.expect[1]"));
}

#[test]
fn invalid_exclusion_selector_fails_the_whole_compiled_plan() {
    let mut plan = ExtractionPlan::css("p").unwrap();
    plan.exclude = Some(vec!["[a=]".into()]);
    assert_eq!(
        CompiledPlan::compile(&plan).err().unwrap().code,
        ErrorCode::InvalidSelector
    );
}

fn guard() -> Guard {
    Guard {
        select: "p".into(),
        scope: GuardScope::Document,
        min: 1,
        max: 1,
        read: Some(Reading::Literal),
        equals: Some("text".into()),
        pattern: None,
    }
}
fn reject(plan: ExtractionPlan) {
    assert!(plan.validate().is_err());
    assert!(CompiledPlan::compile(&plan).is_err());
}

#[test]
fn incompatible_cardinality_readings_and_aggregate_selectors_never_compile() {
    let base = ExtractionPlan::css("p").unwrap();
    let mut p = base.clone();
    p.version = 2;
    reject(p);
    for (min, max) in [(2, Some(1)), (1, Some(1_000_001))] {
        let mut p = base.clone();
        p.match_mode = Match::All;
        p.min = Some(min);
        p.max = max;
        reject(p);
    }
    for index in [0, 100_001] {
        let mut p = base.clone();
        p.match_mode = Match::Nth;
        p.index = Some(index);
        reject(p);
    }
    for name in [
        "".to_string(),
        "bad name".into(),
        "bad\0name".into(),
        "x".repeat(257),
    ] {
        let mut p = base.clone();
        p.read = Some(Reading::Attribute(name));
        reject(p);
    }
    let mut p = base.clone();
    p.read = Some(Reading::Attribute("href".into()));
    p.exclude = Some(vec!["span".into()]);
    reject(p);
    let mut p = base.clone();
    p.exclude = Some(vec![String::new()]);
    reject(p);
    for count in [31, 32, 33] {
        let mut p = base.clone();
        p.exclude = Some(vec!["p".into(); count]);
        assert_eq!(p.validate().is_ok(), count <= 32);
        let mut p = base.clone();
        p.expect = vec![guard(); count];
        assert_eq!(p.validate().is_ok(), count <= 32);
    }
    for member in [
        "schema",
        "strategy",
        "projection",
        "selection",
        "guards",
        "transforms",
    ] {
        let mut wire = serde_json::json!({"version":7,"select":"p"});
        wire[member] = serde_json::json!({});
        assert!(ExtractionPlan::from_json(&serde_json::to_vec(&wire).unwrap()).is_err());
    }
}
#[test]
fn expectation_predicate_reading_count_and_regex_constraints_are_closed() {
    let mut p = ExtractionPlan::css("p").unwrap();
    p.expect.push(guard());
    p.expect[0].equals = None;
    reject(p.clone());
    p.expect[0].read = None;
    p.validate().unwrap();
    p.expect[0].read = Some(Reading::Attribute("data-x".into()));
    reject(p.clone());
    p.expect[0].equals = Some("".into());
    p.validate().unwrap();
    p.expect[0].read = Some(Reading::Attribute(String::new()));
    reject(p.clone());
    p.expect[0] = guard();
    p.expect[0].min = 2;
    reject(p.clone());
    p.expect[0] = guard();
    p.expect[0].max = 100_001;
    reject(p.clone());
    p.expect[0] = guard();
    p.expect[0].select.clear();
    reject(p.clone());
    for pattern in ["(?z)text", "["] {
        p.expect[0] = guard();
        p.expect[0].equals = None;
        p.expect[0].pattern = Some(pattern.into());
        assert_eq!(
            CompiledPlan::compile(&p).err().unwrap().code,
            ErrorCode::InvalidRegex
        );
    }
    p.expect[0] = guard();
    p.expect[0].equals = None;
    p.expect[0].pattern = Some(String::new());
    reject(p);
}
#[test]
fn readings_apply_url_resolution_only_to_supported_positions() {
    for name in [
        "href",
        "src",
        "action",
        "poster",
        "cite",
        "formaction",
        "data",
    ] {
        let mut p = ExtractionPlan::css("p").unwrap();
        p.read = Some(Reading::Url(name.into()));
        p.validate().unwrap();
    }
    for name in ["srcset", "id", "", "bad name"] {
        let mut p = ExtractionPlan::css("p").unwrap();
        p.read = Some(Reading::Url(name.into()));
        reject(p);
    }
    let mut p = ExtractionPlan::css("p").unwrap();
    p.read = Some(Reading::ResolvedMarkdown);
    p.validate().unwrap();
    for retired in [
        "dom_text",
        "normalized_text",
        "resolved_markdown",
        "attribute:id",
        "source",
    ] {
        assert!(retired.parse::<Reading>().is_err());
    }
}

#[test]
fn t29_compilation_depth_quoting_flags_and_regex_size_are_bounded() {
    for selector in [r".\31 a", r#"[data-x='[(']"#] {
        assert!(CompiledPlan::compile(&ExtractionPlan::css(selector).unwrap()).is_ok());
    }
    for depth in [63, 64, 65] {
        let selector = format!("{}p{}", ":is(".repeat(depth), ")".repeat(depth));
        let result = CompiledPlan::compile(&ExtractionPlan::css(selector).unwrap());
        if depth == 65 {
            assert_eq!(result.err().unwrap().code, ErrorCode::ResourceLimit);
        } else {
            assert!(result.is_ok());
        }
    }
    assert_eq!(
        ExtractionPlan::css("x".repeat(8193)).unwrap_err().code,
        ErrorCode::ResourceLimit
    );
    for (pattern, valid) in [("(?imsUx)x", true), ("[", false), (r"\w{1000000}", false)] {
        let mut plan = ExtractionPlan::css("p").unwrap();
        let mut assertion = guard();
        assertion.equals = None;
        assertion.pattern = Some(pattern.into());
        plan.expect.push(assertion);
        let result = CompiledPlan::compile(&plan);
        assert_eq!(result.is_ok(), valid);
        if pattern == r"\w{1000000}" {
            assert_eq!(result.err().unwrap().code, ErrorCode::ResourceLimit);
        }
    }
}

#[test]
fn query_byte_budget_covers_normalized_defaults_and_escaped_predicates() {
    let mut plan = ExtractionPlan::css("p").unwrap();
    let mut assertion = guard();
    assertion.equals = Some(String::new());
    plan.expect.push(assertion);
    let overhead = CompiledPlan::compile(&plan)
        .unwrap()
        .normalized_json()
        .len();
    plan.expect[0].equals = Some("x".repeat(crate::MAX_PLAN_BYTES - overhead));
    let compiled = CompiledPlan::compile(&plan).unwrap();
    assert_eq!(compiled.normalized_json().len(), crate::MAX_PLAN_BYTES);
    ExtractionPlan::from_json(compiled.normalized_json().as_bytes()).unwrap();
    plan.expect[0].equals.as_mut().unwrap().push('x');
    assert_eq!(
        CompiledPlan::compile(&plan).err().unwrap().code,
        ErrorCode::ResourceLimit
    );
    plan.expect[0].equals = Some("\u{1}".repeat(50_000));
    plan.validate().unwrap();
    assert_eq!(
        CompiledPlan::compile(&plan).err().unwrap().code,
        ErrorCode::ResourceLimit
    );
}

#[test]
fn t29_selector_comments_and_quoted_delimiters_do_not_spend_nesting_budget() {
    let comment = format!("p/* {} */", "([".repeat(80));
    assert!(CompiledPlan::compile(&ExtractionPlan::css(comment).unwrap()).is_ok());
    for selector in [
        r#"[data-x="[(]"]"#,
        r#"[data-x='quote\"x']"#,
        r#"p:not([data-x="("])"#,
    ] {
        assert!(
            CompiledPlan::compile(&ExtractionPlan::css(selector).unwrap()).is_ok(),
            "{selector}"
        );
    }
    assert!(CompiledPlan::compile(&ExtractionPlan::css("p/* unterminated").unwrap()).is_ok());
}

#[test]
fn t29_selector_comment_terminators_and_invalid_slashes_reach_the_authoritative_parser() {
    let plan = ExtractionPlan::css("p /* *content */").unwrap();
    assert_eq!(
        prepared("<p>value</p>")
            .execute(&CompiledPlan::compile(&plan).unwrap())
            .unwrap()
            .data()
            .as_values()
            .unwrap(),
        ["value"]
    );
    let plan = ExtractionPlan::css("p / div").unwrap();
    assert_eq!(
        CompiledPlan::compile(&plan).err().unwrap().code,
        ErrorCode::InvalidSelector
    );
    assert_eq!(
        SourceSnapshot::new(
            "",
            SnapshotMetadata {
                base_url: Some("https://:synthetic@example.test".into())
            }
        )
        .unwrap_err()
        .code,
        ErrorCode::InvalidBaseUrl
    );
}

#[test]
fn current_integer_version_and_select_are_required_and_all_defaults_nonempty() {
    for wire in [
        r#"{"select":"p"}"#,
        r#"{"version":5,"select":"p"}"#,
        r#"{"version":"6","select":"p"}"#,
    ] {
        let error = ExtractionPlan::from_json(wire.as_bytes()).unwrap_err();
        assert_eq!(error.code, ErrorCode::InvalidSchema);
        assert_eq!(error.stage, "plan");
        assert_eq!(
            error.cause,
            Some(FailureCause::Configuration {
                role: ConfigurationRole::Plan,
                problem: ConfigurationProblem::UnsupportedVersion
            })
        );
    }
    assert!(ExtractionPlan::from_json(br#"{"version":7}"#).is_err());
    let plan =
        ExtractionPlan::from_json(br#"{"version":7,"select":"aside","match":"all"}"#).unwrap();
    assert_eq!(plan.match_mode, Match::All);
    assert_eq!(plan.min, None);
    assert_eq!(plan.max, None);
    assert_eq!(
        prepared("<p>value</p>")
            .execute(&CompiledPlan::compile(&plan).unwrap())
            .unwrap_err()
            .code,
        ErrorCode::Cardinality
    );
}

#[test]
fn t29_unicode_regex_programs_share_one_aggregate_compilation_allowance() {
    // Under the locked regex engine, this valid Unicode program fits one program share
    // with its DFA allowance, but not half that program share. Syntax and source size are unchanged.
    let mut css = ExtractionPlan::css("p").unwrap();
    let guard = Guard {
        select: "p".into(),
        scope: GuardScope::Document,
        min: 1,
        max: 1,
        read: Some(Reading::Literal),
        equals: None,
        pattern: Some(r"\w{50}".into()),
    };
    css.expect.push(guard.clone());
    CompiledPlan::compile(&css).unwrap();
    css.expect.push(guard);
    assert_eq!(
        CompiledPlan::compile(&css).err().unwrap().code,
        ErrorCode::ResourceLimit
    );
}

#[test]
fn t29_quote_and_comment_content_cannot_inflate_selector_grammar_depth() {
    let parentheses = "(".repeat(130);
    for selector in [
        format!("p[data-value='{parentheses}']"),
        format!("p[data-value=\"{parentheses}\"]"),
        format!("p[data-value='\\'{parentheses}']"),
        format!("p/* *not-close / {parentheses} */"),
    ] {
        CompiledPlan::compile(&ExtractionPlan::css(&selector).unwrap())
            .unwrap_or_else(|error| panic!("{selector}: {error:?}"));
    }
}

#[test]
fn t29_exact_selector_bytes_are_allowed_and_oversized_strings_fail_before_semantics() {
    let prefix = "p/*";
    let suffix = "*/";
    let selector = format!(
        "{prefix}{}{suffix}",
        "x".repeat(8192 - prefix.len() - suffix.len())
    );
    let plan = ExtractionPlan::css(&selector).unwrap();
    CompiledPlan::compile(&plan).unwrap();
    let mut oversized = ExtractionPlan::css("p").unwrap();
    oversized.expect.push(Guard {
        select: "p".into(),
        scope: GuardScope::Document,
        min: 2,
        max: 1,
        read: Some(Reading::Literal),
        equals: Some("x".repeat(256 * 1024 + 1)),
        pattern: None,
    });
    assert_eq!(
        oversized.validate().unwrap_err().code,
        ErrorCode::ResourceLimit
    );
}

#[test]
fn t29_many_sequential_selector_delimiters_do_not_accumulate_nesting() {
    let selector = format!("p{}", "[data-x='value']".repeat(100));
    CompiledPlan::compile(&ExtractionPlan::css(&selector).unwrap()).unwrap();
}

#[test]
fn t29_wildcards_inside_nested_pseudo_classes_cannot_hide_excess_grammar_depth() {
    let selector = format!("{}p{}", ":is(*".repeat(65), ")".repeat(65));
    let plan = ExtractionPlan::css(&selector).unwrap();
    assert_eq!(
        CompiledPlan::compile(&plan).err().unwrap().code,
        ErrorCode::ResourceLimit
    );
}
