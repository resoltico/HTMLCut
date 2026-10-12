// SPDX-License-Identifier: MPL-2.0
//! Operator policy and contract grammar admission.
/// Operator policy, independent of contract data.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct Limits {
    /// Maximum input bytes before decoding.
    pub max_input_bytes: usize,
    /// Maximum parsed tree nodes, admitted between chunks.
    pub max_nodes: usize,
    /// Maximum complete encoded JSON result bytes.
    pub max_output_bytes: usize,
    /// Cooperative time bound for each preparation or execution call.
    pub time_limit: Option<std::time::Duration>,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            max_input_bytes: 64 * 1024 * 1024,
            max_nodes: 2_000_000,
            max_output_bytes: 64 * 1024 * 1024,
            time_limit: Some(std::time::Duration::from_secs(10)),
        }
    }
}

pub(crate) struct Clock {
    started: std::time::Instant,
    limit: Option<std::time::Duration>,
    iterations: std::cell::Cell<usize>,
}
impl Clock {
    pub(crate) fn new(limits: &Limits) -> Self {
        Self {
            started: std::time::Instant::now(),
            limit: limits.time_limit,
            iterations: std::cell::Cell::new(0),
        }
    }
    pub(crate) fn check(&self) -> Result<(), crate::Error> {
        if let Some(limit) = self.limit {
            if self.started.elapsed() >= limit {
                return Err(crate::Error::limit("time", limit.as_secs_f64()));
            }
        }
        Ok(())
    }
    pub(crate) fn tick(&self) -> Result<(), crate::Error> {
        let count = self.iterations.get().wrapping_add(1);
        self.iterations.set(count);
        if count & 1023 == 0 {
            self.check()?;
        }
        Ok(())
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
