// SPDX-License-Identifier: MPL-2.0
//! Per-parse options and per-result resource limits.
/// Options applied while admitting a document.
#[derive(Clone, Debug)]
pub struct ParseOptions {
    /// Explicit WHATWG encoding label, overridden by a BOM.
    pub encoding: Option<String>,
    /// Fallback document base URL.
    pub base_url: Option<String>,
    /// Maximum source bytes before decoding.
    pub max_input_bytes: usize,
    /// Maximum retained tree nodes.
    pub max_nodes: usize,
}
impl Default for ParseOptions {
    fn default() -> Self {
        Self {
            encoding: None,
            base_url: None,
            max_input_bytes: 64 * 1024 * 1024,
            max_nodes: 2_000_000,
        }
    }
}
/// Limits independently applied to each result.
#[derive(Clone, Debug)]
pub struct ResultLimits {
    /// Maximum UTF-8 bytes per transformed source reading.
    pub max_reading_bytes: usize,
    /// Maximum emitted bytes in the requested representation.
    pub max_output_bytes: usize,
}
impl Default for ResultLimits {
    fn default() -> Self {
        Self {
            max_reading_bytes: 64 * 1024 * 1024,
            max_output_bytes: 64 * 1024 * 1024,
        }
    }
}
