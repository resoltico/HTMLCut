use super::*;

impl TryFrom<SourceRequest> for SourceRequestDocument {
    type Error = ContractValueError;

    fn try_from(value: SourceRequest) -> Result<Self, Self::Error> {
        Ok(Self {
            input: value.input.try_into()?,
            base_url: value.base_url.map(TryInto::try_into).transpose()?,
        })
    }
}

impl From<SourceRequestDocument> for SourceRequest {
    fn from(value: SourceRequestDocument) -> Self {
        Self {
            input: value.input.into(),
            base_url: value.base_url.map(Into::into),
        }
    }
}

impl From<RuntimeOptions> for RuntimeOptionsDocument {
    fn from(value: RuntimeOptions) -> Self {
        Self {
            max_bytes: value.max_bytes,
            fetch_timeout_ms: value.fetch_timeout_ms,
            fetch_connect_timeout_ms: value.fetch_connect_timeout_ms,
            fetch_preflight: value.fetch_preflight,
            tls_trust: value.tls_trust.into(),
        }
    }
}

impl From<RuntimeOptionsDocument> for RuntimeOptions {
    fn from(value: RuntimeOptionsDocument) -> Self {
        Self {
            max_bytes: value.max_bytes,
            fetch_timeout_ms: value.fetch_timeout_ms,
            fetch_connect_timeout_ms: value.fetch_connect_timeout_ms,
            fetch_preflight: value.fetch_preflight,
            tls_trust: value.tls_trust.into(),
        }
    }
}

impl From<InspectionOptions> for InspectionOptionsDocument {
    fn from(value: InspectionOptions) -> Self {
        Self {
            include_source_text: value.include_source_text,
            sample_limit: value.sample_limit,
        }
    }
}

impl From<InspectionOptionsDocument> for InspectionOptions {
    fn from(value: InspectionOptionsDocument) -> Self {
        Self {
            include_source_text: value.include_source_text,
            sample_limit: value.sample_limit,
        }
    }
}

impl TryFrom<ExtractionRequest> for ExtractionRequestDocument {
    type Error = ContractValueError;

    fn try_from(value: ExtractionRequest) -> Result<Self, Self::Error> {
        Ok(Self {
            spec_version: value.spec_version,
            source: value.source.try_into()?,
            extraction: value.extraction.into(),
            output: value.output.into(),
        })
    }
}

impl From<ExtractionRequestDocument> for ExtractionRequest {
    fn from(value: ExtractionRequestDocument) -> Self {
        Self {
            spec_version: value.spec_version,
            source: value.source.into(),
            extraction: value.extraction.into(),
            output: value.output.into(),
        }
    }
}

impl TryFrom<ExtractionDefinition> for ExtractionDefinitionDocument {
    type Error = ContractValueError;

    fn try_from(value: ExtractionDefinition) -> Result<Self, Self::Error> {
        Ok(Self {
            schema_name: value.schema_name,
            schema_version: value.schema_version,
            request: value.request.try_into()?,
            runtime: value.runtime.into(),
        })
    }
}

impl From<ExtractionDefinitionDocument> for ExtractionDefinition {
    fn from(value: ExtractionDefinitionDocument) -> Self {
        Self {
            schema_name: value.schema_name,
            schema_version: value.schema_version,
            request: value.request.into(),
            runtime: value.runtime.into(),
        }
    }
}

impl From<ExtractionResult> for ExtractionResultDocument {
    fn from(value: ExtractionResult) -> Self {
        Self {
            operation_id: value.operation_id,
            schema_name: value.schema_name,
            schema_version: value.schema_version,
            ok: value.ok,
            source: value.source.into(),
            document_title: value.document_title,
            extraction: value.extraction.into(),
            stats: value.stats.into(),
            matches: value.matches.into_iter().map(Into::into).collect(),
            diagnostics: value.diagnostics.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<ExtractionResultDocument> for ExtractionResult {
    fn from(value: ExtractionResultDocument) -> Self {
        Self {
            operation_id: value.operation_id,
            schema_name: value.schema_name,
            schema_version: value.schema_version,
            ok: value.ok,
            source: value.source.into(),
            document_title: value.document_title,
            extraction: value.extraction.into(),
            stats: value.stats.into(),
            matches: value.matches.into_iter().map(Into::into).collect(),
            diagnostics: value.diagnostics.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<SourceInspectionResult> for SourceInspectionResultDocument {
    fn from(value: SourceInspectionResult) -> Self {
        Self {
            operation_id: value.operation_id,
            schema_name: value.schema_name,
            schema_version: value.schema_version,
            ok: value.ok,
            source: value.source.into(),
            document: value.document.map(Into::into),
            diagnostics: value.diagnostics.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<SourceInspectionResultDocument> for SourceInspectionResult {
    fn from(value: SourceInspectionResultDocument) -> Self {
        Self {
            operation_id: value.operation_id,
            schema_name: value.schema_name,
            schema_version: value.schema_version,
            ok: value.ok,
            source: value.source.into(),
            document: value.document.map(Into::into),
            diagnostics: value.diagnostics.into_iter().map(Into::into).collect(),
        }
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

impl From<SelectionSpec> for SelectionSpecDocument {
    fn from(value: SelectionSpec) -> Self {
        match value {
            SelectionSpec::Single => Self::Single,
            SelectionSpec::First => Self::First,
            SelectionSpec::Nth { index } => Self::Nth { index },
            SelectionSpec::All => Self::All,
        }
    }
}

impl From<SelectionSpecDocument> for SelectionSpec {
    fn from(value: SelectionSpecDocument) -> Self {
        match value {
            SelectionSpecDocument::Single => Self::Single,
            SelectionSpecDocument::First => Self::First,
            SelectionSpecDocument::Nth { index } => Self::Nth { index },
            SelectionSpecDocument::All => Self::All,
        }
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

impl From<ExtractionSpec> for ExtractionSpecDocument {
    fn from(value: ExtractionSpec) -> Self {
        match value {
            ExtractionSpec::Selector {
                selector,
                selection,
                value,
            } => Self::Selector {
                selector,
                selection: selection.into(),
                value: value.into(),
            },
            ExtractionSpec::Slice {
                slice,
                selection,
                value,
            } => Self::Slice {
                pattern: slice.pattern.into(),
                boundary_retention: slice.boundary_retention.into(),
                selection: selection.into(),
                value: value.into(),
            },
        }
    }
}

impl From<ExtractionSpecDocument> for ExtractionSpec {
    fn from(value: ExtractionSpecDocument) -> Self {
        match value {
            ExtractionSpecDocument::Selector {
                selector,
                selection,
                value,
            } => Self::Selector {
                selector,
                selection: selection.into(),
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
                selection: selection.into(),
                value: value.into(),
            },
        }
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

impl From<OutputOptions> for OutputOptionsDocument {
    fn from(value: OutputOptions) -> Self {
        Self {
            rendering: value.rendering.into(),
            include_source_text: value.include_source_text,
            include_html: value.include_html,
            include_text: value.include_text,
            preview_chars: value.preview_chars,
        }
    }
}

impl From<OutputOptionsDocument> for OutputOptions {
    fn from(value: OutputOptionsDocument) -> Self {
        Self {
            rendering: value.rendering.into(),
            include_source_text: value.include_source_text,
            include_html: value.include_html,
            include_text: value.include_text,
            preview_chars: value.preview_chars,
        }
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
