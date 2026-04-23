use std::sync::LazyLock;

use htmlcut_core::{CliAuxCommandId, OperationId};

use crate::error::CliError;

use super::render::{
    build_operation_long_about, operation_examples_after_help, render_help_examples,
    render_help_sections,
};

static ROOT_LONG_ABOUT: LazyLock<String> =
    LazyLock::new(|| render_help_sections(&htmlcut_core::cli_root_help_document().sections));
static ROOT_AFTER_HELP: LazyLock<String> =
    LazyLock::new(|| render_help_examples(&htmlcut_core::cli_root_help_document()));

static CATALOG_LONG_ABOUT: LazyLock<String> = LazyLock::new(|| {
    render_help_sections(
        &htmlcut_core::cli_aux_command_help_document(CliAuxCommandId::Catalog).sections,
    )
});
static CATALOG_AFTER_HELP: LazyLock<String> = LazyLock::new(|| {
    render_help_examples(&htmlcut_core::cli_aux_command_help_document(
        CliAuxCommandId::Catalog,
    ))
});

static SCHEMA_LONG_ABOUT: LazyLock<String> = LazyLock::new(|| {
    render_help_sections(
        &htmlcut_core::cli_aux_command_help_document(CliAuxCommandId::Schema).sections,
    )
});
static SCHEMA_AFTER_HELP: LazyLock<String> = LazyLock::new(|| {
    render_help_examples(&htmlcut_core::cli_aux_command_help_document(
        CliAuxCommandId::Schema,
    ))
});

static INSPECT_LONG_ABOUT: LazyLock<String> = LazyLock::new(|| {
    render_help_sections(
        &htmlcut_core::cli_aux_command_help_document(CliAuxCommandId::Inspect).sections,
    )
});

static SELECT_LONG_ABOUT: LazyLock<String> = LazyLock::new(|| {
    resolve_cached_help_text(build_operation_long_about(OperationId::SelectExtract))
});
static SLICE_LONG_ABOUT: LazyLock<String> = LazyLock::new(|| {
    resolve_cached_help_text(build_operation_long_about(OperationId::SliceExtract))
});
static INSPECT_SOURCE_LONG_ABOUT: LazyLock<String> = LazyLock::new(|| {
    resolve_cached_help_text(build_operation_long_about(OperationId::SourceInspect))
});
static INSPECT_SELECT_LONG_ABOUT: LazyLock<String> = LazyLock::new(|| {
    resolve_cached_help_text(build_operation_long_about(OperationId::SelectPreview))
});
static INSPECT_SLICE_LONG_ABOUT: LazyLock<String> = LazyLock::new(|| {
    resolve_cached_help_text(build_operation_long_about(OperationId::SlicePreview))
});

static SELECT_AFTER_HELP: LazyLock<String> = LazyLock::new(|| {
    resolve_cached_help_text(operation_examples_after_help(OperationId::SelectExtract))
});
static SLICE_AFTER_HELP: LazyLock<String> = LazyLock::new(|| {
    resolve_cached_help_text(operation_examples_after_help(OperationId::SliceExtract))
});
static INSPECT_SOURCE_AFTER_HELP: LazyLock<String> = LazyLock::new(|| {
    resolve_cached_help_text(operation_examples_after_help(OperationId::SourceInspect))
});
static INSPECT_SELECT_AFTER_HELP: LazyLock<String> = LazyLock::new(|| {
    resolve_cached_help_text(operation_examples_after_help(OperationId::SelectPreview))
});
static INSPECT_SLICE_AFTER_HELP: LazyLock<String> = LazyLock::new(|| {
    resolve_cached_help_text(operation_examples_after_help(OperationId::SlicePreview))
});

pub(super) fn catalog_about() -> &'static str {
    htmlcut_core::cli_aux_command_descriptor(CliAuxCommandId::Catalog).about
}

pub(super) fn schema_about() -> &'static str {
    htmlcut_core::cli_aux_command_descriptor(CliAuxCommandId::Schema).about
}

pub(super) fn inspect_about() -> &'static str {
    htmlcut_core::cli_aux_command_descriptor(CliAuxCommandId::Inspect).about
}

pub(super) fn select_about() -> &'static str {
    htmlcut_core::operation_descriptor(OperationId::SelectExtract).description
}

pub(super) fn slice_about() -> &'static str {
    htmlcut_core::operation_descriptor(OperationId::SliceExtract).description
}

pub(super) fn inspect_source_about() -> &'static str {
    htmlcut_core::operation_descriptor(OperationId::SourceInspect).description
}

pub(super) fn inspect_select_about() -> &'static str {
    htmlcut_core::operation_descriptor(OperationId::SelectPreview).description
}

pub(super) fn inspect_slice_about() -> &'static str {
    htmlcut_core::operation_descriptor(OperationId::SlicePreview).description
}

pub(super) fn root_long_about() -> &'static str {
    ROOT_LONG_ABOUT.as_str()
}

pub(super) fn root_after_help() -> &'static str {
    ROOT_AFTER_HELP.as_str()
}

pub(super) fn catalog_long_about() -> &'static str {
    CATALOG_LONG_ABOUT.as_str()
}

pub(super) fn catalog_after_help() -> &'static str {
    CATALOG_AFTER_HELP.as_str()
}

pub(super) fn schema_long_about() -> &'static str {
    SCHEMA_LONG_ABOUT.as_str()
}

pub(super) fn schema_after_help() -> &'static str {
    SCHEMA_AFTER_HELP.as_str()
}

pub(super) fn inspect_long_about() -> &'static str {
    INSPECT_LONG_ABOUT.as_str()
}

pub(super) fn select_long_about() -> &'static str {
    SELECT_LONG_ABOUT.as_str()
}

pub(super) fn slice_long_about() -> &'static str {
    SLICE_LONG_ABOUT.as_str()
}

pub(super) fn inspect_source_long_about() -> &'static str {
    INSPECT_SOURCE_LONG_ABOUT.as_str()
}

pub(super) fn inspect_select_long_about() -> &'static str {
    INSPECT_SELECT_LONG_ABOUT.as_str()
}

pub(super) fn inspect_slice_long_about() -> &'static str {
    INSPECT_SLICE_LONG_ABOUT.as_str()
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

#[cfg(test)]
pub(crate) fn resolve_cached_help_text_for_tests(result: Result<String, CliError>) -> String {
    resolve_cached_help_text(result)
}
