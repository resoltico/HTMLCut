// SPDX-License-Identifier: MPL-2.0
//! Admission bounds independent of caller policy.
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
