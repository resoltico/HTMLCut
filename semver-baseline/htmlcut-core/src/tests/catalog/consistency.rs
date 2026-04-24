use super::*;

#[test]
fn catalog_lint_rejects_missing_cli_entries_core_only_leaks_duplicates_and_display_drift() {
    let descriptors = vec![
        crate::catalog::OperationDescriptor {
            id: OperationId::SelectExtract,
            cli_surface: Some("select"),
            core_surface: "extract",
            request_contract: crate::catalog::OperationContract {
                rust_shape: "request",
                schema_refs: &[],
            },
            result_contract: crate::catalog::OperationContract {
                rust_shape: "result",
                schema_refs: &[],
            },
            description: "select",
        },
        crate::catalog::OperationDescriptor {
            id: OperationId::DocumentParse,
            cli_surface: None,
            core_surface: "parse_document",
            request_contract: crate::catalog::OperationContract {
                rust_shape: "request",
                schema_refs: &[],
            },
            result_contract: crate::catalog::OperationContract {
                rust_shape: "result",
                schema_refs: &[],
            },
            description: "parse",
        },
        crate::catalog::OperationDescriptor {
            id: OperationId::SliceExtract,
            cli_surface: Some("slice"),
            core_surface: "extract",
            request_contract: crate::catalog::OperationContract {
                rust_shape: "request",
                schema_refs: &[],
            },
            result_contract: crate::catalog::OperationContract {
                rust_shape: "result",
                schema_refs: &[],
            },
            description: "slice",
        },
    ];
    let mut slice_contract = crate::cli_contract::cli_operation_contract(OperationId::SliceExtract)
        .expect("slice contract")
        .clone();
    slice_contract.command_path = &["inspect", "slice"];
    let duplicate = slice_contract.clone();
    let mut core_only_leak =
        crate::cli_contract::cli_operation_contract(OperationId::SelectExtract)
            .expect("select contract")
            .clone();
    core_only_leak.operation_id = OperationId::DocumentParse;
    core_only_leak.command_path = &["document", "parse"];

    let errors = crate::cli_contract::cli_operation_catalog_validation_errors_for(
        &descriptors,
        &[slice_contract, duplicate, core_only_leak],
    );

    for expected in [
        "select.extract is marked CLI-visible in OPERATION_CATALOG but missing from cli_operation_catalog()",
        "document.parse appears in cli_operation_catalog() but is marked core-only in OPERATION_CATALOG",
        "slice.extract appears more than once in cli_operation_catalog()",
        "slice.extract display command drifted",
    ] {
        assert!(
            errors.iter().any(|error| error.contains(expected)),
            "missing catalog-lint error containing {expected:?}: {errors:#?}"
        );
    }
}
#[test]
fn catalog_lint_accepts_consistent_cli_and_core_catalogs() {
    let descriptors = vec![
        crate::catalog::OperationDescriptor {
            id: OperationId::SelectExtract,
            cli_surface: Some("select"),
            core_surface: "extract",
            request_contract: crate::catalog::OperationContract {
                rust_shape: "request",
                schema_refs: &[],
            },
            result_contract: crate::catalog::OperationContract {
                rust_shape: "result",
                schema_refs: &[],
            },
            description: "select",
        },
        crate::catalog::OperationDescriptor {
            id: OperationId::DocumentParse,
            cli_surface: None,
            core_surface: "parse_document",
            request_contract: crate::catalog::OperationContract {
                rust_shape: "request",
                schema_refs: &[],
            },
            result_contract: crate::catalog::OperationContract {
                rust_shape: "result",
                schema_refs: &[],
            },
            description: "parse",
        },
    ];
    let mut select_contract =
        crate::cli_contract::cli_operation_contract(OperationId::SelectExtract)
            .expect("select contract")
            .clone();
    select_contract.command_path = &["select"];

    let errors = crate::cli_contract::cli_operation_catalog_validation_errors_for(
        &descriptors,
        &[select_contract],
    );
    assert!(
        errors.is_empty(),
        "unexpected catalog-lint errors: {errors:#?}"
    );
}
