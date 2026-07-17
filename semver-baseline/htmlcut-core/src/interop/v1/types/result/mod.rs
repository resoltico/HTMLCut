//! Versioned result and error documents for the `htmlcut-v1` interop contract.

mod diagnostics;
mod error;
mod matches;
mod success;

pub use diagnostics::{InteropDiagnostic, InteropDiagnosticCode, InteropDiagnosticLevel};
pub use error::{ErrorCode, InteropError};
pub use matches::{ByteRange, ResultExecution, ResultSource, SelectedMatch, SelectedMatchMetadata};
pub use success::InteropResult;
