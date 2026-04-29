use crate::catalog::{OperationContract, OperationDescriptor, OperationId};
use crate::contracts::ValueType;
use crate::{
    CORE_REQUEST_SCHEMA_VERSION, CORE_RESULT_SCHEMA_NAME, CORE_RESULT_SCHEMA_VERSION,
    CORE_SOURCE_INSPECTION_SCHEMA_NAME, CORE_SOURCE_INSPECTION_SCHEMA_VERSION,
    EXTRACTION_REQUEST_SCHEMA_NAME, INSPECTION_OPTIONS_SCHEMA_NAME, RUNTIME_OPTIONS_SCHEMA_NAME,
    SOURCE_REQUEST_SCHEMA_NAME, SchemaRef,
};

use super::parameters::{
    common_input_forms, common_selection_modes, condition, conditional_default,
    constraints_with_parameter_rules, extract_output_modes, extract_value_modes,
    inspect_output_modes, inspect_select_parameters, inspect_slice_parameters,
    inspect_source_parameters, requires_parameter, restricts_parameter_values,
    select_extract_parameters, slice_extract_parameters,
};
use super::{
    CliConditionalDefault, CliConstraint, CliOutputMode, CliParameterDescriptor, CliParameterId,
    CliSelectionMode, CliValue, OperationCliContract,
};

const NO_SCHEMA_REFS: &[SchemaRef] = &[];
const SOURCE_RUNTIME_SCHEMA_REFS: &[SchemaRef] = &[
    SchemaRef::new(SOURCE_REQUEST_SCHEMA_NAME, CORE_REQUEST_SCHEMA_VERSION),
    SchemaRef::new(RUNTIME_OPTIONS_SCHEMA_NAME, CORE_REQUEST_SCHEMA_VERSION),
];
const SOURCE_RUNTIME_INSPECTION_SCHEMA_REFS: &[SchemaRef] = &[
    SchemaRef::new(SOURCE_REQUEST_SCHEMA_NAME, CORE_REQUEST_SCHEMA_VERSION),
    SchemaRef::new(RUNTIME_OPTIONS_SCHEMA_NAME, CORE_REQUEST_SCHEMA_VERSION),
    SchemaRef::new(INSPECTION_OPTIONS_SCHEMA_NAME, CORE_REQUEST_SCHEMA_VERSION),
];
const EXTRACTION_RUNTIME_SCHEMA_REFS: &[SchemaRef] = &[
    SchemaRef::new(EXTRACTION_REQUEST_SCHEMA_NAME, CORE_REQUEST_SCHEMA_VERSION),
    SchemaRef::new(RUNTIME_OPTIONS_SCHEMA_NAME, CORE_REQUEST_SCHEMA_VERSION),
];
const EXTRACTION_RESULT_SCHEMA_REFS: &[SchemaRef] = &[SchemaRef::new(
    CORE_RESULT_SCHEMA_NAME,
    CORE_RESULT_SCHEMA_VERSION,
)];
const SOURCE_INSPECTION_RESULT_SCHEMA_REFS: &[SchemaRef] = &[SchemaRef::new(
    CORE_SOURCE_INSPECTION_SCHEMA_NAME,
    CORE_SOURCE_INSPECTION_SCHEMA_VERSION,
)];

pub(crate) struct OperationCliSpec {
    pub(crate) command_path: &'static [&'static str],
    pub(crate) invocation: &'static str,
    pub(crate) default_match: Option<CliSelectionMode>,
    pub(crate) selection_modes: fn() -> Vec<CliSelectionMode>,
    pub(crate) default_value: Option<ValueType>,
    pub(crate) value_modes: fn() -> Vec<ValueType>,
    pub(crate) default_output: Option<CliOutputMode>,
    pub(crate) default_output_overrides: fn() -> Vec<CliConditionalDefault>,
    pub(crate) output_modes: fn() -> Vec<CliOutputMode>,
    pub(crate) build_parameters: fn() -> Vec<CliParameterDescriptor>,
    pub(crate) build_constraints: fn(&[CliParameterDescriptor]) -> Vec<CliConstraint>,
    pub(crate) notes: &'static [&'static str],
    pub(crate) examples: &'static [&'static str],
    pub(crate) help_overview: &'static [&'static str],
}

