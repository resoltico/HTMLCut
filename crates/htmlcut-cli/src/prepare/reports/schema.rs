#[cfg(test)]
use std::collections::BTreeSet;

use schemars::schema_for;
use serde_json::Value;

use crate::error::CliError;
use crate::error::internal_error;
use crate::lookup::unknown_schema_error;
use crate::metadata::{HTMLCUT_DESCRIPTION, HTMLCUT_VERSION, TOOL_NAME};
use crate::model::{
    CATALOG_REPORT_SCHEMA_NAME, CATALOG_SCHEMA_VERSION, CatalogCommandReport, CliErrorCode,
    ERROR_COMMAND_REPORT_SCHEMA_NAME, ERROR_COMMAND_REPORT_SCHEMA_VERSION,
    EXTRACTION_COMMAND_REPORT_SCHEMA_NAME, EXTRACTION_COMMAND_REPORT_SCHEMA_VERSION,
    ErrorCommandReport, ExtractionCommandReport, SCHEMA_COMMAND_REPORT_SCHEMA_NAME,
    SCHEMA_COMMAND_REPORT_SCHEMA_VERSION, SCHEMA_INVENTORY_REPORT_SCHEMA_NAME,
    SCHEMA_INVENTORY_REPORT_SCHEMA_VERSION, SOURCE_INSPECTION_COMMAND_REPORT_SCHEMA_NAME,
    SOURCE_INSPECTION_COMMAND_REPORT_SCHEMA_VERSION, SchemaCommandReport, SchemaDocumentReport,
    SchemaInventoryCommandReport, SchemaInventoryEntryReport, SourceInspectionCommandReport,
};

pub(crate) fn build_schema_report(
    name_filter: Option<&str>,
    version_filter: Option<u32>,
) -> Result<SchemaCommandReport, CliError> {
    let filtered = collect_filtered_schemas(name_filter, version_filter)?;

    Ok(SchemaCommandReport {
        tool: TOOL_NAME.to_owned(),
        version: HTMLCUT_VERSION.to_owned(),
        schema_name: SCHEMA_COMMAND_REPORT_SCHEMA_NAME.to_owned(),
        schema_version: SCHEMA_COMMAND_REPORT_SCHEMA_VERSION,
        schema_profile: htmlcut_core::HTMLCUT_JSON_SCHEMA_PROFILE.to_owned(),
        description: HTMLCUT_DESCRIPTION.to_owned(),
        command: "schema".to_owned(),
        schemas: filtered
            .iter()
            .map(build_schema_document_report)
            .collect::<Result<Vec<_>, _>>()?,
    })
}

pub(crate) fn build_schema_inventory_report(
    name_filter: Option<&str>,
    version_filter: Option<u32>,
) -> Result<SchemaInventoryCommandReport, CliError> {
    let filtered = collect_filtered_schemas(name_filter, version_filter)?;

    Ok(SchemaInventoryCommandReport {
        tool: TOOL_NAME.to_owned(),
        version: HTMLCUT_VERSION.to_owned(),
        schema_name: SCHEMA_INVENTORY_REPORT_SCHEMA_NAME.to_owned(),
        schema_version: SCHEMA_INVENTORY_REPORT_SCHEMA_VERSION,
        schema_profile: htmlcut_core::HTMLCUT_JSON_SCHEMA_PROFILE.to_owned(),
        description: HTMLCUT_DESCRIPTION.to_owned(),
        command: "schema".to_owned(),
        schemas: filtered.iter().map(build_schema_inventory_entry).collect(),
    })
}

fn collect_filtered_schemas(
    name_filter: Option<&str>,
    version_filter: Option<u32>,
) -> Result<Vec<htmlcut_core::SchemaDescriptor>, CliError> {
    if let (Some(_), None) = (version_filter, name_filter) {
        return Err(crate::error::usage_error(
            CliErrorCode::SchemaVersionRequiresName,
            "`--schema-version` requires `--name`.",
        ));
    }

    let mut schemas = htmlcut_core::schema_catalog()
        .iter()
        .copied()
        .chain(cli_schema_catalog().iter().copied())
        .collect::<Vec<_>>();

    schemas.sort_by(|left, right| {
        left.schema_ref
            .schema_name
            .cmp(right.schema_ref.schema_name)
            .then(
                left.schema_ref
                    .schema_version
                    .cmp(&right.schema_ref.schema_version),
            )
    });

    let filtered = if let Some(name) = name_filter {
        let filtered = schemas
            .iter()
            .filter(|schema| schema.schema_ref.schema_name == name)
            .filter(|schema| {
                version_filter.is_none_or(|version| schema.schema_ref.schema_version == version)
            })
            .cloned()
            .collect::<Vec<_>>();

        if filtered.is_empty() {
            let known = schemas
                .iter()
                .map(build_schema_inventory_entry)
                .map(|schema| SchemaDocumentReport {
                    schema_name: schema.schema_name,
                    schema_version: schema.schema_version,
                    surface: schema.surface,
                    profile: schema.profile,
                    artifact: schema.artifact,
                    stability: schema.stability,
                    json_schema: Value::Null,
                })
                .collect::<Vec<_>>();
            return Err(unknown_schema_error(name, version_filter, &known));
        }

        filtered
    } else {
        schemas
    };

    Ok(filtered)
}

