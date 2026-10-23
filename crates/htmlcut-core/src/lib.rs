// SPDX-License-Identifier: MPL-2.0
#![doc = include_str!("../README.md")]
#![deny(missing_docs)]
mod bounds;
mod compile;
mod css;
mod document;
mod dom;
mod error;
mod eval;
mod json;
mod options;
mod outline;
mod read;
mod select;
mod spec;
mod stop;
mod value;
pub use compile::Contract;
pub use document::Document;
pub use error::{CountRange, Error, ErrorClass, ErrorCode, ExpectationFailure, Limit};
pub use options::{ParseOptions, ResultLimits};
pub use outline::{OutlineAttribute, OutlineOptions, OutlineReport, OutlineRow};
pub use spec::{
    CONTRACT_SUMMARY, Match, NodeObject, NodeSpec, PredicateObject, PredicateSpec, Reading,
};
pub use stop::Stop;
pub use value::{ExportFormat, Value};

#[cfg(test)]
extern crate self as htmlcut_core;
#[cfg(test)]
mod test_support;

#[cfg(test)]
pub(crate) use test_support::TestOptions;
