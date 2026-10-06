// SPDX-License-Identifier: MPL-2.0
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
fn regex_guards_debit_a_search_even_for_an_empty_value() {
    let document = Html::parse_document("<p></p>");
    let root = document
        .select(&Selector::parse("p").unwrap())
        .next()
        .unwrap();
    let mut plan = crate::ExtractionPlan::css("p").unwrap();
    plan.expect.push(crate::Guard {
        scope: GuardScope::Selected,
        select: ":scope".into(),
        min: 1,
        max: 1,
        read: Some(crate::Reading::Literal),
        equals: Some(String::new()),
        pattern: None,
    });
    let exact = CompiledPlan::compile(&plan).unwrap();
    let exact_budget = SelectorWorkBudget::new(1000);
    check_guards(
        &document,
        &[SelectionScope {
            anchor: root,
            following_siblings: vec![],
        }],
        &exact,
        &exact_budget,
    )
    .unwrap();
    plan.expect[0].equals = None;
    plan.expect[0].pattern = Some("^$".into());
    let regex = CompiledPlan::compile(&plan).unwrap();
    let regex_budget = SelectorWorkBudget::new(1000);
    check_guards(
        &document,
        &[SelectionScope {
            anchor: root,
            following_siblings: vec![],
        }],
        &regex,
        &regex_budget,
    )
    .unwrap();
    assert_eq!(exact_budget.remaining() - regex_budget.remaining(), 1);
}

#[test]
fn exclusion_scope_from_another_document_is_an_invariant_refusal() {
    let first = Html::parse_document("<article><p>A</p></article>");
    let second = Html::parse_document("<article><p>B</p></article>");
    let root = second
        .select(&Selector::parse("article").unwrap())
        .next()
        .unwrap();
    let budget = SelectorWorkBudget::new(1000);
    let error = exclusions(&first, root, &[Selector::parse("p").unwrap()], &budget).unwrap_err();
    assert_eq!(error.code, ErrorCode::InternalInvariant);
    assert_eq!(error.stage, "exclusion");
    assert_eq!(budget.remaining(), 1000);
}