const CLI_SCHEMA_CATALOG: &[htmlcut_core::SchemaDescriptor] = &[
    cli_schema_descriptor(
        htmlcut_core::SchemaRef::new(
            EXTRACTION_COMMAND_REPORT_SCHEMA_NAME,
            EXTRACTION_COMMAND_REPORT_SCHEMA_VERSION,
        ),
        "extraction report",
        extraction_command_report_schema,
    ),
    cli_schema_descriptor(
        htmlcut_core::SchemaRef::new(
            SOURCE_INSPECTION_COMMAND_REPORT_SCHEMA_NAME,
            SOURCE_INSPECTION_COMMAND_REPORT_SCHEMA_VERSION,
        ),
        "source inspection report",
        source_inspection_command_report_schema,
    ),
    cli_schema_descriptor(
        htmlcut_core::SchemaRef::new(CATALOG_REPORT_SCHEMA_NAME, CATALOG_SCHEMA_VERSION),
        "catalog report",
        catalog_command_report_schema,
    ),
    cli_schema_descriptor(
        htmlcut_core::SchemaRef::new(
            ERROR_COMMAND_REPORT_SCHEMA_NAME,
            ERROR_COMMAND_REPORT_SCHEMA_VERSION,
        ),
        "error report",
        error_command_report_schema,
    ),
    cli_schema_descriptor(
        htmlcut_core::SchemaRef::new(
            SCHEMA_COMMAND_REPORT_SCHEMA_NAME,
            SCHEMA_COMMAND_REPORT_SCHEMA_VERSION,
        ),
        "schema report",
        schema_command_report_schema,
    ),
    cli_schema_descriptor(
        htmlcut_core::SchemaRef::new(
            SCHEMA_INVENTORY_REPORT_SCHEMA_NAME,
            SCHEMA_INVENTORY_REPORT_SCHEMA_VERSION,
        ),
        "schema inventory report",
        schema_inventory_command_report_schema,
    ),
];

fn cli_schema_catalog() -> &'static [htmlcut_core::SchemaDescriptor] {
    CLI_SCHEMA_CATALOG
}

const fn cli_schema_descriptor(
    schema_ref: htmlcut_core::SchemaRef,
    contract_family: &'static str,
    json_schema: fn() -> Result<Value, htmlcut_core::SchemaExportError>,
) -> htmlcut_core::SchemaDescriptor {
    htmlcut_core::SchemaDescriptor {
        schema_ref,
        owner: "cli",
        contract_family,
        stability: htmlcut_core::SchemaStability::Versioned,
        json_schema,
    }
}

#[cfg(test)]
pub(crate) fn cli_schema_descriptor_for_tests(
    schema_ref: htmlcut_core::SchemaRef,
    contract_family: &'static str,
    json_schema: fn() -> Result<Value, htmlcut_core::SchemaExportError>,
) -> htmlcut_core::SchemaDescriptor {
    cli_schema_descriptor(schema_ref, contract_family, json_schema)
}

fn build_schema_document_report(
    descriptor: &htmlcut_core::SchemaDescriptor,
) -> Result<SchemaDocumentReport, CliError> {
    let (surface, profile) = public_schema_surface(descriptor.owner);
    Ok(SchemaDocumentReport {
        schema_name: descriptor.schema_ref.schema_name.to_owned(),
        schema_version: descriptor.schema_ref.schema_version,
        surface,
        profile,
        artifact: descriptor.contract_family.to_owned(),
        stability: descriptor.stability,
        json_schema: (descriptor.json_schema)().map_err(schema_export_error)?,
    })
}

