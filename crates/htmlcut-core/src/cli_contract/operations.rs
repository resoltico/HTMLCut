use crate::catalog::OperationId;
use crate::contracts::ValueType;

use super::parameters::{
    common_input_forms, common_selection_modes, condition, conditional_default,
    constraints_with_parameter_rules, extract_output_modes, extract_value_modes,
    inspect_output_modes, inspect_select_parameters, inspect_slice_parameters,
    inspect_source_parameters, requires_parameter, restricts_parameter_values,
    select_extract_parameters, slice_extract_parameters,
};
use super::{CliOutputMode, CliParameterId, CliSelectionMode, CliValue, OperationCliContract};

pub(super) fn build_cli_operation_catalog() -> Vec<OperationCliContract> {
    vec![
        build_source_inspect_contract(),
        build_select_preview_contract(),
        build_slice_preview_contract(),
        build_select_extract_contract(),
        build_slice_extract_contract(),
    ]
}

fn build_source_inspect_contract() -> OperationCliContract {
    let parameters = inspect_source_parameters();
    let output_modes = inspect_output_modes();
    OperationCliContract {
        operation_id: OperationId::SourceInspect,
        command_path: &["inspect", "source"],
        invocation: "htmlcut inspect source [OPTIONS] [INPUT]",
        inputs: common_input_forms(),
        default_match: None,
        selection_modes: Vec::new(),
        default_value: None,
        value_modes: Vec::new(),
        default_output: Some(CliOutputMode::Json),
        default_output_overrides: Vec::new(),
        output_modes: output_modes.clone(),
        constraints: constraints_with_parameter_rules(&parameters, Vec::new()),
        notes: vec![
            "Use this command to inspect document shape, headings, links, classes, and effective base-URL behavior before choosing selectors or slice boundaries.",
            "--include-source-text stores the full source in JSON output and prints a bounded source preview in text mode.",
            "--sample-limit bounds the sampled headings, links, tags, and classes in the summary.",
        ],
        examples: vec![
            "htmlcut inspect source ./page.html",
            "htmlcut inspect source ./page.html --output text --include-source-text --preview-chars 200",
        ],
        parameters,
    }
}

fn build_select_preview_contract() -> OperationCliContract {
    let parameters = inspect_select_parameters();
    let selection_modes = common_selection_modes();
    let output_modes = inspect_output_modes();
    OperationCliContract {
        operation_id: OperationId::SelectPreview,
        command_path: &["inspect", "select"],
        invocation: "htmlcut inspect select [OPTIONS] --css <CSS> [INPUT]",
        inputs: common_input_forms(),
        default_match: Some(CliSelectionMode::First),
        selection_modes: selection_modes.clone(),
        default_value: Some(ValueType::Structured),
        value_modes: vec![ValueType::Structured],
        default_output: Some(CliOutputMode::Json),
        default_output_overrides: Vec::new(),
        output_modes: output_modes.clone(),
        constraints: constraints_with_parameter_rules(&parameters, Vec::new()),
        notes: vec![
            "inspect select always previews matches in structured form; it is a preview workflow, not a final extraction surface.",
            "When --rewrite-urls is requested but no effective base URL can be resolved, relative URLs stay unchanged and a warning is emitted.",
            "Use --emit-request-file to capture the canonical preview definition while you iterate on inline flags.",
        ],
        examples: vec![
            "htmlcut inspect select ./page.html --css article --match single",
            "htmlcut inspect select ./page.html --css '.card' --match all --output text",
            "htmlcut inspect select ./page.html --css article --emit-request-file ./article-preview.json",
            "htmlcut inspect select --request-file ./article-preview.json",
        ],
        parameters,
    }
}

fn build_slice_preview_contract() -> OperationCliContract {
    let parameters = inspect_slice_parameters();
    let selection_modes = common_selection_modes();
    let output_modes = inspect_output_modes();
    OperationCliContract {
        operation_id: OperationId::SlicePreview,
        command_path: &["inspect", "slice"],
        invocation: "htmlcut inspect slice [OPTIONS] --from <FROM> --to <TO> [INPUT]",
        inputs: common_input_forms(),
        default_match: Some(CliSelectionMode::First),
        selection_modes: selection_modes.clone(),
        default_value: Some(ValueType::Structured),
        value_modes: vec![ValueType::Structured],
        default_output: Some(CliOutputMode::Json),
        default_output_overrides: Vec::new(),
        output_modes: output_modes.clone(),
        constraints: constraints_with_parameter_rules(&parameters, Vec::new()),
        notes: vec![
            "Literal boundaries are raw substring matching, not tag-aware; `<a` also matches `<article>`.",
            "Previews exclude both matched boundaries by default unless --include-start and/or --include-end are supplied.",
            "Text output shows fragment context when it adds signal so boundary-consumption mistakes are easier to spot.",
            "Use --emit-request-file to capture the canonical preview definition while you iterate on inline flags.",
        ],
        examples: vec![
            "htmlcut inspect slice ./page.html --from '<article>' --to '</article>'",
            "htmlcut inspect slice ./page.html --from 'START::' --to '::END' --pattern regex --output text",
            "htmlcut inspect slice ./page.html --from '<article>' --to '</article>' --emit-request-file ./article-slice-preview.json",
            "htmlcut inspect slice --request-file ./article-slice-preview.json",
        ],
        parameters,
    }
}