pub(crate) struct OperationSurfaceSpec {
    pub(crate) descriptor: OperationDescriptor,
    pub(crate) cli: Option<OperationCliSpec>,
}

pub(crate) fn operation_surface_specs() -> &'static [OperationSurfaceSpec] {
    OPERATION_SURFACE_SPECS
}

pub(crate) fn operation_surface_spec(
    operation_id: OperationId,
) -> Option<&'static OperationSurfaceSpec> {
    operation_surface_specs()
        .iter()
        .find(|spec| spec.descriptor.id == operation_id)
}

pub(crate) fn build_cli_operation_contract(
    spec: &'static OperationSurfaceSpec,
) -> Option<OperationCliContract> {
    let cli = spec.cli.as_ref()?;
    let parameters = (cli.build_parameters)();

    Some(OperationCliContract {
        operation_id: spec.descriptor.id,
        command_path: cli.command_path,
        invocation: cli.invocation,
        inputs: common_input_forms(),
        default_match: cli.default_match,
        selection_modes: (cli.selection_modes)(),
        default_value: cli.default_value,
        value_modes: (cli.value_modes)(),
        default_output: cli.default_output,
        default_output_overrides: (cli.default_output_overrides)(),
        output_modes: (cli.output_modes)(),
        constraints: (cli.build_constraints)(&parameters),
        notes: cli.notes.to_vec(),
        examples: cli.examples.to_vec(),
        parameters,
    })
}

fn empty_selection_modes() -> Vec<CliSelectionMode> {
    Vec::new()
}

fn empty_value_modes() -> Vec<ValueType> {
    Vec::new()
}

fn structured_value_modes() -> Vec<ValueType> {
    vec![ValueType::Structured]
}

fn no_output_overrides() -> Vec<CliConditionalDefault> {
    Vec::new()
}

fn no_constraints(parameters: &[CliParameterDescriptor]) -> Vec<CliConstraint> {
    constraints_with_parameter_rules(parameters, Vec::new())
}

fn extract_default_output_overrides() -> Vec<CliConditionalDefault> {
    vec![
        conditional_default(
            CliValue::OutputMode(CliOutputMode::Html),
            condition(
                CliParameterId::Value,
                vec![
                    CliValue::ValueType(ValueType::InnerHtml),
                    CliValue::ValueType(ValueType::OuterHtml),
                ],
            ),
        ),
        conditional_default(
            CliValue::OutputMode(CliOutputMode::Json),
            condition(
                CliParameterId::Value,
                vec![CliValue::ValueType(ValueType::Structured)],
            ),
        ),
    ]
}

fn extract_constraints(parameters: &[CliParameterDescriptor]) -> Vec<CliConstraint> {
    constraints_with_parameter_rules(
        parameters,
        vec![
            requires_parameter(
                CliParameterId::Bundle,
                condition(
                    CliParameterId::Output,
                    vec![CliValue::OutputMode(CliOutputMode::None)],
                ),
            ),
            restricts_parameter_values(
                CliParameterId::Output,
                vec![
                    CliValue::OutputMode(CliOutputMode::Json),
                    CliValue::OutputMode(CliOutputMode::None),
                ],
                condition(
                    CliParameterId::Value,
                    vec![CliValue::ValueType(ValueType::Structured)],
                ),
            ),
            restricts_parameter_values(
                CliParameterId::Value,
                vec![
                    CliValue::ValueType(ValueType::InnerHtml),
                    CliValue::ValueType(ValueType::OuterHtml),
                ],
                condition(
                    CliParameterId::Output,
                    vec![CliValue::OutputMode(CliOutputMode::Html)],
                ),
            ),
        ],
    )
}

