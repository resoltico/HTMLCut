#[cfg(test)]
use crate::catalog::operation_descriptor;
#[cfg(test)]
use crate::catalog::{OperationDescriptor, OperationId, operation_catalog};

#[cfg(test)]
use super::documents::{build_cli_operation_help_document, build_cli_root_help_document};
#[cfg(test)]
use super::model::CliAuxCommandId;
#[cfg(test)]
use super::model::{CliAuxCommandDescriptor, cli_aux_command_catalog};

#[cfg(test)]
pub(crate) fn cli_help_catalog_validation_errors() -> Vec<String> {
    cli_help_catalog_validation_errors_with(
        cli_aux_command_catalog(),
        operation_catalog(),
        |operation_id| build_cli_operation_help_document(operation_id).is_some(),
        build_cli_root_help_document().examples.is_empty(),
    )
}

#[cfg(test)]
pub(crate) fn assert_cli_help_catalog_errors_for_tests(errors: Vec<String>) {
    assert!(
        errors.is_empty(),
        "cli_help_catalog drifted:\n- {}",
        errors.join("\n- ")
    );
}

#[cfg(test)]
fn cli_help_catalog_validation_errors_with(
    aux_descriptors: &[CliAuxCommandDescriptor],
    operation_descriptors: &[OperationDescriptor],
    has_help: impl Fn(OperationId) -> bool,
    root_examples_empty: bool,
) -> Vec<String> {
    let mut errors = Vec::new();

    if aux_descriptors.is_empty() {
        errors.push("cli_aux_command_catalog() is empty".to_owned());
    }

    for descriptor in aux_descriptors {
        if descriptor.command_path.is_empty() {
            errors.push(format!("{:?} has an empty command path", descriptor.id));
        }
        if descriptor.about.trim().is_empty() {
            errors.push(format!("{:?} has an empty about string", descriptor.id));
        }
        if descriptor.command_path != descriptor.id.command_path() {
            errors.push(format!(
                "{:?} command path drifted from CliAuxCommandId::command_path()",
                descriptor.id
            ));
        }
    }

    for descriptor in operation_descriptors {
        match (descriptor.cli_surface.is_some(), has_help(descriptor.id)) {
            (true, false) => errors.push(format!(
                "{} is CLI-visible in OPERATION_CATALOG but missing CLI help documentation",
                descriptor.id
            )),
            (false, true) => errors.push(format!(
                "{} is core-only in OPERATION_CATALOG but has CLI help documentation",
                descriptor.id
            )),
            (true, true) | (false, false) => {}
        }
    }

    if root_examples_empty {
        errors.push("root help examples are empty".to_owned());
    }

    errors
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aux_command_ids_keep_stable_command_paths() {
        assert_eq!(CliAuxCommandId::Catalog.command_path(), &["catalog"]);
        assert_eq!(CliAuxCommandId::Schema.command_path(), &["schema"]);
        assert_eq!(CliAuxCommandId::Inspect.command_path(), &["inspect"]);
    }

    #[test]
    fn document_parse_remains_core_only_in_operation_help() {
        assert!(build_cli_operation_help_document(OperationId::DocumentParse).is_none());
    }

    #[test]
    fn validation_helper_reports_missing_help_and_empty_catalog_fields() {
        let malformed_aux = [CliAuxCommandDescriptor {
            id: CliAuxCommandId::Catalog,
            command_path: &[],
            about: "   ",
        }];
        let select_extract = operation_descriptor(OperationId::SelectExtract)
            .copied()
            .expect("select.extract descriptor");

        let errors = cli_help_catalog_validation_errors_with(
            &malformed_aux,
            &[select_extract],
            |_| false,
            true,
        );

        assert!(
            errors
                .iter()
                .any(|error| error.contains("empty command path"))
        );
        assert!(
            errors
                .iter()
                .any(|error| error.contains("empty about string"))
        );
        assert!(
            errors
                .iter()
                .any(|error| error.contains("missing CLI help documentation"))
        );
        assert!(
            errors
                .iter()
                .any(|error| error.contains("root help examples are empty"))
        );
    }

    #[test]
    fn validation_helper_reports_empty_auxiliary_catalogs() {
        let errors = cli_help_catalog_validation_errors_with(&[], &[], |_| false, false);

        assert!(
            errors
                .iter()
                .any(|error| error.contains("cli_aux_command_catalog() is empty"))
        );
    }

    #[test]
    fn validation_helper_reports_help_for_core_only_operations() {
        let document_parse = operation_descriptor(OperationId::DocumentParse)
            .copied()
            .expect("document.parse descriptor");

        let errors = cli_help_catalog_validation_errors_with(
            cli_aux_command_catalog(),
            &[document_parse],
            |_| true,
            false,
        );

        assert!(
            errors
                .iter()
                .any(|error| error.contains("core-only in OPERATION_CATALOG"))
        );
    }
}
