use super::*;
use std::num::NonZeroUsize;

impl TryFrom<SourceRequest> for SourceRequestDocument {
    type Error = ContractValueError;

    fn try_from(value: SourceRequest) -> Result<Self, Self::Error> {
        Ok(Self {
            envelope: WireEnvelope::new(
                crate::SOURCE_REQUEST_SCHEMA_NAME,
                crate::CORE_REQUEST_SCHEMA_VERSION,
            ),
            input: value.input.try_into()?,
            base_url: value.base_url.map(TryInto::try_into).transpose()?,
        })
    }
}

impl TryFrom<SourceRequestDocument> for SourceRequest {
    type Error = ContractValueError;

    fn try_from(value: SourceRequestDocument) -> Result<Self, Self::Error> {
        value.envelope.validate(
            crate::SOURCE_REQUEST_SCHEMA_NAME,
            crate::CORE_REQUEST_SCHEMA_VERSION,
        )?;
        Ok(Self {
            input: value.input.into(),
            base_url: value.base_url.map(Into::into),
        })
    }
}

impl TryFrom<RuntimeOptions> for RuntimeOptionsDocument {
    type Error = ContractValueError;

    fn try_from(value: RuntimeOptions) -> Result<Self, Self::Error> {
        Ok(Self {
            envelope: WireEnvelope::new(
                crate::RUNTIME_OPTIONS_SCHEMA_NAME,
                crate::CORE_REQUEST_SCHEMA_VERSION,
            ),
            max_bytes: wire_u64(value.max_bytes.get(), "max_bytes")?,
            fetch_timeout_ms: value.fetch_timeout_ms,
            fetch_connect_timeout_ms: value.fetch_connect_timeout_ms,
            fetch_preflight: value.fetch_preflight,
            tls_trust: value.tls_trust.into(),
        })
    }
}

impl TryFrom<RuntimeOptionsDocument> for RuntimeOptions {
    type Error = ContractValueError;

    fn try_from(value: RuntimeOptionsDocument) -> Result<Self, Self::Error> {
        value.envelope.validate(
            crate::RUNTIME_OPTIONS_SCHEMA_NAME,
            crate::CORE_REQUEST_SCHEMA_VERSION,
        )?;
        Ok(Self {
            max_bytes: MaxBytes::new(native_usize_from_u64(value.max_bytes, "max_bytes")?)?,
            fetch_timeout_ms: value.fetch_timeout_ms,
            fetch_connect_timeout_ms: value.fetch_connect_timeout_ms,
            fetch_preflight: value.fetch_preflight,
            tls_trust: value.tls_trust.into(),
        })
    }
}

impl TryFrom<InspectionOptions> for InspectionOptionsDocument {
    type Error = ContractValueError;

    fn try_from(value: InspectionOptions) -> Result<Self, Self::Error> {
        Ok(Self {
            envelope: WireEnvelope::new(
                crate::INSPECTION_OPTIONS_SCHEMA_NAME,
                crate::CORE_REQUEST_SCHEMA_VERSION,
            ),
            include_source_text: value.include_source_text,
            sample_limit: wire_u32(value.sample_limit, "sample_limit")?,
        })
    }
}

impl TryFrom<InspectionOptionsDocument> for InspectionOptions {
    type Error = ContractValueError;

    fn try_from(value: InspectionOptionsDocument) -> Result<Self, Self::Error> {
        value.envelope.validate(
            crate::INSPECTION_OPTIONS_SCHEMA_NAME,
            crate::CORE_REQUEST_SCHEMA_VERSION,
        )?;
        Ok(Self {
            include_source_text: value.include_source_text,
            sample_limit: native_usize(value.sample_limit, "sample_limit")?,
        })
    }
}

impl TryFrom<ExtractionRequest> for ExtractionRequestDocument {
    type Error = ContractValueError;

    fn try_from(value: ExtractionRequest) -> Result<Self, Self::Error> {
        Ok(Self {
            envelope: WireEnvelope::new(
                crate::EXTRACTION_REQUEST_SCHEMA_NAME,
                crate::CORE_REQUEST_SCHEMA_VERSION,
            ),
            spec_version: value.spec_version,
            source: value.source.try_into()?,
            extraction: value.extraction.try_into()?,
            output: value.output.try_into()?,
        })
    }
}

impl TryFrom<ExtractionRequestDocument> for ExtractionRequest {
    type Error = ContractValueError;