fn build_schema_inventory_entry(
    descriptor: &htmlcut_core::SchemaDescriptor,
) -> SchemaInventoryEntryReport {
    let (surface, profile) = public_schema_surface(descriptor.owner);
    SchemaInventoryEntryReport {
        schema_name: descriptor.schema_ref.schema_name.to_owned(),
        schema_version: descriptor.schema_ref.schema_version,
        surface,
        profile,
        artifact: descriptor.contract_family.to_owned(),
        stability: descriptor.stability,
    }
}

fn public_schema_surface(owner: &str) -> (String, Option<String>) {
    match owner {
        "core" => ("engine".to_owned(), None),
        "cli" => ("cli".to_owned(), None),
        "interop-v1" => ("integration".to_owned(), Some("htmlcut-v1".to_owned())),
        other => (other.to_owned(), None),
    }
}

fn schema_json_for<T: schemars::JsonSchema>(
    schema_ref: htmlcut_core::SchemaRef,
) -> Result<Value, htmlcut_core::SchemaExportError> {
    serde_json::to_value(schema_for!(T))
        .map_err(|error| schema_export_serialize_error(schema_ref, error))
}

fn schema_export_serialize_error(
    schema_ref: htmlcut_core::SchemaRef,
    error: serde_json::Error,
) -> htmlcut_core::SchemaExportError {
    htmlcut_core::SchemaExportError::Serialize {
        schema_name: schema_ref.schema_name,
        schema_version: schema_ref.schema_version,
        message: error.to_string(),
    }
}

fn extraction_command_report_schema() -> Result<Value, htmlcut_core::SchemaExportError> {
    schema_json_for::<ExtractionCommandReport>(htmlcut_core::SchemaRef::new(
        EXTRACTION_COMMAND_REPORT_SCHEMA_NAME,
        EXTRACTION_COMMAND_REPORT_SCHEMA_VERSION,
    ))
}

fn source_inspection_command_report_schema() -> Result<Value, htmlcut_core::SchemaExportError> {
    schema_json_for::<SourceInspectionCommandReport>(htmlcut_core::SchemaRef::new(
        SOURCE_INSPECTION_COMMAND_REPORT_SCHEMA_NAME,
        SOURCE_INSPECTION_COMMAND_REPORT_SCHEMA_VERSION,
    ))
}

fn catalog_command_report_schema() -> Result<Value, htmlcut_core::SchemaExportError> {
    schema_json_for::<CatalogCommandReport>(htmlcut_core::SchemaRef::new(
        CATALOG_REPORT_SCHEMA_NAME,
        CATALOG_SCHEMA_VERSION,
    ))
}

fn schema_command_report_schema() -> Result<Value, htmlcut_core::SchemaExportError> {
    schema_json_for::<SchemaCommandReport>(htmlcut_core::SchemaRef::new(
        SCHEMA_COMMAND_REPORT_SCHEMA_NAME,
        SCHEMA_COMMAND_REPORT_SCHEMA_VERSION,
    ))
}

fn schema_inventory_command_report_schema() -> Result<Value, htmlcut_core::SchemaExportError> {
    schema_json_for::<SchemaInventoryCommandReport>(htmlcut_core::SchemaRef::new(
        SCHEMA_INVENTORY_REPORT_SCHEMA_NAME,
        SCHEMA_INVENTORY_REPORT_SCHEMA_VERSION,
    ))
}

fn error_command_report_schema() -> Result<Value, htmlcut_core::SchemaExportError> {
    schema_json_for::<ErrorCommandReport>(htmlcut_core::SchemaRef::new(
        ERROR_COMMAND_REPORT_SCHEMA_NAME,
        ERROR_COMMAND_REPORT_SCHEMA_VERSION,
    ))
}

fn schema_export_error(error: htmlcut_core::SchemaExportError) -> CliError {
    internal_error(CliErrorCode::SchemaExportFailed, error.to_string())
}

#[cfg(test)]
pub(crate) fn schema_export_serialize_error_for_tests(
    schema_ref: htmlcut_core::SchemaRef,
) -> htmlcut_core::SchemaExportError {
    schema_export_serialize_error(
        schema_ref,
        serde_json::Error::io(std::io::Error::other(
            "synthetic schema serialization failure",
        )),
    )
}

#[cfg(test)]
pub(crate) fn schema_export_error_for_tests(error: htmlcut_core::SchemaExportError) -> CliError {
    schema_export_error(error)
}

