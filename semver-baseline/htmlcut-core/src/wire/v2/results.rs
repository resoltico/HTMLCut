use super::*;

impl From<Diagnostic> for DiagnosticDocument {
    fn from(value: Diagnostic) -> Self {
        Self {
            level: value.level,
            code: value.code,
            message: value.message,
            details: value.details,
        }
    }
}

impl From<DiagnosticDocument> for Diagnostic {
    fn from(value: DiagnosticDocument) -> Self {
        Self {
            level: value.level,
            code: value.code,
            message: value.message,
            details: value.details,
        }
    }
}

impl TryFrom<SourceMetadata> for SourceMetadataDocument {
    type Error = ContractValueError;

    fn try_from(value: SourceMetadata) -> Result<Self, Self::Error> {
        Ok(Self {
            kind: value.kind,
            value: value.value,
            input_base_url: value.input_base_url,
            effective_base_url: value.effective_base_url,
            bytes_read: wire_u64(value.bytes_read, "source.bytes_read")?,
            load_steps: value.load_steps.into_iter().map(Into::into).collect(),
            text: value.text,
        })
    }
}

impl TryFrom<SourceMetadataDocument> for SourceMetadata {
    type Error = ContractValueError;

    fn try_from(value: SourceMetadataDocument) -> Result<Self, Self::Error> {
        Ok(Self {
            kind: value.kind,
            value: value.value,
            input_base_url: value.input_base_url,
            effective_base_url: value.effective_base_url,
            bytes_read: native_usize_from_u64(value.bytes_read, "source.bytes_read")?,
            load_steps: value.load_steps.into_iter().map(Into::into).collect(),
            text: value.text,
        })
    }
}

impl From<SourceLoadStep> for SourceLoadStepDocument {
    fn from(value: SourceLoadStep) -> Self {
        Self {
            action: value.action,
            outcome: value.outcome,
            status: value.status,
            message: value.message,
        }
    }
}

impl From<SourceLoadStepDocument> for SourceLoadStep {
    fn from(value: SourceLoadStepDocument) -> Self {
        Self {
            action: value.action,
            outcome: value.outcome,
            status: value.status,
            message: value.message,
        }
    }
}

impl TryFrom<ExtractionStats> for ExtractionStatsDocument {
    type Error = ContractValueError;

    fn try_from(value: ExtractionStats) -> Result<Self, Self::Error> {
        Ok(Self {
            duration_ms: wire_duration_ms(value.duration_ms)?,
            candidate_count: wire_u32(value.candidate_count, "stats.candidate_count")?,
            match_count: wire_u32(value.match_count, "stats.match_count")?,
        })
    }
}

impl TryFrom<ExtractionStatsDocument> for ExtractionStats {
    type Error = ContractValueError;

    fn try_from(value: ExtractionStatsDocument) -> Result<Self, Self::Error> {
        Ok(Self {
            duration_ms: u128::from(value.duration_ms),
            candidate_count: native_usize(value.candidate_count, "stats.candidate_count")?,
            match_count: native_usize(value.match_count, "stats.match_count")?,
        })
    }
}

impl TryFrom<Range> for RangeDocument {
    type Error = ContractValueError;

    fn try_from(value: Range) -> Result<Self, Self::Error> {
        Ok(Self {
            start: wire_u64(value.start, "range.start")?,
            end: wire_u64(value.end, "range.end")?,
        })
    }
}

impl TryFrom<RangeDocument> for Range {
    type Error = ContractValueError;

    fn try_from(value: RangeDocument) -> Result<Self, Self::Error> {
        Ok(Self {
            start: native_usize_from_u64(value.start, "range.start")?,
            end: native_usize_from_u64(value.end, "range.end")?,
        })
    }
}

impl TryFrom<SelectorMatchMetadata> for SelectorMatchMetadataDocument {
    type Error = ContractValueError;

    fn try_from(value: SelectorMatchMetadata) -> Result<Self, Self::Error> {
        Ok(Self {
            candidate_count: wire_u32(value.candidate_count, "selector.candidate_count")?,
            candidate_index: wire_u32(value.candidate_index, "selector.candidate_index")?,
            path: value.path,
            tag_name: value.tag_name,
            attributes: value.attributes,
        })
    }
}

impl TryFrom<SelectorMatchMetadataDocument> for SelectorMatchMetadata {
    type Error = ContractValueError;

    fn try_from(value: SelectorMatchMetadataDocument) -> Result<Self, Self::Error> {
        Ok(Self {
            candidate_count: native_usize(value.candidate_count, "selector.candidate_count")?,
            candidate_index: native_usize(value.candidate_index, "selector.candidate_index")?,
            path: value.path,
            tag_name: value.tag_name,
            attributes: value.attributes,
        })
    }
}

