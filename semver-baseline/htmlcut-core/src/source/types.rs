use std::ops::Deref;

use crate::contracts::{Diagnostic, SourceKind, SourceLoadStep, SourceMetadata};

#[derive(Clone, Debug)]
pub(crate) struct LoadedSource {
    pub(crate) kind: SourceKind,
    pub(crate) value: String,
    pub(crate) text: String,
    pub(crate) bytes_read: usize,
    pub(crate) input_base_url: Option<String>,
    pub(crate) load_steps: Vec<SourceLoadStep>,
}

#[derive(Clone, Debug)]
pub(crate) struct SourceLoadFailure {
    pub(crate) metadata: Box<SourceMetadata>,
    pub(crate) diagnostic: Diagnostic,
}

impl Deref for SourceLoadFailure {
    type Target = Diagnostic;

    fn deref(&self) -> &Self::Target {
        &self.diagnostic
    }
}

impl SourceLoadFailure {
    pub(crate) fn into_parts(self) -> (SourceMetadata, Diagnostic) {
        (*self.metadata, self.diagnostic)
    }
}
