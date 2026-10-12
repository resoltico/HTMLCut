// SPDX-License-Identifier: MPL-2.0
use super::*;

#[test]
fn work_debits_are_exact_and_failed_debits_do_not_consume_partial_capacity() {
    let budget = WorkBudget::new(3);
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
fn exclusion_scope_from_another_document_is_an_invariant_refusal() {
    let first = Html::parse_document("<article><p>A</p></article>");
    let second = Html::parse_document("<article><p>B</p></article>");
    let root = second
        .select(&Selector::parse("article").unwrap())
        .next()
        .unwrap();
    let budget = WorkBudget::new(1000);
    let error = exclusions(&first, root, &[Selector::parse("p").unwrap()], &budget).unwrap_err();
    assert_eq!(error.code, ErrorCode::InternalInvariant);
    assert_eq!(error.stage, "exclusion");
    assert_eq!(budget.remaining(), 1000);
}