const OPERATION_SURFACE_SPECS: &[OperationSurfaceSpec] = &[
    OperationSurfaceSpec {
        descriptor: OperationDescriptor {
            id: OperationId::DocumentParse,
            cli_surface: None,
            core_surface: "parse_document(SourceRequest, RuntimeOptions)",
            request_contract: OperationContract {
                rust_shape: "SourceRequest + RuntimeOptions",
                schema_refs: SOURCE_RUNTIME_SCHEMA_REFS,
            },
            result_contract: OperationContract {
                rust_shape: "ParseDocumentResult",
                schema_refs: NO_SCHEMA_REFS,
            },
            description: "Load and parse HTML into a document tree for in-process callers.",
        },
        cli: None,
    },
    OperationSurfaceSpec {
        descriptor: OperationDescriptor {
            id: OperationId::SourceInspect,
            cli_surface: Some("inspect source"),
            core_surface: "inspect_source(SourceRequest, RuntimeOptions, InspectionOptions)",
            request_contract: OperationContract {
                rust_shape: "SourceRequest + RuntimeOptions + InspectionOptions",
                schema_refs: SOURCE_RUNTIME_INSPECTION_SCHEMA_REFS,
            },
            result_contract: OperationContract {
                rust_shape: "SourceInspectionResult",
                schema_refs: SOURCE_INSPECTION_RESULT_SCHEMA_REFS,
            },
            description: "Inspect the parsed document and summarize structure, samples, and base-URL behavior.",
        },
        cli: Some(OperationCliSpec {
            command_path: &["inspect", "source"],
            invocation: "htmlcut inspect source [OPTIONS] [INPUT]",
            default_match: None,
            selection_modes: empty_selection_modes,
            default_value: None,
            value_modes: empty_value_modes,
            default_output: Some(CliOutputMode::Json),
            default_output_overrides: no_output_overrides,
            output_modes: inspect_output_modes,
            build_parameters: inspect_source_parameters,
            build_constraints: no_constraints,
            notes: &[
                "Use this command to inspect document shape, headings, links, classes, and effective base-URL behavior before choosing selectors or slice boundaries.",
                "--include-source-text stores the full source in JSON output and prints a bounded source preview in text mode.",
                "--sample-limit bounds the sampled headings, links, tags, and classes in the summary.",
            ],
            examples: &[
                "htmlcut inspect source ./page.html",
                "htmlcut inspect source ./page.html --output text --include-source-text --preview-chars 200",
            ],
            help_overview: &[
                "This command summarizes title, counts, headings, link previews, top tags, top classes, document base behavior, and optional source text. It is designed to help you choose selectors or confirm how URL rewriting will behave before extracting data.",
            ],
        }),
    },
    OperationSurfaceSpec {
        descriptor: OperationDescriptor {
            id: OperationId::SelectPreview,
            cli_surface: Some("inspect select"),
            core_surface: "preview_extraction(ExtractionRequest{kind=selector}, RuntimeOptions)",
            request_contract: OperationContract {
                rust_shape: "ExtractionRequest + RuntimeOptions",
                schema_refs: EXTRACTION_RUNTIME_SCHEMA_REFS,
            },
            result_contract: OperationContract {
                rust_shape: "ExtractionResult",
                schema_refs: EXTRACTION_RESULT_SCHEMA_REFS,
            },
            description: "Preview selector matches without committing to a final extraction payload.",
        },
        cli: Some(OperationCliSpec {
            command_path: &["inspect", "select"],
            invocation: "htmlcut inspect select [OPTIONS] --css <CSS> [INPUT]",
            default_match: Some(CliSelectionMode::First),
            selection_modes: common_selection_modes,
            default_value: Some(ValueType::Structured),
            value_modes: structured_value_modes,
            default_output: Some(CliOutputMode::Json),
            default_output_overrides: no_output_overrides,
            output_modes: inspect_output_modes,
            build_parameters: inspect_select_parameters,
            build_constraints: no_constraints,
            notes: &[
                "inspect select always previews matches in structured form; it is a preview workflow, not a final extraction surface.",
                "When --rewrite-urls is requested but no effective base URL can be resolved, relative URLs stay unchanged and a warning is emitted.",
                "Use --emit-request-file to capture the canonical preview definition while you iterate on inline flags.",
            ],
            examples: &[
                "htmlcut inspect select ./page.html --css article --match single",
                "htmlcut inspect select ./page.html --css '.card' --match all --output text",
                "htmlcut inspect select ./page.html --css article --emit-request-file ./article-preview.json",
                "htmlcut inspect select --request-file ./article-preview.json",
            ],
            help_overview: &[
                "Use this preview workflow to inspect structured per-match metadata before final extraction.",
            ],
        }),
    },
    OperationSurfaceSpec {
        descriptor: OperationDescriptor {
            id: OperationId::SlicePreview,
            cli_surface: Some("inspect slice"),
            core_surface: "preview_extraction(ExtractionRequest{kind=slice}, RuntimeOptions)",
            request_contract: OperationContract {
                rust_shape: "ExtractionRequest + RuntimeOptions",
                schema_refs: EXTRACTION_RUNTIME_SCHEMA_REFS,
            },
            result_contract: OperationContract {
                rust_shape: "ExtractionResult",
                schema_refs: EXTRACTION_RESULT_SCHEMA_REFS,
            },
            description: "Preview literal or regex slices without committing to a final extraction payload.",
        },
        cli: Some(OperationCliSpec {
            command_path: &["inspect", "slice"],
            invocation: "htmlcut inspect slice [OPTIONS] --from <FROM> --to <TO> [INPUT]",
            default_match: Some(CliSelectionMode::First),
            selection_modes: common_selection_modes,
            default_value: Some(ValueType::Structured),
            value_modes: structured_value_modes,
            default_output: Some(CliOutputMode::Json),
            default_output_overrides: no_output_overrides,
            output_modes: inspect_output_modes,
            build_parameters: inspect_slice_parameters,
            build_constraints: no_constraints,
            notes: &[
                "Literal boundaries are raw substring matching, not tag-aware; `<a` also matches `<article>`.",
                "Previews exclude both matched boundaries by default unless --include-start and/or --include-end are supplied.",
                "Text output shows fragment context when it adds signal so boundary-consumption mistakes are easier to spot.",
                "Use --emit-request-file to capture the canonical preview definition while you iterate on inline flags.",
            ],
            examples: &[
                "htmlcut inspect slice ./page.html --from '<article>' --to '</article>'",
                "htmlcut inspect slice ./page.html --from 'START::' --to '::END' --pattern regex --output text",
                "htmlcut inspect slice ./page.html --from '<article>' --to '</article>' --emit-request-file ./article-slice-preview.json",
                "htmlcut inspect slice --request-file ./article-slice-preview.json",
            ],
            help_overview: &[
                "Use this preview workflow to inspect literal or regex slice ranges before final extraction.",
            ],
        }),
    },
    OperationSurfaceSpec {
        descriptor: OperationDescriptor {
            id: OperationId::SelectExtract,
            cli_surface: Some("select"),
            core_surface: "extract(ExtractionRequest{kind=selector}, RuntimeOptions)",
            request_contract: OperationContract {
                rust_shape: "ExtractionRequest + RuntimeOptions",
                schema_refs: EXTRACTION_RUNTIME_SCHEMA_REFS,
            },
            result_contract: OperationContract {
                rust_shape: "ExtractionResult",
                schema_refs: EXTRACTION_RESULT_SCHEMA_REFS,
            },
            description: "Extract final values from CSS selector matches.",
        },
        cli: Some(OperationCliSpec {
            command_path: &["select"],
            invocation: "htmlcut select [OPTIONS] --css <CSS> [INPUT]",
            default_match: Some(CliSelectionMode::First),
            selection_modes: common_selection_modes,
            default_value: Some(ValueType::Text),
            value_modes: extract_value_modes,
            default_output: Some(CliOutputMode::Text),
            default_output_overrides: extract_default_output_overrides,
            output_modes: extract_output_modes,
            build_parameters: select_extract_parameters,
            build_constraints: extract_constraints,
            notes: &[
                "Structured extraction only supports --output json or --output none.",
                "--output html is only valid with --value inner-html or --value outer-html.",
                "When --rewrite-urls is requested but no effective base URL can be resolved, relative URLs stay unchanged and a warning is emitted.",
                "Use --emit-request-file to capture the canonical extraction definition while you iterate on inline flags.",
            ],
            examples: &[
                "htmlcut select ./page.html --css article --match single",
                "htmlcut select ./page.html --css '.card' --match all --value outer-html",
                "htmlcut select ./page.html --css 'article a.more' --value attribute --attribute href --rewrite-urls",
                "htmlcut select ./page.html --css 'article a.more' --value attribute --attribute href --emit-request-file ./article-links.json",
                "htmlcut select --request-file ./article-links.json --output-file ./links.json",
            ],
            help_overview: &[
                "Use inspect source first when you need to learn the document shape, then inspect select to preview matches before emitting the final payload.",
            ],
        }),
    },
    OperationSurfaceSpec {
        descriptor: OperationDescriptor {
            id: OperationId::SliceExtract,
            cli_surface: Some("slice"),
            core_surface: "extract(ExtractionRequest{kind=slice}, RuntimeOptions)",
            request_contract: OperationContract {
                rust_shape: "ExtractionRequest + RuntimeOptions",
                schema_refs: EXTRACTION_RUNTIME_SCHEMA_REFS,
            },
            result_contract: OperationContract {
                rust_shape: "ExtractionResult",
                schema_refs: EXTRACTION_RESULT_SCHEMA_REFS,
            },
            description: "Extract final values between literal or regex boundaries in raw source text.",
        },
        cli: Some(OperationCliSpec {
            command_path: &["slice"],
            invocation: "htmlcut slice [OPTIONS] --from <FROM> --to <TO> [INPUT]",
            default_match: Some(CliSelectionMode::First),
            selection_modes: common_selection_modes,
            default_value: Some(ValueType::Text),
            value_modes: extract_value_modes,
            default_output: Some(CliOutputMode::Text),
            default_output_overrides: extract_default_output_overrides,
            output_modes: extract_output_modes,
            build_parameters: slice_extract_parameters,
            build_constraints: extract_constraints,
            notes: &[
                "Literal boundaries are raw substring matching, not tag-aware; `<a` also matches `<article>`.",
                "The selected fragment excludes both matched boundaries by default; --include-start and --include-end control that selected fragment precisely.",
                "Structured extraction only supports --output json or --output none.",
                "--output html is only valid with --value inner-html or --value outer-html.",
                "When --rewrite-urls is requested but no effective base URL can be resolved, relative URLs stay unchanged and a warning is emitted.",
                "Use --emit-request-file to capture the canonical extraction definition while you iterate on inline flags.",
            ],
            examples: &[
                "htmlcut slice ./page.html --from '<article>' --to '</article>' --match single",
                "htmlcut slice ./page.html --from 'START::' --to '::END' --pattern regex --value outer-html",
                "htmlcut slice ./page.html --from '<a' --to '</a>' --include-start --include-end --value attribute --attribute href --rewrite-urls",
                "htmlcut slice ./page.html --from '<article>' --to '</article>' --emit-request-file ./article-slice.json",
                "htmlcut slice --request-file ./article-slice.json --output-file ./article.txt",
            ],
            help_overview: &[
                "Use --pattern literal for plain substring boundaries or --pattern regex for regex boundaries. Boundary matches are consumed exactly as matched.",
            ],
        }),
    },
];
