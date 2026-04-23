use super::*;
use htmlcut_core::{CliHelpDocument, OperationId};

use crate::help::{
    build_operation_long_about_from_sources_for_tests,
    operation_examples_after_help_from_document_for_tests, resolve_cached_help_text_for_tests,
};
use crate::lookup::operation_contract;

#[test]
fn help_rendering_helpers_surface_contract_errors_without_panicking() {
    let error = internal_error("CLI_CONTRACT_MISSING", "missing help contract");

    assert_eq!(
        resolve_cached_help_text_for_tests(Ok("ready".to_owned())),
        "ready"
    );
    let fallback = resolve_cached_help_text_for_tests(Err(internal_error(
        "CLI_CONTRACT_MISSING",
        "missing cached help",
    )));
    assert!(fallback.contains("Internal HTMLCut CLI contract error."));
    assert!(fallback.contains("missing cached help"));

    assert_eq!(
        build_operation_long_about_from_sources_for_tests(
            Err(internal_error("CLI_CONTRACT_MISSING", "missing contract")),
            Ok(CliHelpDocument {
                sections: Vec::new(),
                examples: Vec::new(),
            }),
        )
        .expect_err("missing contract should fail")
        .message,
        "missing contract"
    );
    assert_eq!(
        build_operation_long_about_from_sources_for_tests(
            Ok(operation_contract(OperationId::SelectExtract).expect("select contract")),
            Err(error),
        )
        .expect_err("missing help doc should fail")
        .message,
        "missing help contract"
    );
    assert_eq!(
        operation_examples_after_help_from_document_for_tests(Err(internal_error(
            "CLI_CONTRACT_MISSING",
            "missing examples",
        )))
        .expect_err("missing examples should fail")
        .message,
        "missing examples"
    );
}
