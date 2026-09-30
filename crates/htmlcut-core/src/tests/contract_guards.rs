use super::*;

#[test]
fn t18_t19_guards_are_original_dom_conjunctive_and_all_values() {
    let source = prepared(
        "<h2 id='label'>Repair cost</h2><p id='amount' data-x=''>EUR 180<span class='remove'> RAW </span></p><b>good</b><b>bad</b>",
    );
    let mut plan = ExtractionPlan::css("#amount").unwrap();
    plan.exclude = vec![".remove".into()];
    plan.transforms = vec![Transform::NormalizeWhitespace];
    plan.guards.push(Guard {
        scope: GuardScope::Document,
        selector: "#label".into(),
        min: 1,
        max: Some(1),
        read: GuardRead::DomText,
        predicate: Some(Predicate::Exact {
            value: "Repair cost".into(),
        }),
    });
    plan.guards.push(Guard {
        scope: GuardScope::Selected,
        selector: ":scope .remove".into(),
        min: 1,
        max: Some(1),
        read: GuardRead::DomText,
        predicate: Some(Predicate::Exact {
            value: " RAW ".into(),
        }),
    });
    assert_eq!(
        source
            .execute(&CompiledPlan::compile(&plan).unwrap())
            .unwrap()
            .values,
        ["EUR 180"]
    );
    plan.guards[0].predicate = Some(Predicate::Exact {
        value: "Tax".into(),
    });
    assert_eq!(
        source
            .execute(&CompiledPlan::compile(&plan).unwrap())
            .unwrap_err()
            .code,
        ErrorCode::GuardFailed
    );
    plan.guards[0].selector = "b".into();
    plan.guards[0].max = Some(2);
    plan.guards[0].predicate = Some(Predicate::Regex {
        pattern: "good".into(),
        flags: "".into(),
    });
    assert_eq!(
        source
            .execute(&CompiledPlan::compile(&plan).unwrap())
            .unwrap_err()
            .code,
        ErrorCode::GuardFailed
    );
    plan.guards.clear();
    plan.guards.push(Guard {
        scope: GuardScope::Selected,
        selector: ":scope".into(),
        min: 1,
        max: Some(1),
        read: GuardRead::Attribute {
            name: "data-x".into(),
        },
        predicate: None,
    });
    assert!(
        source
            .execute(&CompiledPlan::compile(&plan).unwrap())
            .is_ok()
    );
    plan.guards[0].read = GuardRead::Attribute {
        name: "absent".into(),
    };
    assert_eq!(
        source
            .execute(&CompiledPlan::compile(&plan).unwrap())
            .unwrap_err()
            .code,
        ErrorCode::MissingAttribute
    );
}

#[test]
fn t16_t17_complete_counts_positions_and_explicit_empty_selection() {
    let source = prepared("<p>A</p><p>B</p><p>C</p>");
    let mut plan = ExtractionPlan::css("p, p").unwrap();
    plan.selection = Selection::Nth { index: 2 };
    let result = source
        .execute(&CompiledPlan::compile(&plan).unwrap())
        .unwrap();
    assert_eq!(result.values, ["B"]);
    assert_eq!((result.candidate_count, result.selected_count), (3, 1));
    plan.limits.max_candidates = 2;
    assert_eq!(
        source
            .execute(&CompiledPlan::compile(&plan).unwrap())
            .unwrap_err()
            .code,
        ErrorCode::ResourceLimit
    );
    plan = ExtractionPlan::css("aside").unwrap();
    plan.selection = Selection::All {
        min: 0,
        max: Some(0),
    };
    assert!(
        source
            .execute(&CompiledPlan::compile(&plan).unwrap())
            .unwrap()
            .values
            .is_empty()
    );
    plan.selection = Selection::All { min: 1, max: None };
    assert_eq!(
        source
            .execute(&CompiledPlan::compile(&plan).unwrap())
            .unwrap_err()
            .code,
        ErrorCode::Cardinality
    );
    plan.selection = Selection::Nth { index: 0 };
    assert_eq!(
        CompiledPlan::compile(&plan).err().unwrap().code,
        ErrorCode::InvalidPlan
    );
}