#[cfg(test)]
pub(crate) fn cli_schema_catalog_validation_errors_for_tests(
    catalog: &[htmlcut_core::SchemaDescriptor],
) -> Vec<String> {
    cli_schema_catalog_validation_errors(catalog)
}

#[cfg(test)]
pub(crate) fn cli_schema_catalog_for_tests() -> &'static [htmlcut_core::SchemaDescriptor] {
    cli_schema_catalog()
}

#[cfg(test)]
pub(crate) fn assert_cli_schema_catalog_for_tests(catalog: &[htmlcut_core::SchemaDescriptor]) {
    let errors = cli_schema_catalog_validation_errors(catalog);
    assert!(
        errors.is_empty(),
        "cli schema catalog drifted:\n- {}",
        errors.join("\n- ")
    );
}

#[cfg(test)]
fn cli_schema_catalog_validation_errors(catalog: &[htmlcut_core::SchemaDescriptor]) -> Vec<String> {
    let expected_refs = [
        htmlcut_core::SchemaRef::new(
            EXTRACTION_COMMAND_REPORT_SCHEMA_NAME,
            EXTRACTION_COMMAND_REPORT_SCHEMA_VERSION,
        ),
        htmlcut_core::SchemaRef::new(
            SOURCE_INSPECTION_COMMAND_REPORT_SCHEMA_NAME,
            SOURCE_INSPECTION_COMMAND_REPORT_SCHEMA_VERSION,
        ),
        htmlcut_core::SchemaRef::new(CATALOG_REPORT_SCHEMA_NAME, CATALOG_SCHEMA_VERSION),
        htmlcut_core::SchemaRef::new(
            ERROR_COMMAND_REPORT_SCHEMA_NAME,
            ERROR_COMMAND_REPORT_SCHEMA_VERSION,
        ),
        htmlcut_core::SchemaRef::new(
            SCHEMA_COMMAND_REPORT_SCHEMA_NAME,
            SCHEMA_COMMAND_REPORT_SCHEMA_VERSION,
        ),
        htmlcut_core::SchemaRef::new(
            SCHEMA_INVENTORY_REPORT_SCHEMA_NAME,
            SCHEMA_INVENTORY_REPORT_SCHEMA_VERSION,
        ),
    ]
    .into_iter()
    .collect::<BTreeSet<_>>();
    let mut errors = Vec::new();
    let mut seen_refs = BTreeSet::new();

    for descriptor in catalog {
        if !seen_refs.insert(descriptor.schema_ref) {
            errors.push(format!(
                "{}@{} appears more than once in cli_schema_catalog()",
                descriptor.schema_ref.schema_name, descriptor.schema_ref.schema_version
            ));
        }
        if descriptor.owner != "cli" {
            errors.push(format!(
                "{}@{} owner drifted: expected \"cli\", found {:?}",
                descriptor.schema_ref.schema_name,
                descriptor.schema_ref.schema_version,
                descriptor.owner
            ));
        }

        if !expected_refs.contains(&descriptor.schema_ref) {
            errors.push(format!(
                "{}@{} is not part of the maintained CLI schema inventory",
                descriptor.schema_ref.schema_name, descriptor.schema_ref.schema_version
            ));
        }
    }

    for schema_ref in expected_refs {
        if !seen_refs.contains(&schema_ref) {
            errors.push(format!(
                "{}@{} is missing from cli_schema_catalog()",
                schema_ref.schema_name, schema_ref.schema_version
            ));
        }
    }

    errors
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_inventory_and_surface_helpers_cover_inventory_and_unknown_owners() {
        let inventory = build_schema_inventory_report(
            Some(htmlcut_core::interop::v1::RESULT_SCHEMA_NAME),
            Some(htmlcut_core::interop::v1::RESULT_SCHEMA_VERSION),
        )
        .expect("schema inventory");
        assert_eq!(inventory.command, "schema");
        assert_eq!(inventory.schemas.len(), 1);
        assert_eq!(
            inventory.schemas[0].schema_name,
            htmlcut_core::interop::v1::RESULT_SCHEMA_NAME
        );
        assert_eq!(inventory.schemas[0].surface, "integration");
        assert_eq!(inventory.schemas[0].profile.as_deref(), Some("htmlcut-v1"));

        assert_eq!(
            public_schema_surface("synthetic-owner"),
            ("synthetic-owner".to_owned(), None)
        );
    }
}