impl TryFrom<DelimiterPairMatchMetadata> for DelimiterPairMatchMetadataDocument {
    type Error = ContractValueError;

    fn try_from(value: DelimiterPairMatchMetadata) -> Result<Self, Self::Error> {
        Ok(Self {
            candidate_count: wire_u32(value.candidate_count, "delimiter_pair.candidate_count")?,
            candidate_index: wire_u32(value.candidate_index, "delimiter_pair.candidate_index")?,
            selected_range: value.selected_range.try_into()?,
            inner_range: value.inner_range.try_into()?,
            outer_range: value.outer_range.try_into()?,
            include_start: value.include_start,
            include_end: value.include_end,
            matched_start: value.matched_start,
            matched_end: value.matched_end,
        })
    }
}

impl TryFrom<DelimiterPairMatchMetadataDocument> for DelimiterPairMatchMetadata {
    type Error = ContractValueError;

    fn try_from(value: DelimiterPairMatchMetadataDocument) -> Result<Self, Self::Error> {
        Ok(Self {
            candidate_count: native_usize(value.candidate_count, "delimiter_pair.candidate_count")?,
            candidate_index: native_usize(value.candidate_index, "delimiter_pair.candidate_index")?,
            selected_range: value.selected_range.try_into()?,
            inner_range: value.inner_range.try_into()?,
            outer_range: value.outer_range.try_into()?,
            include_start: value.include_start,
            include_end: value.include_end,
            matched_start: value.matched_start,
            matched_end: value.matched_end,
        })
    }
}

impl TryFrom<ExtractionMatchMetadata> for ExtractionMatchMetadataDocument {
    type Error = ContractValueError;

    fn try_from(value: ExtractionMatchMetadata) -> Result<Self, Self::Error> {
        Ok(match value {
            ExtractionMatchMetadata::Selector(metadata) => Self::Selector(metadata.try_into()?),
            ExtractionMatchMetadata::DelimiterPair(metadata) => {
                Self::DelimiterPair(metadata.try_into()?)
            }
        })
    }
}

impl TryFrom<ExtractionMatchMetadataDocument> for ExtractionMatchMetadata {
    type Error = ContractValueError;

    fn try_from(value: ExtractionMatchMetadataDocument) -> Result<Self, Self::Error> {
        Ok(match value {
            ExtractionMatchMetadataDocument::Selector(metadata) => {
                Self::Selector(metadata.try_into()?)
            }
            ExtractionMatchMetadataDocument::DelimiterPair(metadata) => {
                Self::DelimiterPair(metadata.try_into()?)
            }
        })
    }
}

impl TryFrom<ExtractionMatch> for ExtractionMatchDocument {
    type Error = ContractValueError;

    fn try_from(value: ExtractionMatch) -> Result<Self, Self::Error> {
        Ok(Self {
            index: wire_u32(value.index, "match.index")?,
            path: value.path,
            value_type: value.value_type,
            value: value.value,
            html: value.html,
            text: value.text,
            preview: value.preview,
            metadata: value.metadata.try_into()?,
        })
    }
}

impl TryFrom<ExtractionMatchDocument> for ExtractionMatch {
    type Error = ContractValueError;

    fn try_from(value: ExtractionMatchDocument) -> Result<Self, Self::Error> {
        Ok(Self {
            index: native_usize(value.index, "match.index")?,
            path: value.path,
            value_type: value.value_type,
            value: value.value,
            html: value.html,
            text: value.text,
            preview: value.preview,
            metadata: value.metadata.try_into()?,
        })
    }
}

impl TryFrom<InspectionCount> for InspectionCountDocument {
    type Error = ContractValueError;

    fn try_from(value: InspectionCount) -> Result<Self, Self::Error> {
        Ok(Self {
            name: value.name,
            count: wire_u32(value.count, "inspection.count")?,
        })
    }
}

impl TryFrom<InspectionCountDocument> for InspectionCount {
    type Error = ContractValueError;

    fn try_from(value: InspectionCountDocument) -> Result<Self, Self::Error> {
        Ok(Self {
            name: value.name,
            count: native_usize(value.count, "inspection.count")?,
        })
    }
}

impl From<HeadingInspection> for HeadingInspectionDocument {
    fn from(value: HeadingInspection) -> Self {
        Self {
            level: value.level,
            text: value.text,
            path: value.path,
        }
    }
}

impl From<HeadingInspectionDocument> for HeadingInspection {
    fn from(value: HeadingInspectionDocument) -> Self {
        Self {
            level: value.level,
            text: value.text,
            path: value.path,
        }
    }
}

impl From<LinkInspection> for LinkInspectionDocument {
    fn from(value: LinkInspection) -> Self {
        Self {
            text: value.text,
            href: value.href,
            resolved_href: value.resolved_href,
            path: value.path,
        }
    }
}

