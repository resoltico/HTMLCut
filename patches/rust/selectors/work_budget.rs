//! Bounded selector-evaluation accounting owned by HTMLCut's maintained selector fork.

use std::cell::Cell;

/// Bounded work accounting for one selector evaluation.
///
/// A matching step consumes one unit at every complex-selector entry, including recursive
/// relative-selector evaluation. Once exhausted, matching short-circuits and the caller must
/// report a typed budget failure rather than treating the selector as non-matching.
pub struct SelectorWorkBudget {
    remaining: Cell<u32>,
    exhausted: Cell<bool>,
}

impl SelectorWorkBudget {
    /// Creates one positive selector work budget.
    pub fn new(units: u32) -> Self {
        assert!(units > 0, "selector work budget must be positive");
        Self {
            remaining: Cell::new(units),
            exhausted: Cell::new(false),
        }
    }

    /// Consumes one matching work unit, returning false after exhaustion.
    pub fn consume(&self) -> bool {
        let remaining = self.remaining.get();
        if remaining == 0 {
            self.exhausted.set(true);
            return false;
        }
        self.remaining.set(remaining - 1);
        true
    }

    /// Returns whether matching exhausted this budget.
    pub fn exhausted(&self) -> bool {
        self.exhausted.get()
    }

    /// Returns the number of work units still available to the caller.
    pub fn remaining(&self) -> u32 {
        self.remaining.get()
    }
}

#[cfg(test)]
mod tests {
    use super::SelectorWorkBudget;

    #[test]
    fn exhaustion_is_sticky_after_the_last_available_unit() {
        let budget = SelectorWorkBudget::new(2);
        assert_eq!(budget.remaining(), 2);
        assert!(budget.consume());
        assert_eq!(budget.remaining(), 1);
        assert!(!budget.exhausted());
        assert!(budget.consume());
        assert_eq!(budget.remaining(), 0);
        assert!(!budget.consume());
        assert!(budget.exhausted());
        assert!(!budget.consume());
        assert_eq!(budget.remaining(), 0);
    }

    #[test]
    #[should_panic(expected = "selector work budget must be positive")]
    fn zero_work_budget_is_rejected_at_construction() {
        let _ = SelectorWorkBudget::new(0);
    }
}
