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

impl From<SourceMetadata> for SourceMetadataDocument {
    fn from(value: SourceMetadata) -> Self {
        Self {
            kind: value.kind,
            value: value.value,
            input_base_url: value.input_base_url,
            effective_base_url: value.effective_base_url,
            bytes_read: value.bytes_read,
            load_steps: value.load_steps.into_iter().map(Into::into).collect(),
            text: value.text,
        }
    }
}

impl From<SourceMetadataDocument> for SourceMetadata {
    fn from(value: SourceMetadataDocument) -> Self {
        Self {
            kind: value.kind,
            value: value.value,
            input_base_url: value.input_base_url,
            effective_base_url: value.effective_base_url,
            bytes_read: value.bytes_read,
            load_steps: value.load_steps.into_iter().map(Into::into).collect(),
            text: value.text,
        }
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

impl From<ExtractionStats> for ExtractionStatsDocument {
    fn from(value: ExtractionStats) -> Self {
        Self {
            duration_ms: value.duration_ms,
            candidate_count: value.candidate_count,
            match_count: value.match_count,
        }
    }
}

impl From<ExtractionStatsDocument> for ExtractionStats {
    fn from(value: ExtractionStatsDocument) -> Self {
        Self {
            duration_ms: value.duration_ms,
            candidate_count: value.candidate_count,
            match_count: value.match_count,
        }
    }
}

impl From<Range> for RangeDocument {
    fn from(value: Range) -> Self {
        Self {
            start: value.start,
            end: value.end,
        }
    }
}

impl From<RangeDocument> for Range {
    fn from(value: RangeDocument) -> Self {
        Self {
            start: value.start,
            end: value.end,
        }
    }
}

impl From<SelectorMatchMetadata> for SelectorMatchMetadataDocument {
    fn from(value: SelectorMatchMetadata) -> Self {
        Self {
            candidate_count: value.candidate_count,
            candidate_index: value.candidate_index,
            path: value.path,
            tag_name: value.tag_name,
            attributes: value.attributes,
        }
    }
}

impl From<SelectorMatchMetadataDocument> for SelectorMatchMetadata {
    fn from(value: SelectorMatchMetadataDocument) -> Self {
        Self {
            candidate_count: value.candidate_count,
            candidate_index: value.candidate_index,
            path: value.path,
            tag_name: value.tag_name,
            attributes: value.attributes,
        }
    }
}

impl From<DelimiterPairMatchMetadata> for DelimiterPairMatchMetadataDocument {
    fn from(value: DelimiterPairMatchMetadata) -> Self {
        Self {
            candidate_count: value.candidate_count,
            candidate_index: value.candidate_index,
            selected_range: value.selected_range.into(),
            inner_range: value.inner_range.into(),
            outer_range: value.outer_range.into(),
            include_start: value.include_start,
            include_end: value.include_end,
            matched_start: value.matched_start,
            matched_end: value.matched_end,
        }
    }
}

impl From<DelimiterPairMatchMetadataDocument> for DelimiterPairMatchMetadata {
    fn from(value: DelimiterPairMatchMetadataDocument) -> Self {
        Self {
            candidate_count: value.candidate_count,
            candidate_index: value.candidate_index,
            selected_range: value.selected_range.into(),
            inner_range: value.inner_range.into(),
            outer_range: value.outer_range.into(),
            include_start: value.include_start,
            include_end: value.include_end,
            matched_start: value.matched_start,
            matched_end: value.matched_end,
        }
    }
}

impl From<ExtractionMatchMetadata> for ExtractionMatchMetadataDocument {
    fn from(value: ExtractionMatchMetadata) -> Self {
        match value {
            ExtractionMatchMetadata::Selector(metadata) => Self::Selector(metadata.into()),
            ExtractionMatchMetadata::DelimiterPair(metadata) => {
                Self::DelimiterPair(metadata.into())
            }
        }
    }
}

impl From<ExtractionMatchMetadataDocument> for ExtractionMatchMetadata {
    fn from(value: ExtractionMatchMetadataDocument) -> Self {
        match value {
            ExtractionMatchMetadataDocument::Selector(metadata) => Self::Selector(metadata.into()),
            ExtractionMatchMetadataDocument::DelimiterPair(metadata) => {
                Self::DelimiterPair(metadata.into())
            }
        }
    }
}

impl From<ExtractionMatch> for ExtractionMatchDocument {
    fn from(value: ExtractionMatch) -> Self {
        Self {
            index: value.index,
            path: value.path,
            value_type: value.value_type,
            value: value.value,
            html: value.html,
            text: value.text,
            preview: value.preview,
            metadata: value.metadata.into(),
        }
    }
}

impl From<ExtractionMatchDocument> for ExtractionMatch {
    fn from(value: ExtractionMatchDocument) -> Self {
        Self {
            index: value.index,
            path: value.path,
            value_type: value.value_type,
            value: value.value,
            html: value.html,
            text: value.text,
            preview: value.preview,
            metadata: value.metadata.into(),
        }
    }
}

impl From<InspectionCount> for InspectionCountDocument {
    fn from(value: InspectionCount) -> Self {
        Self {
            name: value.name,
            count: value.count,
        }
    }
}

impl From<InspectionCountDocument> for InspectionCount {
    fn from(value: InspectionCountDocument) -> Self {
        Self {
            name: value.name,
            count: value.count,
        }
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

impl From<ContentCandidateInspection> for ContentCandidateInspectionDocument {
    fn from(value: ContentCandidateInspection) -> Self {
        Self {
            selector: value.selector,
            path: value.path,
            tag_name: value.tag_name,
            text_char_count: value.text_char_count,
            heading_count: value.heading_count,
            link_count: value.link_count,
        }
    }
}

impl From<ContentCandidateInspectionDocument> for ContentCandidateInspection {
    fn from(value: ContentCandidateInspectionDocument) -> Self {
        Self {
            selector: value.selector,
            path: value.path,
            tag_name: value.tag_name,
            text_char_count: value.text_char_count,
            heading_count: value.heading_count,
            link_count: value.link_count,
        }
    }
}

impl From<DocumentInspection> for DocumentInspectionDocument {
    fn from(value: DocumentInspection) -> Self {
        Self {
            title: value.title,
            root_tag: value.root_tag,
            element_count: value.element_count,
            text_char_count: value.text_char_count,
            link_count: value.link_count,
            image_count: value.image_count,
            form_count: value.form_count,
            table_count: value.table_count,
            script_count: value.script_count,
            style_count: value.style_count,
            document_base_href: value.document_base_href,
            top_tags: value.top_tags.into_iter().map(Into::into).collect(),
            top_classes: value.top_classes.into_iter().map(Into::into).collect(),
            extraction_candidates: value
                .extraction_candidates
                .into_iter()
                .map(Into::into)
                .collect(),
            reading_candidates: value
                .reading_candidates
                .into_iter()
                .map(Into::into)
                .collect(),
            headings: value.headings.into_iter().map(Into::into).collect(),
            links: value.links.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<DocumentInspectionDocument> for DocumentInspection {
    fn from(value: DocumentInspectionDocument) -> Self {
        Self {
            title: value.title,
            root_tag: value.root_tag,
            element_count: value.element_count,
            text_char_count: value.text_char_count,
            link_count: value.link_count,
            image_count: value.image_count,
            form_count: value.form_count,
            table_count: value.table_count,
            script_count: value.script_count,
            style_count: value.style_count,
            document_base_href: value.document_base_href,
            top_tags: value.top_tags.into_iter().map(Into::into).collect(),
            top_classes: value.top_classes.into_iter().map(Into::into).collect(),
            extraction_candidates: value
                .extraction_candidates
                .into_iter()
                .map(Into::into)
                .collect(),
            reading_candidates: value
                .reading_candidates
                .into_iter()
                .map(Into::into)
                .collect(),
            headings: value.headings.into_iter().map(Into::into).collect(),
            links: value.links.into_iter().map(Into::into).collect(),
        }
    }
}
