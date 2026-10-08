// SPDX-License-Identifier: MPL-2.0
use super::*;

#[test]
fn t18_t19_guards_are_original_dom_conjunctive_and_all_values() {
    let source = prepared(
        "<h2 id='label'>Repair cost</h2><p id='amount' data-x=''>EUR 180<span class='remove'> RAW </span></p><b>good</b><b>bad</b>",
    );
    let mut plan = ExtractionPlan::css("#amount").unwrap();
    plan.exclude = Some(vec![".remove".into()]);
    plan.read = Some(Reading::Text);
    plan.expect.push(Guard {
        select: "#label".into(),
        scope: GuardScope::Document,
        min: 1,
        max: 1,
        read: Some(Reading::Literal),
        equals: Some("Repair cost".into()),
        pattern: None,
    });
    plan.expect.push(Guard {
        select: ":scope .remove".into(),
        scope: GuardScope::Selected,
        min: 1,
        max: 1,
        read: Some(Reading::Literal),
        equals: Some(" RAW ".into()),
        pattern: None,
    });
    assert_eq!(
        source
            .execute(&CompiledPlan::compile(&plan).unwrap())
            .unwrap()
            .data()
            .as_values()
            .unwrap(),
        ["EUR 180"]
    );
    plan.expect[0].equals = Some("Tax".into());
    plan.expect[0].pattern = None;
    assert_eq!(
        source
            .execute(&CompiledPlan::compile(&plan).unwrap())
            .unwrap_err()
            .code,
        ErrorCode::GuardFailed
    );
    plan.expect[0].select = "b".into();
    plan.expect[0].max = 2;
    plan.expect[0].equals = None;
    plan.expect[0].pattern = Some("good".into());
    assert_eq!(
        source
            .execute(&CompiledPlan::compile(&plan).unwrap())
            .unwrap_err()
            .code,
        ErrorCode::GuardFailed
    );
    plan.expect.clear();
    plan.expect.push(Guard {
        select: ":scope[data-x]".into(),
        scope: GuardScope::Selected,
        min: 1,
        max: 1,
        read: None,
        equals: None,
        pattern: None,
    });
    assert!(
        source
            .execute(&CompiledPlan::compile(&plan).unwrap())
            .is_ok()
    );
    plan.expect[0].select = ":scope[absent]".into();
    assert_eq!(
        source
            .execute(&CompiledPlan::compile(&plan).unwrap())
            .unwrap_err()
            .code,
        ErrorCode::GuardFailed
    );
}

#[test]
fn t16_t17_complete_counts_positions_and_explicit_empty_selection() {
    let source = prepared("<p>A</p><p>B</p><p>C</p>");
    let mut plan = ExtractionPlan::css("p, p").unwrap();
    plan.match_mode = Match::Nth;
    plan.index = Some(2);
    let result = source
        .execute(&CompiledPlan::compile(&plan).unwrap())
        .unwrap();
    assert_eq!(result.data().as_values().unwrap(), ["B"]);
    assert_eq!((result.candidate_count(), result.selected_count()), (3, 1));
    plan.limits.max_candidates = 2;
    assert_eq!(
        source
            .execute(&CompiledPlan::compile(&plan).unwrap())
            .unwrap_err()
            .code,
        ErrorCode::ResourceLimit
    );
    plan = ExtractionPlan::css("aside").unwrap();
    plan.match_mode = Match::All;
    plan.min = Some(0);
    plan.max = Some(0);
    assert!(
        source
            .execute(&CompiledPlan::compile(&plan).unwrap())
            .unwrap()
            .data()
            .as_values()
            .unwrap()
            .is_empty()
    );
    plan.match_mode = Match::All;
    plan.min = Some(1);
    plan.max = None;
    assert_eq!(
        source
            .execute(&CompiledPlan::compile(&plan).unwrap())
            .unwrap_err()
            .code,
        ErrorCode::Cardinality
    );
    plan.min = None;
    plan.max = None;
    plan.match_mode = Match::Nth;
    plan.index = Some(0);
    assert_eq!(
        CompiledPlan::compile(&plan).err().unwrap().code,
        ErrorCode::InvalidPlan
    );
}

#[test]
fn t18_guard_cardinality_scope_and_search_predicate_are_declared() {
    let source = prepared("<p id='root'><span>prefix VALUE suffix</span></p><span>outside</span>");
    let mut plan = ExtractionPlan::css("#root").unwrap();
    plan.expect = vec![Guard {
        select: "span".into(),
        scope: GuardScope::Selected,
        min: 1,
        max: 1,
        read: Some(Reading::Literal),
        equals: None,
        pattern: Some("VALUE".into()),
    }];
    assert!(
        source
            .execute(&CompiledPlan::compile(&plan).unwrap())
            .is_ok()
    );
    plan.expect[0].select = "aside".into();
    assert_eq!(
        source
            .execute(&CompiledPlan::compile(&plan).unwrap())
            .unwrap_err()
            .code,
        ErrorCode::GuardFailed
    );
    plan.expect[0].min = 0;
    assert!(
        source
            .execute(&CompiledPlan::compile(&plan).unwrap())
            .is_ok()
    );
    plan.expect[0].scope = GuardScope::Document;
    plan.expect[0].select = "span".into();
    plan.expect[0].min = 1;
    assert_eq!(
        source
            .execute(&CompiledPlan::compile(&plan).unwrap())
            .unwrap_err()
            .code,
        ErrorCode::GuardFailed
    );
    plan.expect.clear();
    plan.match_mode = Match::Nth;
    plan.index = Some(2);
    assert_eq!(
        source
            .execute(&CompiledPlan::compile(&plan).unwrap())
            .unwrap_err()
            .code,
        ErrorCode::Cardinality
    );
}

#[test]
fn t16_t18_declared_selection_and_guard_candidate_caps_fail_whole_operations() {
    let source = prepared("<p>A</p><p>B</p><b>X</b><b>Y</b>");
    let mut plan = ExtractionPlan::css("p").unwrap();
    plan.match_mode = Match::All;
    plan.min = Some(1);
    plan.max = Some(1);
    let error = source
        .execute(&CompiledPlan::compile(&plan).unwrap())
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::Cardinality);
    assert_eq!(error.candidate_count, Some(2));
    let mut plan = ExtractionPlan::css("p:first-of-type").unwrap();
    plan.limits.max_candidates = 1;
    plan.expect.push(Guard {
        select: "b".into(),
        scope: GuardScope::Document,
        min: 0,
        max: 1,
        read: Some(Reading::Literal),
        equals: Some("X".into()),
        pattern: None,
    });
    let error = source
        .execute(&CompiledPlan::compile(&plan).unwrap())
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::ResourceLimit);
    assert_eq!(error.observed_at_least, Some(2));
}

#[test]
fn t17_the_last_one_based_position_is_valid_and_preserves_the_complete_count() {
    let source = prepared("<p>A</p><p>B</p><p>C</p>");
    let mut plan = ExtractionPlan::css("p").unwrap();
    plan.match_mode = Match::Nth;
    plan.index = Some(3);
    let result = source
        .execute(&CompiledPlan::compile(&plan).unwrap())
        .unwrap();
    assert_eq!(result.data().as_values().unwrap(), ["C"]);
    assert_eq!(result.candidate_count(), 3);
}
