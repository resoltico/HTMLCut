// SPDX-License-Identifier: MPL-2.0
//! Bounded selector-evaluation accounting at HTMLCut-owned traversal and predicate boundaries.

use std::cell::Cell;

/// Bounded work accounting for one selector evaluation.
///
/// Product traversal, selector navigation and predicate callbacks consume units.
/// Predicate payload bytes are additionally charged in batches of 64. Sticky
/// exhaustion makes callbacks refuse traversal and the matcher rejects the result,
/// including matches produced by negation. It does not count upstream instructions.
#[derive(Debug)]
pub struct WorkBudget {
    configured: u32,
    remaining: Cell<u32>,
    exhausted: Cell<bool>,
}

impl WorkBudget {
    /// Creates one positive selector work budget.
    pub fn new(units: u32) -> Self {
        assert!(units > 0, "selector work budget must be positive");
        Self {
            configured: units,
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

    /// Original configured allowance; consuming work never changes it.
    pub fn configured(&self) -> u32 {
        self.configured
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
    use super::WorkBudget;

    #[test]
    fn exhaustion_is_sticky_after_the_last_available_unit() {
        let budget = WorkBudget::new(2);
        assert_eq!(budget.configured(), 2);
        assert_eq!(budget.remaining(), 2);
        assert!(budget.consume());
        assert_eq!(budget.remaining(), 1);
        assert!(!budget.exhausted());
        assert!(budget.consume());
        assert_eq!(budget.remaining(), 0);
        assert!(!budget.consume());
        assert_eq!(budget.configured(), 2);
        assert!(budget.exhausted());
        assert!(!budget.consume());
        assert_eq!(budget.remaining(), 0);
    }

    #[test]
    #[should_panic(expected = "selector work budget must be positive")]
    fn zero_work_budget_is_rejected_at_construction() {
        let _ = WorkBudget::new(0);
    }
}
