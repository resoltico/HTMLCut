// SPDX-License-Identifier: MPL-2.0
//! Operator policy and contract grammar admission.
/// Operator policy, independent of contract data.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct Limits {
    /// Maximum input bytes before decoding.
    pub max_input_bytes: usize,
    /// Maximum retained tree nodes, admitted before allocation.
    pub max_nodes: usize,
    /// Maximum complete canonical export bytes, including final LF.
    pub max_output_bytes: usize,
    /// Maximum UTF-8 bytes in each transformed source reading, including predicate/capture sources.
    pub max_reading_bytes: usize,
    /// Default duration used when creating a host job; documents never retain it.
    pub time_limit: Option<std::time::Duration>,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            max_input_bytes: 64 * 1024 * 1024,
            max_nodes: 2_000_000,
            max_output_bytes: 64 * 1024 * 1024,
            max_reading_bytes: 64 * 1024 * 1024,
            time_limit: Some(std::time::Duration::from_secs(10)),
        }
    }
}

/// Limits applied only while admitting a document.
#[derive(Clone, Debug)]
pub struct PreparationPolicy {
    /// Maximum source byte length.
    pub max_input_bytes: usize,
    /// Maximum retained tree nodes.
    pub max_nodes: usize,
}
/// Limits applied independently to each prepared-document operation.
#[derive(Clone, Debug)]
pub struct ExecutionPolicy {
    /// Complete canonical export ceiling, including framing LF.
    pub max_output_bytes: usize,
    /// Transformed source reading ceiling.
    pub max_reading_bytes: usize,
}
impl Limits {
    /// The preparation subset of this application configuration.
    pub fn preparation(&self) -> PreparationPolicy {
        PreparationPolicy {
            max_input_bytes: self.max_input_bytes,
            max_nodes: self.max_nodes,
        }
    }
    /// The execution subset of this application configuration.
    pub fn execution(&self) -> ExecutionPolicy {
        ExecutionPolicy {
            max_output_bytes: self.max_output_bytes,
            max_reading_bytes: self.max_reading_bytes,
        }
    }
}

/// Cooperative checks shared by every phase of one operation.
#[derive(Clone)]
pub struct ExecutionContext {
    checkpoint: std::sync::Arc<dyn Fn() -> Result<(), crate::Error> + Send + Sync>,
    iterations: std::rc::Rc<std::cell::Cell<usize>>,
}
impl ExecutionContext {
    /// Creates one checked absolute deadline. No phase restarts it.
    pub fn new(limit: Option<std::time::Duration>) -> Result<Self, crate::Error> {
        let started = std::time::Instant::now();
        let deadline = limit
            .map(|limit| {
                started
                    .checked_add(limit)
                    .ok_or_else(|| crate::Error::InvalidRequest {
                        reason: ("The deadline cannot be represented.").into(),
                    })
            })
            .transpose()?;
        Ok(Self::with_checkpoint(move || {
            if deadline.is_some_and(|deadline| std::time::Instant::now() >= deadline) {
                return Err(crate::Error::LimitExceeded {
                    limit: crate::Limit::Time,
                    bound: serde_json::Number::from_f64(limit.unwrap().as_secs_f64()).unwrap(),
                });
            }
            Ok(())
        }))
    }
    /// Uses the host job's stop/commit arbiter without resetting its deadline.
    pub fn with_checkpoint(
        checkpoint: impl Fn() -> Result<(), crate::Error> + Send + Sync + 'static,
    ) -> Self {
        Self {
            checkpoint: std::sync::Arc::new(checkpoint),
            iterations: std::rc::Rc::new(std::cell::Cell::new(0)),
        }
    }
    /// Checks cancellation and the absolute deadline immediately.
    pub fn check(&self) -> Result<(), crate::Error> {
        (self.checkpoint)()
    }
    pub(crate) fn tick(&self) -> Result<(), crate::Error> {
        let count = self.iterations.get().wrapping_add(1);
        self.iterations.set(count);
        if count & 1023 == 0 {
            self.check()?;
        }
        Ok(())
    }
    #[cfg(test)]
    pub(crate) fn work_ticks(&self) -> usize {
        self.iterations.get()
    }
}
// Contract grammar bounds; schema copies derive from these authorities.
pub(crate) const MAX_CONTRACT_BYTES: usize = 256 * 1024;
pub(crate) const MAX_CONTRACT_NODES: usize = 512;
pub(crate) const MAX_RECORD_DEPTH: usize = 8;
pub(crate) const MAX_ENTRIES: usize = 32;
pub(crate) const MAX_DECLARED_COUNT: usize = 1_000_000;
pub(crate) const MAX_ATTRIBUTE_NAME_CHARS: usize = 256;
pub(crate) const MAX_FIELD_NAME_CHARS: usize = 64;

pub(crate) const MAX_PATTERN_BYTES: usize = 8 * 1024;
pub(crate) const MAX_PATTERN_DEPTH: u32 = 64;
pub(crate) const MAX_REGEX_BYTES: usize = 8 * 1024 * 1024;
pub(crate) const MAX_JSON_DEPTH: usize = 64;
pub(crate) const MAX_RECORD_FIELDS: usize = 64;
pub(crate) const MAX_SELECTOR_MATCHING_DEPTH: usize = 64;

pub(crate) const MAX_DOM_DEPTH: usize = 1_024;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn context_deadline_is_absolute_and_overflow_is_a_request_error() {
        let context = ExecutionContext::new(Some(std::time::Duration::ZERO)).unwrap();
        assert_eq!(
            serde_json::to_value(context.check().unwrap_err()).unwrap()["limit"].as_str(),
            Some("time")
        );
        assert!(ExecutionContext::new(Some(std::time::Duration::MAX)).is_err());
        let clock = context.clone();
        assert!((0..1024).any(|_| clock.tick().is_err()));
    }
}