fn build_select_extract_contract() -> OperationCliContract {
    let parameters = select_extract_parameters();
    let selection_modes = common_selection_modes();
    let value_modes = extract_value_modes();
    let output_modes = extract_output_modes();
    OperationCliContract {
        operation_id: OperationId::SelectExtract,
        command_path: &["select"],
        invocation: "htmlcut select [OPTIONS] --css <CSS> [INPUT]",
        inputs: common_input_forms(),
        default_match: Some(CliSelectionMode::First),
        selection_modes: selection_modes.clone(),
        default_value: Some(ValueType::Text),
        value_modes: value_modes.clone(),
        default_output: Some(CliOutputMode::Text),
        default_output_overrides: vec![conditional_default(
            CliValue::OutputMode(CliOutputMode::Json),
            condition(
                CliParameterId::Value,
                vec![CliValue::ValueType(ValueType::Structured)],
            ),
        )],
        output_modes: output_modes.clone(),
        constraints: constraints_with_parameter_rules(
            &parameters,
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
        ),
        notes: vec![
            "Structured extraction only supports --output json or --output none.",
            "--output html is only valid with --value inner-html or --value outer-html.",
            "When --rewrite-urls is requested but no effective base URL can be resolved, relative URLs stay unchanged and a warning is emitted.",
            "Use --emit-request-file to capture the canonical extraction definition while you iterate on inline flags.",
        ],
        examples: vec![
            "htmlcut select ./page.html --css article --match single",
            "htmlcut select ./page.html --css '.card' --match all --value outer-html",
            "htmlcut select ./page.html --css 'article a.more' --value attribute --attribute href --rewrite-urls",
            "htmlcut select ./page.html --css 'article a.more' --value attribute --attribute href --emit-request-file ./article-links.json",
            "htmlcut select --request-file ./article-links.json --output-file ./links.json",
        ],
        parameters,
    }
}

fn build_slice_extract_contract() -> OperationCliContract {
    let parameters = slice_extract_parameters();
    let selection_modes = common_selection_modes();
    let value_modes = extract_value_modes();
    let output_modes = extract_output_modes();
    OperationCliContract {
        operation_id: OperationId::SliceExtract,
        command_path: &["slice"],
        invocation: "htmlcut slice [OPTIONS] --from <FROM> --to <TO> [INPUT]",
        inputs: common_input_forms(),
        default_match: Some(CliSelectionMode::First),
        selection_modes: selection_modes.clone(),
        default_value: Some(ValueType::Text),
        value_modes: value_modes.clone(),
        default_output: Some(CliOutputMode::Text),
        default_output_overrides: vec![conditional_default(
            CliValue::OutputMode(CliOutputMode::Json),
            condition(
                CliParameterId::Value,
                vec![CliValue::ValueType(ValueType::Structured)],
            ),
        )],
        output_modes: output_modes.clone(),
        constraints: constraints_with_parameter_rules(
            &parameters,
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
        ),
        notes: vec![
            "Literal boundaries are raw substring matching, not tag-aware; `<a` also matches `<article>`.",
            "The selected fragment excludes both matched boundaries by default; --include-start and --include-end control that selected fragment precisely.",
            "For --value inner-html, HTMLCut returns the selected fragment as HTML. For --value outer-html, HTMLCut returns the full outer matched range including both boundaries.",
            "When extracting --value attribute from sliced HTML, use --include-start when the opening tag lives in the start boundary.",
            "Structured extraction only supports --output json or --output none.",
            "Use --emit-request-file to capture the canonical extraction definition while you iterate on inline flags.",
        ],
        examples: vec![
            "htmlcut slice ./page.html --from '<article>' --to '</article>'",
            "htmlcut slice ./page.html --from 'START::' --to '::END' --pattern regex --match all --output json",
            "htmlcut slice ./page.html --from '<a ' --to '</a>' --include-start --include-end --value attribute --attribute href",
            "htmlcut slice ./page.html --from '<article>' --to '</article>' --emit-request-file ./article-slice.json",
            "htmlcut slice --request-file ./article-slice.json --output-file ./fragment.html",
        ],
        parameters,
    }
}
