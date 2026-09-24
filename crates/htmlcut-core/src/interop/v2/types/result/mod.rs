//! Versioned result and error documents for the `htmlcut-v2` interop contract.

mod diagnostics;
mod error;
mod matches;
mod success;

pub use diagnostics::{InteropDiagnostic, InteropDiagnosticCode, InteropDiagnosticLevel};
pub use error::{
    ErrorCode, InteropAdapterFailure, InteropError, InteropErrorDetail,
    InteropFinalizationRejection,
};
pub use matches::{ByteRange, ResultExecution, ResultSource, SelectedMatch, SelectedMatchMetadata};
pub use success::InteropResult;
#[cfg(test)]
pub(crate) use success::selected_match_count_for_tests;
