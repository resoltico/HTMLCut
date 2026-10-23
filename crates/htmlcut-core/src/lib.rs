// SPDX-License-Identifier: MPL-2.0
//! Deterministic extraction contracts for caller-owned HTML documents.
//!
//! ```
//! use htmlcut_core::{Contract, Document, Limits, ExecutionContext, Source, Value};
//! let limits = Limits::default();
//! let context = ExecutionContext::new(limits.time_limit)?;
//! let document = Document::parse(Source::new(b"<h1>Snapshot</h1>"), &limits.preparation(), &context)?;
//! let contract = Contract::from_json(r#""h1""#, &context)?;
//! assert_eq!(document.extract(&contract, &limits.execution(), &context)?, Value::String("Snapshot".into()));
//! # Ok::<(), htmlcut_core::Error>(())
//! ```
#![forbid(unsafe_code)]
#![deny(missing_docs)]
mod contract;
mod css;
mod document;
mod dom;
mod error;
mod eval;
mod json;
mod limits;
mod outline;
mod read;
mod select;
mod value;
pub use contract::{Contract, QueryCardinality, QuickQuery};
pub use document::{Document, Source};
pub use error::{CountRange, Error, ErrorClass, ErrorCode, ExpectationFailure, Limit};
pub use limits::{ExecutionContext, ExecutionPolicy, Limits, PreparationPolicy};
pub use outline::{OutlineAttribute, OutlineOptions, OutlineReport, OutlineRow};
pub use value::{ExportFormat, Value};

/// Default reading syntax for value contracts and frontend overrides.
pub const DEFAULT_READING: &str = "text";
