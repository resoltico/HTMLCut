use schemars::schema_for;
use serde_json::Value;

use crate::error::CliError;
use crate::lookup::unknown_schema_error;
use crate::metadata::{HTMLCUT_DESCRIPTION, HTMLCUT_VERSION, TOOL_NAME};
use crate::model::{
    CATALOG_REPORT_SCHEMA_NAME, CATALOG_SCHEMA_VERSION, CatalogCommandReport,
    EXTRACTION_COMMAND_REPORT_SCHEMA_NAME, EXTRACTION_COMMAND_REPORT_SCHEMA_VERSION,
    ExtractionCommandReport, SCHEMA_COMMAND_REPORT_SCHEMA_NAME,
    SCHEMA_COMMAND_REPORT_SCHEMA_VERSION, SOURCE_INSPECTION_COMMAND_REPORT_SCHEMA_NAME,
    SOURCE_INSPECTION_COMMAND_REPORT_SCHEMA_VERSION, SchemaCommandReport, SchemaDocumentReport,
    SourceInspectionCommandReport,
};

pub(crate) fn build_schema_report(
    name_filter: Option<&str>,
    version_filter: Option<u32>,
) -> Result<SchemaCommandReport, CliError> {
    if let (Some(_), None) = (version_filter, name_filter) {
        return Err(crate::error::usage_error(
            "CLI_SCHEMA_VERSION_REQUIRES_NAME",
            "`--schema-version` requires `--name`.",
        ));
    }

    let mut schemas = htmlcut_core::schema_catalog()
        .iter()
        .map(build_schema_document_report)
        .chain(
            cli_schema_catalog()
                .iter()
                .map(build_schema_document_report),
        )
        .collect::<Vec<_>>();

    schemas.sort_by(|left, right| {
        left.schema_name
            .cmp(&right.schema_name)
            .then(left.schema_version.cmp(&right.schema_version))
    });

    let filtered = schemas
        .iter()
        .filter(|schema| name_filter.is_none_or(|name| schema.schema_name == name))
        .filter(|schema| version_filter.is_none_or(|version| schema.schema_version == version))
        .cloned()
        .collect::<Vec<_>>();

    if filtered.is_empty() {
        let name = name_filter.expect("version-only filters return earlier");
        return Err(unknown_schema_error(name, version_filter, &schemas));
    }

    Ok(SchemaCommandReport {
        tool: TOOL_NAME.to_owned(),
        version: HTMLCUT_VERSION.to_owned(),
        schema_name: SCHEMA_COMMAND_REPORT_SCHEMA_NAME.to_owned(),
        schema_version: SCHEMA_COMMAND_REPORT_SCHEMA_VERSION,
        schema_profile: htmlcut_core::HTMLCUT_JSON_SCHEMA_PROFILE.to_owned(),
        description: HTMLCUT_DESCRIPTION.to_owned(),
        command: "schema".to_owned(),
        schemas: filtered,
    })
}

const CLI_SCHEMA_CATALOG: &[htmlcut_core::SchemaDescriptor] = &[
    htmlcut_core::SchemaDescriptor {
        schema_ref: htmlcut_core::SchemaRef::new(
            EXTRACTION_COMMAND_REPORT_SCHEMA_NAME,
            EXTRACTION_COMMAND_REPORT_SCHEMA_VERSION,
        ),
        owner_surface: "htmlcut-cli",
        rust_shape: "ExtractionCommandReport",
        stability: htmlcut_core::SchemaStability::Versioned,
        json_schema: extraction_command_report_schema,
    },
    htmlcut_core::SchemaDescriptor {
        schema_ref: htmlcut_core::SchemaRef::new(
            SOURCE_INSPECTION_COMMAND_REPORT_SCHEMA_NAME,
            SOURCE_INSPECTION_COMMAND_REPORT_SCHEMA_VERSION,
        ),
        owner_surface: "htmlcut-cli",
        rust_shape: "SourceInspectionCommandReport",
        stability: htmlcut_core::SchemaStability::Versioned,
        json_schema: source_inspection_command_report_schema,
    },
    htmlcut_core::SchemaDescriptor {
        schema_ref: htmlcut_core::SchemaRef::new(
            CATALOG_REPORT_SCHEMA_NAME,
            CATALOG_SCHEMA_VERSION,
        ),
        owner_surface: "htmlcut-cli",
        rust_shape: "CatalogCommandReport",
        stability: htmlcut_core::SchemaStability::Versioned,
        json_schema: catalog_command_report_schema,
    },
    htmlcut_core::SchemaDescriptor {
        schema_ref: htmlcut_core::SchemaRef::new(
            SCHEMA_COMMAND_REPORT_SCHEMA_NAME,
            SCHEMA_COMMAND_REPORT_SCHEMA_VERSION,
        ),
        owner_surface: "htmlcut-cli",
        rust_shape: "SchemaCommandReport",
        stability: htmlcut_core::SchemaStability::Versioned,
        json_schema: schema_command_report_schema,
    },
];

fn cli_schema_catalog() -> &'static [htmlcut_core::SchemaDescriptor] {
    CLI_SCHEMA_CATALOG
}

fn build_schema_document_report(
    descriptor: &htmlcut_core::SchemaDescriptor,
) -> SchemaDocumentReport {
    SchemaDocumentReport {
        schema_name: descriptor.schema_ref.schema_name.to_owned(),
        schema_version: descriptor.schema_ref.schema_version,
        owner_surface: descriptor.owner_surface.to_owned(),
        rust_shape: descriptor.rust_shape.to_owned(),
        stability: descriptor.stability,
        json_schema: (descriptor.json_schema)(),
    }
}

fn schema_json_for<T: schemars::JsonSchema>() -> Value {
    serde_json::to_value(schema_for!(T)).expect("JSON Schema documents should always serialize")
}

fn extraction_command_report_schema() -> Value {
    schema_json_for::<ExtractionCommandReport>()
}

fn source_inspection_command_report_schema() -> Value {
    schema_json_for::<SourceInspectionCommandReport>()
}

fn catalog_command_report_schema() -> Value {
    schema_json_for::<CatalogCommandReport>()
}

fn schema_command_report_schema() -> Value {
    schema_json_for::<SchemaCommandReport>()
}