#[test]
fn t13_t15_all_slice_inclusions_progress_and_unmatched_tail() {
    let mut plan = ExtractionPlan::css("p").unwrap();
    plan.projection = Projection::Source;
    for (start, end, expected) in [
        (false, false, "✓"),
        (true, false, "[✓"),
        (false, true, "✓]"),
        (true, true, "[✓]"),
    ] {
        plan.strategy = Strategy::Slice {
            start: Boundary::Literal { value: "[".into() },
            end: Boundary::Literal { value: "]".into() },
            include_start: start,
            include_end: end,
        };
        assert_eq!(
            prepared("[✓]")
                .execute(&CompiledPlan::compile(&plan).unwrap())
                .unwrap()
                .values,
            [expected]
        );
    }
    assert_eq!(
        prepared("[✓][tail")
            .execute(&CompiledPlan::compile(&plan).unwrap())
            .unwrap_err()
            .code,
        ErrorCode::MissingBoundary
    );
    plan.strategy = Strategy::Slice {
        start: Boundary::Regex {
            pattern: "^".into(),
            flags: "".into(),
        },
        end: Boundary::Literal { value: "]".into() },
        include_start: false,
        include_end: false,
    };
    assert_eq!(
        prepared("[✓]")
            .execute(&CompiledPlan::compile(&plan).unwrap())
            .unwrap_err()
            .code,
        ErrorCode::EmptyBoundaryMatch
    );
    plan.strategy = Strategy::Slice {
        start: Boundary::Literal { value: "".into() },
        end: Boundary::Literal { value: "]".into() },
        include_start: false,
        include_end: false,
    };
    assert!(CompiledPlan::compile(&plan).is_err());
}

#[test]
fn t18_guard_cardinality_scope_and_search_predicate_are_declared() {
    let source = prepared("<p id='root'><span>prefix VALUE suffix</span></p><span>outside</span>");
    let mut plan = ExtractionPlan::css("#root").unwrap();
    plan.guards = vec![Guard {
        scope: GuardScope::Selected,
        selector: "span".into(),
        min: 1,
        max: Some(1),
        read: GuardRead::DomText,
        predicate: Some(Predicate::Regex {
            pattern: "VALUE".into(),
            flags: String::new(),
        }),
    }];
    assert!(
        source
            .execute(&CompiledPlan::compile(&plan).unwrap())
            .is_ok()
    );
    plan.guards[0].selector = "aside".into();
    assert_eq!(
        source
            .execute(&CompiledPlan::compile(&plan).unwrap())
            .unwrap_err()
            .code,
        ErrorCode::GuardFailed
    );
    plan.guards[0].min = 0;
    assert!(
        source
            .execute(&CompiledPlan::compile(&plan).unwrap())
            .is_ok()
    );
    plan.guards[0].scope = GuardScope::Document;
    plan.guards[0].selector = "span".into();
    plan.guards[0].min = 1;
    assert_eq!(
        source
            .execute(&CompiledPlan::compile(&plan).unwrap())
            .unwrap_err()
            .code,
        ErrorCode::GuardFailed
    );
    plan.guards.clear();
    plan.selection = Selection::Nth { index: 2 };
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
    plan.selection = Selection::All {
        min: 1,
        max: Some(1),
    };
    let error = source
        .execute(&CompiledPlan::compile(&plan).unwrap())
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::Cardinality);
    assert_eq!(error.candidate_count, Some(2));
    let mut plan = ExtractionPlan::css("p:first-of-type").unwrap();
    plan.limits.max_candidates = 1;
    plan.guards.push(Guard {
        scope: GuardScope::Document,
        selector: "b".into(),
        min: 0,
        max: Some(1),
        read: GuardRead::DomText,
        predicate: Some(Predicate::Exact { value: "X".into() }),
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
    plan.selection = Selection::Nth { index: 3 };
    let result = source
        .execute(&CompiledPlan::compile(&plan).unwrap())
        .unwrap();
    assert_eq!(result.values, ["C"]);
    assert_eq!(result.candidate_count, 3);
}
