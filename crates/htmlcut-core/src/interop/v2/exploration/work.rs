//! Shared request-local accounting for DOM work outside selector matching.

use selectors::work_budget::SelectorWorkBudget;

/// Indicates that a request exhausted its shared exploration work budget.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::interop::v2) struct WorkLimitExceeded;

/// Charges one inspected DOM element to the same budget used by selector matching.
pub(super) fn inspect_element(budget: &SelectorWorkBudget) -> Result<(), WorkLimitExceeded> {
    budget.consume().then_some(()).ok_or(WorkLimitExceeded)
}
