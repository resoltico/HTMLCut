mod documents;
mod model;
#[cfg(test)]
mod validation;

pub use documents::{
    cli_aux_command_help_document, cli_operation_help_document, cli_root_help_document,
};
pub use model::{
    CliAuxCommandDescriptor, CliAuxCommandId, CliHelpDocument, CliHelpSection, CliHelpSectionStyle,
    cli_aux_command_catalog, cli_aux_command_descriptor, cli_aux_command_display_command,
};

#[cfg(test)]
pub(crate) use validation::cli_help_catalog_validation_errors;