    fn try_from(value: ExtractionRequestDocument) -> Result<Self, Self::Error> {
        value.envelope.validate(
            crate::EXTRACTION_REQUEST_SCHEMA_NAME,
            crate::CORE_REQUEST_SCHEMA_VERSION,
        )?;
        Ok(Self {
            spec_version: value.spec_version,
            source: value.source.try_into()?,
            extraction: value.extraction.try_into()?,
            output: value.output.try_into()?,
        })
    }
}

impl TryFrom<ExtractionDefinition> for ExtractionDefinitionDocument {
    type Error = ContractValueError;

    fn try_from(value: ExtractionDefinition) -> Result<Self, Self::Error> {
        Ok(Self {
            schema_name: value.schema_name,
            schema_version: value.schema_version,
            wire_profile: crate::HTMLCUT_JSON_SCHEMA_PROFILE.to_owned(),
            request: value.request.try_into()?,
            runtime: value.runtime.try_into()?,
        })
    }
}

impl TryFrom<ExtractionDefinitionDocument> for ExtractionDefinition {
    type Error = ContractValueError;

    fn try_from(value: ExtractionDefinitionDocument) -> Result<Self, Self::Error> {
        validate_wire_identity(
            &value.schema_name,
            value.schema_version,
            &value.wire_profile,
            crate::EXTRACTION_DEFINITION_SCHEMA_NAME,
            crate::EXTRACTION_DEFINITION_SCHEMA_VERSION,
        )?;
        Ok(Self {
            schema_name: value.schema_name,
            schema_version: value.schema_version,
            request: value.request.try_into()?,
            runtime: value.runtime.try_into()?,
        })
    }
}

impl TryFrom<ExtractionResult> for ExtractionResultDocument {
    type Error = ContractValueError;

    fn try_from(value: ExtractionResult) -> Result<Self, Self::Error> {
        Ok(Self {
            operation_id: value.operation_id,
            schema_name: value.schema_name,
            schema_version: value.schema_version,
            wire_profile: crate::HTMLCUT_JSON_SCHEMA_PROFILE.to_owned(),
            ok: value.ok,
            source: value.source.try_into()?,
            document_title: value.document_title,
            extraction: value.extraction.try_into()?,
            stats: value.stats.try_into()?,
            matches: value
                .matches
                .into_iter()
                .map(TryInto::try_into)
                .collect::<Result<_, _>>()?,
            diagnostics: value.diagnostics.into_iter().map(Into::into).collect(),
        })
    }
}

impl TryFrom<ExtractionResultDocument> for ExtractionResult {
    type Error = ContractValueError;

    fn try_from(value: ExtractionResultDocument) -> Result<Self, Self::Error> {
        validate_wire_identity(
            &value.schema_name,
            value.schema_version,
            &value.wire_profile,
            crate::CORE_RESULT_SCHEMA_NAME,
            crate::CORE_RESULT_SCHEMA_VERSION,
        )?;
        Ok(Self {
            operation_id: value.operation_id,
            schema_name: value.schema_name,
            schema_version: value.schema_version,
            ok: value.ok,
            source: value.source.try_into()?,
            document_title: value.document_title,
            extraction: value.extraction.try_into()?,
            stats: value.stats.try_into()?,
            matches: value
                .matches
                .into_iter()
                .map(TryInto::try_into)
                .collect::<Result<_, _>>()?,
            diagnostics: value.diagnostics.into_iter().map(Into::into).collect(),
        })
    }
}

impl TryFrom<SourceInspectionResult> for SourceInspectionResultDocument {
    type Error = ContractValueError;

    fn try_from(value: SourceInspectionResult) -> Result<Self, Self::Error> {
        Ok(Self {
            operation_id: value.operation_id,
            schema_name: value.schema_name,
            schema_version: value.schema_version,
            wire_profile: crate::HTMLCUT_JSON_SCHEMA_PROFILE.to_owned(),
            ok: value.ok,
            source: value.source.try_into()?,
            document: value.document.map(TryInto::try_into).transpose()?,
            diagnostics: value.diagnostics.into_iter().map(Into::into).collect(),
        })
    }
}

impl TryFrom<SourceInspectionResultDocument> for SourceInspectionResult {
    type Error = ContractValueError;

    fn try_from(value: SourceInspectionResultDocument) -> Result<Self, Self::Error> {
        validate_wire_identity(
            &value.schema_name,
            value.schema_version,
            &value.wire_profile,
            crate::CORE_SOURCE_INSPECTION_SCHEMA_NAME,
            crate::CORE_SOURCE_INSPECTION_SCHEMA_VERSION,
        )?;
        Ok(Self {
            operation_id: value.operation_id,
            schema_name: value.schema_name,
            schema_version: value.schema_version,
            ok: value.ok,
            source: value.source.try_into()?,
            document: value.document.map(TryInto::try_into).transpose()?,
            diagnostics: value.diagnostics.into_iter().map(Into::into).collect(),
        })
    }
}

