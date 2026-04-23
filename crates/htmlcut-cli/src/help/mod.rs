mod cache;
mod render;

#[cfg(test)]
use crate::error::CliError;
#[cfg(test)]
use htmlcut_core::{CliHelpSection, OperationCliContract};

pub(crate) const ROOT_HELP_TEMPLATE: &str = "\
{before-help}{usage-heading} {usage}

{all-args}{about-section}{after-help}\
";

pub(crate) fn catalog_about() -> &'static str {
    cache::catalog_about()
}

pub(crate) fn schema_about() -> &'static str {
    cache::schema_about()
}

pub(crate) fn inspect_about() -> &'static str {
    cache::inspect_about()
}

pub(crate) fn select_about() -> &'static str {
    cache::select_about()
}

pub(crate) fn slice_about() -> &'static str {
    cache::slice_about()
}

pub(crate) fn inspect_source_about() -> &'static str {
    cache::inspect_source_about()
}

pub(crate) fn inspect_select_about() -> &'static str {
    cache::inspect_select_about()
}

pub(crate) fn inspect_slice_about() -> &'static str {
    cache::inspect_slice_about()
}

pub(crate) fn root_long_about() -> &'static str {
    cache::root_long_about()
}

pub(crate) fn root_before_help() -> &'static str {
    cache::root_before_help()
}

pub(crate) fn root_after_help() -> &'static str {
    cache::root_after_help()
}

pub(crate) fn catalog_long_about() -> &'static str {
    cache::catalog_long_about()
}

pub(crate) fn catalog_after_help() -> &'static str {
    cache::catalog_after_help()
}

pub(crate) fn schema_long_about() -> &'static str {
    cache::schema_long_about()
}

pub(crate) fn schema_after_help() -> &'static str {
    cache::schema_after_help()
}

pub(crate) fn inspect_long_about() -> &'static str {
    cache::inspect_long_about()
}

pub(crate) fn select_long_about() -> &'static str {
    cache::select_long_about()
}

pub(crate) fn slice_long_about() -> &'static str {
    cache::slice_long_about()
}

pub(crate) fn inspect_source_long_about() -> &'static str {
    cache::inspect_source_long_about()
}

pub(crate) fn inspect_select_long_about() -> &'static str {
    cache::inspect_select_long_about()
}

pub(crate) fn inspect_slice_long_about() -> &'static str {
    cache::inspect_slice_long_about()
}

pub(crate) fn select_after_help() -> &'static str {
    cache::select_after_help()
}

pub(crate) fn slice_after_help() -> &'static str {
    cache::slice_after_help()
}

pub(crate) fn inspect_source_after_help() -> &'static str {
    cache::inspect_source_after_help()
}

pub(crate) fn inspect_select_after_help() -> &'static str {
    cache::inspect_select_after_help()
}

pub(crate) fn inspect_slice_after_help() -> &'static str {
    cache::inspect_slice_after_help()
}

#[cfg(test)]
pub(crate) fn render_help_section_for_tests(section: &CliHelpSection) -> String {
    render::render_help_section(section)
}

#[cfg(test)]
pub(crate) fn render_contract_mode_summary_for_tests(contract: &OperationCliContract) -> String {
    render::render_contract_mode_summary(contract)
}

#[cfg(test)]
pub(crate) fn build_operation_long_about_from_parts_for_tests(
    sections: Vec<CliHelpSection>,
    contract: &OperationCliContract,
) -> String {
    render::build_operation_long_about_from_parts(sections, contract)
}

#[cfg(test)]
pub(crate) fn resolve_cached_help_text_for_tests(result: Result<String, CliError>) -> String {
    cache::resolve_cached_help_text_for_tests(result)
}

#[cfg(test)]
pub(crate) fn build_operation_long_about_from_sources_for_tests(
    contract: Result<&'static OperationCliContract, CliError>,
    document: Result<htmlcut_core::CliHelpDocument, CliError>,
) -> Result<String, CliError> {
    render::build_operation_long_about_from_sources_for_tests(contract, document)
}

#[cfg(test)]
pub(crate) fn operation_examples_after_help_from_document_for_tests(
    document: Result<htmlcut_core::CliHelpDocument, CliError>,
) -> Result<String, CliError> {
    render::operation_examples_after_help_from_document_for_tests(document)
}
