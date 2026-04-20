use htmlcut_core::{
    ExtractionResult, HTMLCUT_JSON_SCHEMA_PROFILE, SchemaStability, SourceInspectionResult,
};
use schemars::schema_for;
use serde_json::Value;

use crate::error::CliError;
use crate::lookup::{unknown_operation_id_error, unknown_schema_error};
use crate::metadata::{ENGINE_NAME, HTMLCUT_DESCRIPTION, HTMLCUT_VERSION, TOOL_NAME};
use crate::model::{
    BundlePaths, CATALOG_REPORT_SCHEMA_NAME, CATALOG_SCHEMA_VERSION, CatalogAvailability,
    CatalogCommandContract, CatalogCommandReport, CatalogCondition, CatalogConditionalDefault,
    CatalogConstraint, CatalogContractSurface, CatalogOperationReport, CatalogParameterKind,
    CatalogParameterRequirement, CatalogParameterSpec, EXTRACTION_COMMAND_REPORT_SCHEMA_NAME,
    EXTRACTION_COMMAND_REPORT_SCHEMA_VERSION, ExtractionCommandReport,
    SCHEMA_COMMAND_REPORT_SCHEMA_NAME, SCHEMA_COMMAND_REPORT_SCHEMA_VERSION,
    SOURCE_INSPECTION_COMMAND_REPORT_SCHEMA_NAME, SOURCE_INSPECTION_COMMAND_REPORT_SCHEMA_VERSION,
    SchemaCommandReport, SchemaDocumentReport, SchemaRefReport, SourceInspectionCommandReport,
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

pub(crate) fn build_catalog_report(
    operation_filter: Option<&str>,
) -> Result<CatalogCommandReport, CliError> {
    let requested_operation = operation_filter
        .map(|operation_id| {
            operation_id
                .parse::<htmlcut_core::OperationId>()
                .map_err(|_| unknown_operation_id_error(operation_id))
        })
        .transpose()?;

    let operations = htmlcut_core::operation_catalog()
        .iter()
        .filter(|descriptor| {
            requested_operation.is_none_or(|operation_id| descriptor.id == operation_id)
        })
        .map(|descriptor| {
            let cli_contract = htmlcut_core::cli_operation_contract(descriptor.id);
            CatalogOperationReport {
                operation_id: descriptor.id,
                command: cli_contract.map(|contract| contract.display_command()),
                availability: match cli_contract {
                    Some(_) => CatalogAvailability::Cli,
                    None => CatalogAvailability::CoreOnly,
                },
                summary: descriptor.description.to_owned(),
                core_surface: descriptor.core_surface.to_owned(),
                request_contract: build_contract_surface(&descriptor.request_contract),
                result_contract: build_contract_surface(&descriptor.result_contract),
                command_contract: cli_contract.map(build_catalog_command_contract),
            }
        })
        .collect::<Vec<_>>();

    Ok(CatalogCommandReport {
        tool: TOOL_NAME.to_owned(),
        version: HTMLCUT_VERSION.to_owned(),
        schema_name: CATALOG_REPORT_SCHEMA_NAME.to_owned(),
        schema_version: CATALOG_SCHEMA_VERSION,
        schema_profile: HTMLCUT_JSON_SCHEMA_PROFILE.to_owned(),
        description: HTMLCUT_DESCRIPTION.to_owned(),
        command: "catalog".to_owned(),
        operations,
    })
}

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
        schema_profile: HTMLCUT_JSON_SCHEMA_PROFILE.to_owned(),
        description: HTMLCUT_DESCRIPTION.to_owned(),
        command: "schema".to_owned(),
        schemas: filtered,
    })
}

fn build_contract_surface(contract: &htmlcut_core::OperationContract) -> CatalogContractSurface {
    CatalogContractSurface {
        rust_shape: contract.rust_shape.to_owned(),
        schema_refs: contract
            .schema_refs
            .iter()
            .map(build_schema_ref_report)
            .collect(),
    }
}

fn build_schema_ref_report(schema_ref: &htmlcut_core::SchemaRef) -> SchemaRefReport {
    SchemaRefReport {
        schema_name: schema_ref.schema_name.to_owned(),
        schema_version: schema_ref.schema_version,
    }
}