impl TryFrom<SourceInput> for SourceInputDocument {
    type Error = ContractValueError;

    fn try_from(value: SourceInput) -> Result<Self, Self::Error> {
        match value {
            SourceInput::Url { href } => Ok(Self::Url {
                href: href.try_into()?,
            }),
            SourceInput::File { path } => Ok(Self::File { path }),
            SourceInput::Stdin => Ok(Self::Stdin),
            SourceInput::Memory { label, text } => Ok(Self::Memory { label, text }),
        }
    }
}

impl From<SourceInputDocument> for SourceInput {
    fn from(value: SourceInputDocument) -> Self {
        match value {
            SourceInputDocument::Url { href } => Self::Url { href: href.into() },
            SourceInputDocument::File { path } => Self::File { path },
            SourceInputDocument::Stdin => Self::Stdin,
            SourceInputDocument::Memory { label, text } => Self::Memory { label, text },
        }
    }
}

impl From<SlicePatternSpec> for SlicePatternSpecDocument {
    fn from(value: SlicePatternSpec) -> Self {
        match value {
            SlicePatternSpec::Literal { from, to } => Self::Literal { from, to },
            SlicePatternSpec::Regex { from, to, flags } => Self::Regex { from, to, flags },
        }
    }
}

impl From<SlicePatternSpecDocument> for SlicePatternSpec {
    fn from(value: SlicePatternSpecDocument) -> Self {
        match value {
            SlicePatternSpecDocument::Literal { from, to } => Self::Literal { from, to },
            SlicePatternSpecDocument::Regex { from, to, flags } => Self::Regex { from, to, flags },
        }
    }
}

impl TryFrom<SelectionSpec> for SelectionSpecDocument {
    type Error = ContractValueError;

    fn try_from(value: SelectionSpec) -> Result<Self, Self::Error> {
        Ok(match value {
            SelectionSpec::Single => Self::Single,
            SelectionSpec::First => Self::First,
            SelectionSpec::Nth { index } => Self::Nth {
                index: NonZeroU32::new(wire_u32(index.get(), "selection.index")?)
                    .expect("non-zero native selection index must remain non-zero on the wire"),
            },
            SelectionSpec::All => Self::All,
        })
    }
}

impl TryFrom<SelectionSpecDocument> for SelectionSpec {
    type Error = ContractValueError;

    fn try_from(value: SelectionSpecDocument) -> Result<Self, Self::Error> {
        Ok(match value {
            SelectionSpecDocument::Single => Self::Single,
            SelectionSpecDocument::First => Self::First,
            SelectionSpecDocument::Nth { index } => Self::Nth {
                index: NonZeroUsize::new(native_usize(index.get(), "selection.index")?)
                    .expect("non-zero wire selection index must remain non-zero on this platform"),
            },
            SelectionSpecDocument::All => Self::All,
        })
    }
}

impl From<ValueSpec> for ValueSpecDocument {
    fn from(value: ValueSpec) -> Self {
        match value {
            ValueSpec::Text => Self::Text,
            ValueSpec::SelectedHtml => Self::SelectedHtml,
            ValueSpec::InnerHtml => Self::InnerHtml,
            ValueSpec::OuterHtml => Self::OuterHtml,
            ValueSpec::Attribute { name } => Self::Attribute { name },
            ValueSpec::Structured => Self::Structured,
        }
    }
}

impl From<ValueSpecDocument> for ValueSpec {
    fn from(value: ValueSpecDocument) -> Self {
        match value {
            ValueSpecDocument::Text => Self::Text,
            ValueSpecDocument::SelectedHtml => Self::SelectedHtml,
            ValueSpecDocument::InnerHtml => Self::InnerHtml,
            ValueSpecDocument::OuterHtml => Self::OuterHtml,
            ValueSpecDocument::Attribute { name } => Self::Attribute { name },
            ValueSpecDocument::Structured => Self::Structured,
        }
    }
}

impl From<BoundaryRetention> for BoundaryRetentionDocument {
    fn from(value: BoundaryRetention) -> Self {
        match value {
            BoundaryRetention::ExcludeBoth => Self::ExcludeBoth,
            BoundaryRetention::IncludeStart => Self::IncludeStart,
            BoundaryRetention::IncludeEnd => Self::IncludeEnd,
            BoundaryRetention::IncludeBoth => Self::IncludeBoth,
        }
    }
}

