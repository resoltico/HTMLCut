use super::*;

#[test]
fn work_debits_are_exact_and_failed_debits_do_not_consume_partial_capacity() {
    let budget = SelectorWorkBudget::new(3);
    charge(&budget, 0).unwrap();
    assert_eq!(budget.remaining(), 3);
    charge(&budget, 2).unwrap();
    assert_eq!(budget.remaining(), 1);
    assert_eq!(
        charge(&budget, 2).unwrap_err().code,
        crate::ErrorCode::ResourceLimit
    );
    assert_eq!(budget.remaining(), 1);
    charge(&budget, 1).unwrap();
    assert_eq!(budget.remaining(), 0);
    assert_eq!(
        charge(&budget, 1).unwrap_err().code,
        crate::ErrorCode::ResourceLimit
    );
}

#[test]
fn boundary_search_debits_remaining_bytes_and_one_unit_even_for_an_empty_tail() {
    let matcher = Matcher::Literal("!".into());
    let source = format!("{}!", "x".repeat(64));
    let budget = SelectorWorkBudget::new(6);
    assert_eq!(find(&matcher, &source, 0, &budget).unwrap(), Some((64, 65)));
    assert_eq!(budget.remaining(), 3); // ceil(65/64) + one search.
    assert_eq!(
        find(&matcher, &source, 64, &budget).unwrap(),
        Some((64, 65))
    );
    assert_eq!(budget.remaining(), 1); // ceil(1/64) + one search.
    assert_eq!(find(&matcher, &source, 65, &budget).unwrap(), None);
    assert_eq!(budget.remaining(), 0);
    assert_eq!(
        find(&matcher, &source, 65, &budget).unwrap_err().code,
        crate::ErrorCode::ResourceLimit
    );
}

#[test]
fn regex_guards_debit_a_search_even_for_an_empty_value() {
    let document = Html::parse_document("<p></p>");
    let root = document
        .select(&Selector::parse("p").unwrap())
        .next()
        .unwrap();
    let mut plan = crate::ExtractionPlan::css("p").unwrap();
    plan.guards.push(crate::Guard {
        scope: GuardScope::Selected,
        selector: ":scope".into(),
        min: 1,
        max: Some(1),
        read: GuardRead::DomText,
        predicate: Some(Predicate::Exact {
            value: String::new(),
        }),
    });
    let exact = CompiledPlan::compile(&plan).unwrap();
    let exact_budget = SelectorWorkBudget::new(1000);
    check_guards(&document, &[root], &exact, &exact_budget).unwrap();
    plan.guards[0].predicate = Some(Predicate::Regex {
        pattern: "^$".into(),
        flags: String::new(),
    });
    let regex = CompiledPlan::compile(&plan).unwrap();
    let regex_budget = SelectorWorkBudget::new(1000);
    check_guards(&document, &[root], &regex, &regex_budget).unwrap();
    assert_eq!(exact_budget.remaining() - regex_budget.remaining(), 1);
}
