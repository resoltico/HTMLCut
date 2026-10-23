// SPDX-License-Identifier: MPL-2.0
//! Deterministic extraction contracts for caller-owned HTML documents.
//!
//! ```
//! use htmlcut_core::{Contract, Document, ParseOptions, ResultLimits, Stop, ExportFormat, Value};
//! let stop = Stop::none();
//! let document = Document::parse(b"<h1>Snapshot</h1>", &ParseOptions::default(), &stop)?;
//! let contract = Contract::from_json(r#""h1""#, &stop)?;
//! assert_eq!(document.extract(&contract, ExportFormat::Json, &ResultLimits::default(), &stop)?, Value::String("Snapshot".into()));
//! # Ok::<(), htmlcut_core::Error>(())
//! ```
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
#[path = "../tests/support/mod.rs"]
mod test_support;

#[cfg(test)]
pub(crate) use test_support::TestOptions;