impl From<BoundaryRetentionDocument> for BoundaryRetention {
    fn from(value: BoundaryRetentionDocument) -> Self {
        match value {
            BoundaryRetentionDocument::ExcludeBoth => Self::ExcludeBoth,
            BoundaryRetentionDocument::IncludeStart => Self::IncludeStart,
            BoundaryRetentionDocument::IncludeEnd => Self::IncludeEnd,
            BoundaryRetentionDocument::IncludeBoth => Self::IncludeBoth,
        }
    }
}

impl TryFrom<ExtractionSpec> for ExtractionSpecDocument {
    type Error = ContractValueError;

    fn try_from(value: ExtractionSpec) -> Result<Self, Self::Error> {
        Ok(match value {
            ExtractionSpec::Selector {
                selector,
                selection,
                value,
            } => Self::Selector {
                selector,
                selection: selection.try_into()?,
                value: value.into(),
            },
            ExtractionSpec::Slice {
                slice,
                selection,
                value,
            } => Self::Slice {
                pattern: slice.pattern.into(),
                boundary_retention: slice.boundary_retention.into(),
                selection: selection.try_into()?,
                value: value.into(),
            },
        })
    }
}

impl TryFrom<ExtractionSpecDocument> for ExtractionSpec {
    type Error = ContractValueError;

    fn try_from(value: ExtractionSpecDocument) -> Result<Self, Self::Error> {
        Ok(match value {
            ExtractionSpecDocument::Selector {
                selector,
                selection,
                value,
            } => Self::Selector {
                selector,
                selection: selection.try_into()?,
                value: value.into(),
            },
            ExtractionSpecDocument::Slice {
                pattern,
                boundary_retention,
                selection,
                value,
            } => Self::Slice {
                slice: SliceSpec {
                    pattern: pattern.into(),
                    boundary_retention: boundary_retention.into(),
                },
                selection: selection.try_into()?,
                value: value.into(),
            },
        })
    }
}

impl From<RenderingOptions> for RenderingOptionsDocument {
    fn from(value: RenderingOptions) -> Self {
        Self {
            whitespace: value.whitespace,
            rewrite_urls: value.rewrite_urls,
        }
    }
}

impl From<RenderingOptionsDocument> for RenderingOptions {
    fn from(value: RenderingOptionsDocument) -> Self {
        Self {
            whitespace: value.whitespace,
            rewrite_urls: value.rewrite_urls,
        }
    }
}

impl TryFrom<OutputOptions> for OutputOptionsDocument {
    type Error = ContractValueError;

    fn try_from(value: OutputOptions) -> Result<Self, Self::Error> {
        Ok(Self {
            rendering: value.rendering.into(),
            include_source_text: value.include_source_text,
            include_html: value.include_html,
            include_text: value.include_text,
            preview_chars: NonZeroU32::new(wire_u32(value.preview_chars.get(), "preview_chars")?)
                .expect("non-zero native preview length must remain non-zero on the wire"),
        })
    }
}

impl TryFrom<OutputOptionsDocument> for OutputOptions {
    type Error = ContractValueError;

    fn try_from(value: OutputOptionsDocument) -> Result<Self, Self::Error> {
        Ok(Self {
            rendering: value.rendering.into(),
            include_source_text: value.include_source_text,
            include_html: value.include_html,
            include_text: value.include_text,
            // HTMLCut supports only targets whose native `usize` can represent its fixed u32
            // wire range, so this conversion is exact rather than a recoverable data failure.
            preview_chars: NonZeroUsize::new({
                const _: () = assert!(usize::BITS >= u32::BITS);
                value.preview_chars.get() as usize
            })
            .expect("non-zero v2 wire preview length must remain non-zero on this platform"),
        })
    }
}

impl From<TlsTrustPolicy> for TlsTrustPolicyDocument {
    fn from(value: TlsTrustPolicy) -> Self {
        match value {
            TlsTrustPolicy::WebPki => Self::WebPki,
            TlsTrustPolicy::Platform => Self::Platform,
            TlsTrustPolicy::CustomCaBundle { path } => Self::CustomCaBundle { path },
        }
    }
}

impl From<TlsTrustPolicyDocument> for TlsTrustPolicy {
    fn from(value: TlsTrustPolicyDocument) -> Self {
        match value {
            TlsTrustPolicyDocument::WebPki => Self::WebPki,
            TlsTrustPolicyDocument::Platform => Self::Platform,
            TlsTrustPolicyDocument::CustomCaBundle { path } => Self::CustomCaBundle { path },
        }
    }
}
