mod documents;
mod model;
mod validation;

pub use documents::{
    cli_aux_command_help_document, cli_operation_help_document, cli_root_help_document,
};
pub use model::{
    CliAuxCommandDescriptor, CliAuxCommandId, CliHelpDocument, CliHelpSection, CliHelpSectionStyle,
    cli_aux_command_catalog, cli_aux_command_descriptor, cli_aux_command_display_command,
};

#[cfg(test)]
pub(crate) use validation::assert_cli_help_catalog_errors_for_tests;
#[cfg(test)]
pub(crate) use validation::cli_help_catalog_validation_errors;

pub(crate) fn ensure_cli_help_catalog_validated() {}

#[cfg(test)]
pub(crate) fn cli_aux_command_catalog_validation_errors_for_tests(
    descriptors: &[CliAuxCommandDescriptor],
) -> Vec<String> {
    model::cli_aux_command_catalog_validation_errors_for_tests(descriptors)
}

#[cfg(test)]
pub(crate) fn assert_cli_aux_command_catalog_for_tests(descriptors: &[CliAuxCommandDescriptor]) {
    model::assert_cli_aux_command_catalog_for_tests(descriptors);
}
