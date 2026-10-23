// SPDX-License-Identifier: MPL-2.0
//! Operator policy and contract grammar admission.
/// Operator policy, independent of contract data.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct Settings {
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
impl Default for Settings {
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

impl Settings {
    /// The preparation subset of this application configuration.
    pub fn preparation(&self) -> htmlcut_core::ParseOptions {
        htmlcut_core::ParseOptions {
            max_input_bytes: self.max_input_bytes,
            max_nodes: self.max_nodes,
            ..Default::default()
        }
    }
    /// The execution subset of this application configuration.
    pub fn execution(&self) -> htmlcut_core::ResultLimits {
        htmlcut_core::ResultLimits {
            max_output_bytes: self.max_output_bytes,
            max_reading_bytes: self.max_reading_bytes,
        }
    }
}