impl From<LinkInspectionDocument> for LinkInspection {
    fn from(value: LinkInspectionDocument) -> Self {
        Self {
            text: value.text,
            href: value.href,
            resolved_href: value.resolved_href,
            path: value.path,
        }
    }
}

impl TryFrom<ContentCandidateInspection> for ContentCandidateInspectionDocument {
    type Error = ContractValueError;

    fn try_from(value: ContentCandidateInspection) -> Result<Self, Self::Error> {
        Ok(Self {
            selector: value.selector,
            path: value.path,
            tag_name: value.tag_name,
            text_char_count: wire_u32(value.text_char_count, "content_candidate.text_char_count")?,
            heading_count: wire_u32(value.heading_count, "content_candidate.heading_count")?,
            link_count: wire_u32(value.link_count, "content_candidate.link_count")?,
        })
    }
}

impl TryFrom<ContentCandidateInspectionDocument> for ContentCandidateInspection {
    type Error = ContractValueError;

    fn try_from(value: ContentCandidateInspectionDocument) -> Result<Self, Self::Error> {
        Ok(Self {
            selector: value.selector,
            path: value.path,
            tag_name: value.tag_name,
            // Every supported platform represents v2's fixed u32 counts exactly.
            text_char_count: {
                const _: () = assert!(usize::BITS >= u32::BITS);
                value.text_char_count as usize
            },
            heading_count: value.heading_count as usize,
            link_count: value.link_count as usize,
        })
    }
}

impl TryFrom<DocumentInspection> for DocumentInspectionDocument {
    type Error = ContractValueError;

    fn try_from(value: DocumentInspection) -> Result<Self, Self::Error> {
        Ok(Self {
            title: value.title,
            root_tag: value.root_tag,
            element_count: wire_u32(value.element_count, "document.element_count")?,
            text_char_count: wire_u32(value.text_char_count, "document.text_char_count")?,
            link_count: wire_u32(value.link_count, "document.link_count")?,
            image_count: wire_u32(value.image_count, "document.image_count")?,
            form_count: wire_u32(value.form_count, "document.form_count")?,
            table_count: wire_u32(value.table_count, "document.table_count")?,
            script_count: wire_u32(value.script_count, "document.script_count")?,
            style_count: wire_u32(value.style_count, "document.style_count")?,
            document_base_href: value.document_base_href,
            top_tags: value
                .top_tags
                .into_iter()
                .map(TryInto::try_into)
                .collect::<Result<_, _>>()?,
            top_classes: value
                .top_classes
                .into_iter()
                .map(TryInto::try_into)
                .collect::<Result<_, _>>()?,
            extraction_candidates: value
                .extraction_candidates
                .into_iter()
                .map(TryInto::try_into)
                .collect::<Result<_, _>>()?,
            reading_candidates: value
                .reading_candidates
                .into_iter()
                .map(TryInto::try_into)
                .collect::<Result<_, _>>()?,
            headings: value.headings.into_iter().map(Into::into).collect(),
            links: value.links.into_iter().map(Into::into).collect(),
        })
    }
}

impl TryFrom<DocumentInspectionDocument> for DocumentInspection {
    type Error = ContractValueError;

    fn try_from(value: DocumentInspectionDocument) -> Result<Self, Self::Error> {
        Ok(Self {
            title: value.title,
            root_tag: value.root_tag,
            element_count: native_usize(value.element_count, "document.element_count")?,
            text_char_count: native_usize(value.text_char_count, "document.text_char_count")?,
            link_count: native_usize(value.link_count, "document.link_count")?,
            image_count: native_usize(value.image_count, "document.image_count")?,
            form_count: native_usize(value.form_count, "document.form_count")?,
            table_count: native_usize(value.table_count, "document.table_count")?,
            script_count: native_usize(value.script_count, "document.script_count")?,
            style_count: native_usize(value.style_count, "document.style_count")?,
            document_base_href: value.document_base_href,
            top_tags: value
                .top_tags
                .into_iter()
                .map(TryInto::try_into)
                .collect::<Result<_, _>>()?,
            top_classes: value
                .top_classes
                .into_iter()
                .map(TryInto::try_into)
                .collect::<Result<_, _>>()?,
            extraction_candidates: value
                .extraction_candidates
                .into_iter()
                .map(TryInto::try_into)
                .collect::<Result<_, _>>()?,
            reading_candidates: value
                .reading_candidates
                .into_iter()
                .map(TryInto::try_into)
                .collect::<Result<_, _>>()?,
            headings: value.headings.into_iter().map(Into::into).collect(),
            links: value.links.into_iter().map(Into::into).collect(),
        })
    }
}
