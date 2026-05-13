use std::sync::LazyLock;

use crate::contract::CliAuxCommandId;
use htmlcut_core::OperationId;

use crate::error::CliError;

use crate::metadata::identity_banner;

use super::render::{
    build_operation_long_about, operation_examples_after_help, render_examples_then_operator_guide,
};

static ROOT_BEFORE_HELP: LazyLock<String> = LazyLock::new(identity_banner);
static ROOT_AFTER_HELP: LazyLock<String> = LazyLock::new(|| {
    render_examples_then_operator_guide(&crate::contract::cli_root_help_document())
});

static CATALOG_AFTER_HELP: LazyLock<String> = LazyLock::new(|| {
    render_examples_then_operator_guide(&crate::contract::cli_aux_command_help_document(
        CliAuxCommandId::Catalog,
    ))
});

static SCHEMA_AFTER_HELP: LazyLock<String> = LazyLock::new(|| {
    render_examples_then_operator_guide(&crate::contract::cli_aux_command_help_document(
        CliAuxCommandId::Schema,
    ))
});

static SELECT_AFTER_HELP: LazyLock<String> =
    LazyLock::new(|| operation_after_help(OperationId::SelectExtract));
static SLICE_AFTER_HELP: LazyLock<String> =
    LazyLock::new(|| operation_after_help(OperationId::SliceExtract));
static INSPECT_SOURCE_AFTER_HELP: LazyLock<String> =
    LazyLock::new(|| operation_after_help(OperationId::SourceInspect));
static INSPECT_SELECT_AFTER_HELP: LazyLock<String> =
    LazyLock::new(|| operation_after_help(OperationId::SelectPreview));
static INSPECT_SLICE_AFTER_HELP: LazyLock<String> =
    LazyLock::new(|| operation_after_help(OperationId::SlicePreview));

pub(super) fn catalog_about() -> &'static str {
    crate::contract::cli_aux_command_descriptor(CliAuxCommandId::Catalog).about
}

pub(super) fn schema_about() -> &'static str {
    crate::contract::cli_aux_command_descriptor(CliAuxCommandId::Schema).about
}

pub(super) fn inspect_about() -> &'static str {
    crate::contract::cli_aux_command_descriptor(CliAuxCommandId::Inspect).about
}

pub(super) fn select_about() -> &'static str {
    htmlcut_core::operation_descriptor(OperationId::SelectExtract)
        .map(|descriptor| descriptor.description)
        .unwrap_or("Operation description unavailable.")
}

pub(super) fn slice_about() -> &'static str {
    htmlcut_core::operation_descriptor(OperationId::SliceExtract)
        .map(|descriptor| descriptor.description)
        .unwrap_or("Operation description unavailable.")
}

pub(super) fn inspect_source_about() -> &'static str {
    htmlcut_core::operation_descriptor(OperationId::SourceInspect)
        .map(|descriptor| descriptor.description)
        .unwrap_or("Operation description unavailable.")
}

pub(super) fn inspect_select_about() -> &'static str {
    htmlcut_core::operation_descriptor(OperationId::SelectPreview)
        .map(|descriptor| descriptor.description)
        .unwrap_or("Operation description unavailable.")
}

pub(super) fn inspect_slice_about() -> &'static str {
    htmlcut_core::operation_descriptor(OperationId::SlicePreview)
        .map(|descriptor| descriptor.description)
        .unwrap_or("Operation description unavailable.")
}

pub(super) fn root_before_help() -> &'static str {
    ROOT_BEFORE_HELP.as_str()
}

pub(super) fn root_after_help() -> &'static str {
    ROOT_AFTER_HELP.as_str()
}

pub(super) fn catalog_after_help() -> &'static str {
    CATALOG_AFTER_HELP.as_str()
}

pub(super) fn schema_after_help() -> &'static str {
    SCHEMA_AFTER_HELP.as_str()
}

pub(super) fn select_after_help() -> &'static str {
    SELECT_AFTER_HELP.as_str()
}

pub(super) fn slice_after_help() -> &'static str {
    SLICE_AFTER_HELP.as_str()
}

pub(super) fn inspect_source_after_help() -> &'static str {
    INSPECT_SOURCE_AFTER_HELP.as_str()
}

pub(super) fn inspect_select_after_help() -> &'static str {
    INSPECT_SELECT_AFTER_HELP.as_str()
}

pub(super) fn inspect_slice_after_help() -> &'static str {
    INSPECT_SLICE_AFTER_HELP.as_str()
}

fn resolve_cached_help_text(result: Result<String, CliError>) -> String {
    result
        .unwrap_or_else(|error| format!("Internal HTMLCut CLI contract error.\n{}", error.message))
}

fn operation_after_help(operation_id: OperationId) -> String {
    let examples = resolve_cached_help_text(operation_examples_after_help(operation_id));
    let guide = resolve_cached_help_text(build_operation_long_about(operation_id));
    compose_examples_and_operator_guide(examples, &guide)
}

fn compose_examples_and_operator_guide(examples: String, guide: &str) -> String {
    if guide.is_empty() {
        return examples;
    }

    let guide = format!("Operator Guide:\n\n{}", indent_operator_guide(guide));
    if examples.is_empty() {
        guide
    } else {
        format!("{examples}\n\n{guide}")
    }
}

fn indent_operator_guide(guide: &str) -> String {
    guide
        .lines()
        .map(|line| {
            if line.is_empty() {
                String::new()
            } else {
                format!("  {line}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
pub(crate) fn resolve_cached_help_text_for_tests(result: Result<String, CliError>) -> String {
    resolve_cached_help_text(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compose_examples_and_operator_guide_handles_empty_and_non_empty_guides() {
        assert_eq!(
            compose_examples_and_operator_guide("Examples:\n  htmlcut".to_owned(), ""),
            "Examples:\n  htmlcut"
        );
        assert_eq!(
            compose_examples_and_operator_guide(String::new(), "Workflow:\nLine"),
            "Operator Guide:\n\n  Workflow:\n  Line"
        );
        assert_eq!(
            compose_examples_and_operator_guide(
                "Examples:\n  htmlcut".to_owned(),
                "Workflow:\nLine"
            ),
            "Examples:\n  htmlcut\n\nOperator Guide:\n\n  Workflow:\n  Line"
        );
    }

    #[test]
    fn cached_help_accessors_return_operator_guide_surfaces() {
        assert!(root_after_help().contains("Operator Guide:"));
        assert!(catalog_after_help().contains("Operator Guide:"));
        assert!(schema_after_help().contains("Operator Guide:"));
        assert!(select_after_help().contains("Operator Guide:"));
        assert!(slice_after_help().contains("Operator Guide:"));
        assert!(inspect_source_after_help().contains("Operator Guide:"));
        assert!(inspect_select_after_help().contains("Operator Guide:"));
        assert!(inspect_slice_after_help().contains("Operator Guide:"));
    }
}