const CLI_SCHEMA_CATALOG: &[htmlcut_core::SchemaDescriptor] = &[
    htmlcut_core::SchemaDescriptor {
        schema_ref: htmlcut_core::SchemaRef::new(
            EXTRACTION_COMMAND_REPORT_SCHEMA_NAME,
            EXTRACTION_COMMAND_REPORT_SCHEMA_VERSION,
        ),
        owner_surface: "htmlcut-cli",
        rust_shape: "ExtractionCommandReport",
        stability: SchemaStability::Versioned,
        json_schema: extraction_command_report_schema,
    },
    htmlcut_core::SchemaDescriptor {
        schema_ref: htmlcut_core::SchemaRef::new(
            SOURCE_INSPECTION_COMMAND_REPORT_SCHEMA_NAME,
            SOURCE_INSPECTION_COMMAND_REPORT_SCHEMA_VERSION,
        ),
        owner_surface: "htmlcut-cli",
        rust_shape: "SourceInspectionCommandReport",
        stability: SchemaStability::Versioned,
        json_schema: source_inspection_command_report_schema,
    },
    htmlcut_core::SchemaDescriptor {
        schema_ref: htmlcut_core::SchemaRef::new(
            CATALOG_REPORT_SCHEMA_NAME,
            CATALOG_SCHEMA_VERSION,
        ),
        owner_surface: "htmlcut-cli",
        rust_shape: "CatalogCommandReport",
        stability: SchemaStability::Versioned,
        json_schema: catalog_command_report_schema,
    },
    htmlcut_core::SchemaDescriptor {
        schema_ref: htmlcut_core::SchemaRef::new(
            SCHEMA_COMMAND_REPORT_SCHEMA_NAME,
            SCHEMA_COMMAND_REPORT_SCHEMA_VERSION,
        ),
        owner_surface: "htmlcut-cli",
        rust_shape: "SchemaCommandReport",
        stability: SchemaStability::Versioned,
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

fn build_catalog_command_contract(
    descriptor: &htmlcut_core::OperationCliContract,
) -> CatalogCommandContract {
    CatalogCommandContract {
        invocation: descriptor.invocation.to_owned(),
        inputs: descriptor
            .inputs
            .iter()
            .copied()
            .map(|input| input.description().to_owned())
            .collect(),
        default_match: descriptor.default_match.map(render_selection_mode),
        selection_modes: descriptor
            .selection_modes
            .iter()
            .copied()
            .map(render_selection_mode)
            .collect(),
        default_value: descriptor.default_value.map(render_value_type),
        value_modes: descriptor
            .value_modes
            .iter()
            .copied()
            .map(render_value_type)
            .collect(),
        default_output: descriptor.default_output.map(render_output_mode),
        default_output_overrides: descriptor
            .default_output_overrides
            .iter()
            .map(build_conditional_default)
            .collect(),
        output_modes: descriptor
            .output_modes
            .iter()
            .copied()
            .map(render_output_mode)
            .collect(),
        constraints: descriptor
            .constraints
            .iter()
            .map(build_constraint)
            .collect(),
        notes: descriptor
            .notes
            .iter()
            .map(|note| (*note).to_owned())
            .collect(),
        examples: descriptor
            .examples
            .iter()
            .map(|example| (*example).to_owned())
            .collect(),
        parameters: descriptor
            .parameters
            .iter()
            .map(build_parameter_spec)
            .collect(),
    }
}

fn build_conditional_default(
    descriptor: &htmlcut_core::CliConditionalDefault,
) -> CatalogConditionalDefault {
    CatalogConditionalDefault {
        value: htmlcut_core::render_cli_value(descriptor.value),
        when: build_condition(&descriptor.when),
    }
}

fn build_constraint(descriptor: &htmlcut_core::CliConstraint) -> CatalogConstraint {
    match descriptor {
        htmlcut_core::CliConstraint::RequiresParameter { parameter, when } => {
            CatalogConstraint::RequiresParameter {
                parameter: parameter.to_string(),
                when: build_condition(when),
            }
        }
        htmlcut_core::CliConstraint::AllowedOnlyWhen { parameter, when } => {
            CatalogConstraint::AllowedOnlyWhen {
                parameter: parameter.to_string(),
                when: build_condition(when),
            }
        }
        htmlcut_core::CliConstraint::RestrictsParameterValues {
            parameter,
            allowed_values,
            when,
        } => CatalogConstraint::RestrictsParameterValues {
            parameter: parameter.to_string(),
            allowed_values: allowed_values
                .iter()
                .copied()
                .map(htmlcut_core::render_cli_value)
                .collect(),
            when: build_condition(when),
        },
    }
}

fn build_condition(condition: &htmlcut_core::CliCondition) -> CatalogCondition {
    CatalogCondition {
        parameter: condition.parameter.to_string(),
        values: condition
            .values
            .iter()
            .copied()
            .map(htmlcut_core::render_cli_value)
            .collect(),
    }
}

fn build_parameter_spec(parameter: &htmlcut_core::CliParameterDescriptor) -> CatalogParameterSpec {
    let (requirement, requirement_note) = render_parameter_requirement(&parameter.requirement);
    CatalogParameterSpec {
        section: parameter.section.to_string(),
        name: parameter.id.to_string(),
        kind: match parameter.kind {
            htmlcut_core::CliParameterKind::Positional => CatalogParameterKind::Positional,
            htmlcut_core::CliParameterKind::Option => CatalogParameterKind::Option,
            htmlcut_core::CliParameterKind::Flag => CatalogParameterKind::Flag,
        },
        requirement,
        requirement_note,
        value_hint: parameter.value_hint.map(str::to_owned),
        default: parameter.default.map(htmlcut_core::render_cli_value),
        allowed_values: parameter
            .allowed_values
            .iter()
            .copied()
            .map(htmlcut_core::render_cli_value)
            .collect(),
        summary: parameter.summary.to_owned(),
    }
}

fn render_parameter_requirement(
    requirement: &htmlcut_core::CliParameterRequirement,
) -> (CatalogParameterRequirement, Option<String>) {
    match requirement {
        htmlcut_core::CliParameterRequirement::Required => {
            (CatalogParameterRequirement::Required, None)
        }
        htmlcut_core::CliParameterRequirement::Optional => {
            (CatalogParameterRequirement::Optional, None)
        }
        htmlcut_core::CliParameterRequirement::RequiredUnless(parameter) => (
            CatalogParameterRequirement::Conditional,
            Some(format!("required unless {parameter} is used")),
        ),
        htmlcut_core::CliParameterRequirement::RequiredWhen(condition) => (
            CatalogParameterRequirement::Conditional,
            Some(format!(
                "required when {}",
                render_condition_expression(condition)
            )),
        ),
        htmlcut_core::CliParameterRequirement::AllowedOnlyWhen(condition) => (
            CatalogParameterRequirement::Conditional,
            Some(format!(
                "allowed only when {}",
                render_condition_expression(condition)
            )),
        ),
    }
}

fn render_condition_expression(condition: &htmlcut_core::CliCondition) -> String {
    let values = condition
        .values
        .iter()
        .copied()
        .map(htmlcut_core::render_cli_value)
        .collect::<Vec<_>>();

    match values.as_slice() {
        [single] => format!("{} {single} is used", condition.parameter),
        _ => format!("{} is one of {}", condition.parameter, values.join(", ")),
    }
}

#[cfg(test)]
pub(crate) fn render_condition_expression_for_tests(
    condition: &htmlcut_core::CliCondition,
) -> String {
    render_condition_expression(condition)
}

fn render_selection_mode(mode: htmlcut_core::CliSelectionMode) -> String {
    htmlcut_core::render_cli_value(htmlcut_core::CliValue::SelectionMode(mode))
}

fn render_value_type(value_type: htmlcut_core::ValueType) -> String {
    htmlcut_core::render_cli_value(htmlcut_core::CliValue::ValueType(value_type))
}

fn render_output_mode(mode: htmlcut_core::CliOutputMode) -> String {
    htmlcut_core::render_cli_value(htmlcut_core::CliValue::OutputMode(mode))
}
