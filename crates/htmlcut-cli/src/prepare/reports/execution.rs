use htmlcut_core::{ExtractionResult, SourceInspectionResult};

use crate::metadata::{ENGINE_NAME, HTMLCUT_VERSION, TOOL_NAME};
use crate::model::{
    BundlePaths, EXTRACTION_COMMAND_REPORT_SCHEMA_NAME, EXTRACTION_COMMAND_REPORT_SCHEMA_VERSION,
    ExtractionCommandReport, SOURCE_INSPECTION_COMMAND_REPORT_SCHEMA_NAME,
    SOURCE_INSPECTION_COMMAND_REPORT_SCHEMA_VERSION, SourceInspectionCommandReport,
};

pub(crate) fn build_extraction_report(
    command: impl Into<String>,
    result: ExtractionResult,
    bundle: Option<BundlePaths>,
) -> ExtractionCommandReport {
    ExtractionCommandReport {
        tool: TOOL_NAME.to_owned(),
        engine: ENGINE_NAME.to_owned(),
        version: HTMLCUT_VERSION.to_owned(),
        schema_name: EXTRACTION_COMMAND_REPORT_SCHEMA_NAME.to_owned(),
        schema_version: EXTRACTION_COMMAND_REPORT_SCHEMA_VERSION,
        command: command.into(),
        operation_id: result.operation_id,
        ok: result.ok,
        source: result.source,
        extraction: result.extraction,
        stats: result.stats,
        document_title: result.document_title,
        matches: result.matches,
        diagnostics: result.diagnostics,
        bundle,
    }
}

pub(crate) fn build_source_inspection_report(
    command: impl Into<String>,
    result: SourceInspectionResult,
) -> SourceInspectionCommandReport {
    SourceInspectionCommandReport {
        tool: TOOL_NAME.to_owned(),
        engine: ENGINE_NAME.to_owned(),
        version: HTMLCUT_VERSION.to_owned(),
        schema_name: SOURCE_INSPECTION_COMMAND_REPORT_SCHEMA_NAME.to_owned(),
        schema_version: SOURCE_INSPECTION_COMMAND_REPORT_SCHEMA_VERSION,
        command: command.into(),
        operation_id: result.operation_id,
        ok: result.ok,
        source: result.source,
        document: result.document,
        diagnostics: result.diagnostics,
    }
}
