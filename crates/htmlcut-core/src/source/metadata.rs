use crate::contracts::{
    Diagnostic, SourceInput, SourceKind, SourceLoadStep, SourceMetadata, SourceRequest,
};

use super::{LoadedSource, SourceLoadFailure};

pub(crate) fn source_metadata(
    source: &LoadedSource,
    include_text: bool,
    effective_base_url: Option<String>,
) -> SourceMetadata {
    SourceMetadata {
        kind: source.kind,
        value: source.value.clone(),
        input_base_url: source.input_base_url.clone(),
        effective_base_url,
        bytes_read: source.bytes_read,
        load_steps: source.load_steps.clone(),
        text: include_text.then_some(source.text.clone()),
    }
}

pub(super) fn source_load_failure(
    source: &SourceRequest,
    kind: SourceKind,
    value: String,
    load_steps: Vec<SourceLoadStep>,
    diagnostic: Diagnostic,
) -> SourceLoadFailure {
    let input_base_url = source
        .base_url
        .as_ref()
        .map(ToString::to_string)
        .or_else(|| matches!(kind, SourceKind::Url).then(|| value.clone()));

    SourceLoadFailure {
        metadata: Box::new(SourceMetadata {
            kind,
            value,
            input_base_url: input_base_url.clone(),
            effective_base_url: input_base_url,
            bytes_read: 0,
            load_steps,
            text: None,
        }),
        diagnostic,
    }
}

pub(crate) fn empty_source_metadata(source: &SourceRequest) -> SourceMetadata {
    let kind = source.kind();
    let value = source_locator_value(&source.input);
    let input_base_url = source
        .base_url
        .as_ref()
        .map(ToString::to_string)
        .or_else(|| matches!(source.input, SourceInput::Url { .. }).then(|| value.clone()));
    SourceMetadata {
        kind,
        value,
        input_base_url: input_base_url.clone(),
        effective_base_url: input_base_url,
        bytes_read: 0,
        load_steps: Vec::new(),
        text: None,
    }
}

fn source_locator_value(input: &SourceInput) -> String {
    match input {
        SourceInput::Url { href } => href.to_string(),
        SourceInput::File { path } => path.to_string_lossy().into_owned(),
        SourceInput::Stdin => "-".to_owned(),
        SourceInput::Memory { label, .. } => memory_label(label),
    }
}

pub(crate) fn memory_label(label: &str) -> String {
    if label.trim().is_empty() {
        "memory".to_owned()
    } else {
        label.to_owned()
    }
}
