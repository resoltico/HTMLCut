use super::*;

#[test]
fn direct_render_helpers_cover_empty_optional_branches() {
    let minimal_contract = CatalogCommandContract {
        invocation: "htmlcut select <INPUT>".to_owned(),
        inputs: Vec::new(),
        default_match: None,
        selection_modes: Vec::new(),
        default_value: None,
        value_modes: Vec::new(),
        default_output: None,
        default_output_overrides: Vec::new(),
        output_modes: Vec::new(),
        constraints: Vec::new(),
        notes: Vec::new(),
        examples: Vec::new(),
        parameters: Vec::new(),
    };
    let minimal_report = CatalogCommandReport {
        tool: TOOL_NAME.to_owned(),
        version: HTMLCUT_VERSION.to_owned(),
        schema_name: CATALOG_REPORT_SCHEMA_NAME.to_owned(),
        schema_version: crate::model::CATALOG_SCHEMA_VERSION,
        schema_profile: htmlcut_core::HTMLCUT_JSON_SCHEMA_PROFILE.to_owned(),
        description: HTMLCUT_DESCRIPTION.to_owned(),
        command: "catalog".to_owned(),
        operations: vec![CatalogOperationReport {
            operation_id: htmlcut_core::OperationId::DocumentParse,
            command: Some("select".to_owned()),
            availability: CatalogAvailability::Cli,
            summary: "Minimal".to_owned(),
            core_surface: "BareCoreSurface".to_owned(),
            request_contract: CatalogContractSurface {
                rust_shape: "BareShape".to_owned(),
                schema_refs: Vec::new(),
            },
            result_contract: CatalogContractSurface {
                rust_shape: "BareResult".to_owned(),
                schema_refs: Vec::new(),
            },
            command_contract: Some(minimal_contract),
        }],
    };
    let minimal_render = render_catalog_text(&minimal_report);
    assert!(minimal_render.contains("usage: htmlcut select <INPUT>"));
    assert!(minimal_render.contains("request: BareShape"));
    assert!(minimal_render.contains("result: BareResult"));
    assert!(!minimal_render.contains("inputs:"));
    assert!(!minimal_render.contains("default output:"));
    assert!(!minimal_render.contains("constraints:"));
    assert!(!minimal_render.contains("parameters:"));

    let focused_render = render_catalog_text(&CatalogCommandReport {
        operations: vec![CatalogOperationReport {
            operation_id: htmlcut_core::OperationId::SelectExtract,
            command: Some("select".to_owned()),
            availability: CatalogAvailability::Cli,
            summary: "Focused".to_owned(),
            core_surface: "FocusedCoreSurface".to_owned(),
            request_contract: CatalogContractSurface {
                rust_shape: "FocusedRequest".to_owned(),
                schema_refs: Vec::new(),
            },
            result_contract: CatalogContractSurface {
                rust_shape: "FocusedResult".to_owned(),
                schema_refs: Vec::new(),
            },
            command_contract: Some(CatalogCommandContract {
                invocation: "htmlcut select <INPUT>".to_owned(),
                inputs: vec!["file".to_owned(), "url".to_owned()],
                default_match: None,
                selection_modes: Vec::new(),
                default_value: None,
                value_modes: Vec::new(),
                default_output: Some("text".to_owned()),
                default_output_overrides: Vec::new(),
                output_modes: Vec::new(),
                constraints: vec![CatalogConstraint::RequiresParameter {
                    parameter: "--thing".to_owned(),
                    when: CatalogCondition {
                        parameter: "--mode".to_owned(),
                        values: Vec::new(),
                    },
                }],
                notes: Vec::new(),
                examples: Vec::new(),
                parameters: vec![
                    CatalogParameterSpec {
                        section: "Synthetic".to_owned(),
                        name: "--flag".to_owned(),
                        kind: CatalogParameterKind::Flag,
                        requirement: CatalogParameterRequirement::Optional,
                        requirement_note: None,
                        value_hint: Some("IGNORED".to_owned()),
                        default: None,
                        allowed_values: Vec::new(),
                        summary: "Synthetic flag.".to_owned(),
                    },
                    CatalogParameterSpec {
                        section: "Synthetic".to_owned(),
                        name: "--conditional".to_owned(),
                        kind: CatalogParameterKind::Option,
                        requirement: CatalogParameterRequirement::Conditional,
                        requirement_note: None,
                        value_hint: Some("VALUE".to_owned()),
                        default: None,
                        allowed_values: Vec::new(),
                        summary: "Synthetic conditional.".to_owned(),
                    },
                ],
            }),
        }],
        ..minimal_report
    });
    assert!(focused_render.contains("inputs: file | url"));
    assert!(focused_render.contains("default output: text"));
    assert!(focused_render.contains("requires --thing when --mode"));
    assert!(focused_render.contains("flag --flag | optional"));
    assert!(
        focused_render.contains("option --conditional <VALUE> | conditional (see command notes)")
    );

    let empty_schema_report = SchemaCommandReport {
        tool: TOOL_NAME.to_owned(),
        version: HTMLCUT_VERSION.to_owned(),
        schema_name: SCHEMA_COMMAND_REPORT_SCHEMA_NAME.to_owned(),
        schema_version: SCHEMA_COMMAND_REPORT_SCHEMA_VERSION,
        schema_profile: htmlcut_core::HTMLCUT_JSON_SCHEMA_PROFILE.to_owned(),
        description: HTMLCUT_DESCRIPTION.to_owned(),
        command: "schema".to_owned(),
        schemas: Vec::new(),
    };
    let empty_schema_text = render_schema_text(&empty_schema_report);
    assert!(!empty_schema_text.contains("Schema:"));
    assert!(!empty_schema_text.contains("Schemas:"));
    assert!(empty_schema_text.contains("Schema profile:"));
}
