use std::any::type_name;
#[cfg(test)]
use std::collections::BTreeSet;
use std::sync::LazyLock;

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
            CliErrorCode::SchemaVersionRequiresName,
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
        .collect::<Result<Vec<_>, _>>()?;

    schemas.sort_by(|left, right| {
        left.schema_name
            .cmp(&right.schema_name)
            .then(left.schema_version.cmp(&right.schema_version))
    });

    let filtered = if let Some(name) = name_filter {
        let filtered = schemas
            .iter()
            .filter(|schema| schema.schema_name == name)
            .filter(|schema| version_filter.is_none_or(|version| schema.schema_version == version))
            .cloned()
            .collect::<Vec<_>>();

        if filtered.is_empty() {
            return Err(unknown_schema_error(name, version_filter, &schemas));
        }

        filtered
    } else {
        schemas
    };

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

static CLI_SCHEMA_CATALOG: LazyLock<Vec<htmlcut_core::SchemaDescriptor>> = LazyLock::new(|| {
    vec![
        cli_schema_descriptor::<ExtractionCommandReport>(
            htmlcut_core::SchemaRef::new(
                EXTRACTION_COMMAND_REPORT_SCHEMA_NAME,
                EXTRACTION_COMMAND_REPORT_SCHEMA_VERSION,
            ),
            extraction_command_report_schema,
        ),
        cli_schema_descriptor::<SourceInspectionCommandReport>(
            htmlcut_core::SchemaRef::new(
                SOURCE_INSPECTION_COMMAND_REPORT_SCHEMA_NAME,
                SOURCE_INSPECTION_COMMAND_REPORT_SCHEMA_VERSION,
            ),
            source_inspection_command_report_schema,
        ),
        cli_schema_descriptor::<CatalogCommandReport>(
            htmlcut_core::SchemaRef::new(CATALOG_REPORT_SCHEMA_NAME, CATALOG_SCHEMA_VERSION),
            catalog_command_report_schema,
        ),
        cli_schema_descriptor::<ErrorCommandReport>(
            htmlcut_core::SchemaRef::new(
                ERROR_COMMAND_REPORT_SCHEMA_NAME,
                ERROR_COMMAND_REPORT_SCHEMA_VERSION,
            ),
            error_command_report_schema,
        ),
        cli_schema_descriptor::<SchemaCommandReport>(
            htmlcut_core::SchemaRef::new(
                SCHEMA_COMMAND_REPORT_SCHEMA_NAME,
                SCHEMA_COMMAND_REPORT_SCHEMA_VERSION,
            ),
            schema_command_report_schema,
        ),
    ]
});

fn cli_schema_catalog() -> &'static [htmlcut_core::SchemaDescriptor] {
    CLI_SCHEMA_CATALOG.as_slice()
}

fn cli_schema_descriptor<T>(
    schema_ref: htmlcut_core::SchemaRef,
    json_schema: fn() -> Result<Value, htmlcut_core::SchemaExportError>,
) -> htmlcut_core::SchemaDescriptor {
    htmlcut_core::SchemaDescriptor {
        schema_ref,
        owner_surface: "htmlcut-cli",
        rust_shape: short_type_name::<T>(),
        stability: htmlcut_core::SchemaStability::Versioned,
        json_schema,
    }
}

fn build_schema_document_report(
    descriptor: &htmlcut_core::SchemaDescriptor,
) -> Result<SchemaDocumentReport, CliError> {
    Ok(SchemaDocumentReport {
        schema_name: descriptor.schema_ref.schema_name.to_owned(),
        schema_version: descriptor.schema_ref.schema_version,
        owner_surface: descriptor.owner_surface.to_owned(),
        rust_shape: descriptor.rust_shape.to_owned(),
        stability: descriptor.stability,
        json_schema: (descriptor.json_schema)().map_err(schema_export_error)?,
    })
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
        if descriptor.owner_surface != "htmlcut-cli" {
            errors.push(format!(
                "{}@{} owner_surface drifted: expected \"htmlcut-cli\", found {:?}",
                descriptor.schema_ref.schema_name,
                descriptor.schema_ref.schema_version,
                descriptor.owner_surface
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
fn short_type_name<T>() -> &'static str {
    type_name::<T>()
        .rsplit("::")
        .next()
        .unwrap_or(type_name::<T>())
}
