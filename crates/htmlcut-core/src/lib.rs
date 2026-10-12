// SPDX-License-Identifier: MPL-2.0
//! Deterministic extraction contracts for caller-owned HTML documents.
//!
//! ```
//! use htmlcut_core::{Contract, Document, Limits, Source, Value};
//! let document = Document::parse(Source::new(b"<h1>Snapshot</h1>"), &Limits::default())?;
//! let contract = Contract::from_json(r#""h1""#)?;
//! assert_eq!(document.extract(&contract)?, Value::String("Snapshot".into()));
//! # Ok::<(), htmlcut_core::Error>(())
//! ```
#![forbid(unsafe_code)]
#![deny(missing_docs)]
mod contract;
mod document;
mod error;
mod eval;
mod json;
mod limits;
mod outline;
mod read;
mod select;
mod value;
pub use contract::Contract;
pub use document::{Document, Source};
pub use error::{Error, ErrorCode, ErrorFacts};
pub use limits::Limits;
pub use outline::OutlineOptions;
pub use value::Value;
