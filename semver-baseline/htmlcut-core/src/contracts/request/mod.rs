mod extraction;
mod options;
mod primitives;
mod requests;

pub use extraction::{ExtractionSpec, SelectionSpec, SlicePatternSpec, SliceSpec, ValueSpec};
pub use options::{InspectionOptions, NormalizationOptions, OutputOptions, RuntimeOptions};
pub use primitives::{
    AttributeName, ContractValueError, ExtractionStrategy, FetchPreflightMode, PatternMode,
    SelectorQuery, SliceBoundary, SourceKind, ValueType, WhitespaceMode,
};
pub use requests::{ExtractionDefinition, ExtractionRequest, SourceInput, SourceRequest};
