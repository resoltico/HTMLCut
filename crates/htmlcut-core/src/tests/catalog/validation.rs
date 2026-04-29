use super::*;

fn panic_message(panic: Box<dyn std::any::Any + Send>) -> String {
    if let Some(message) = panic.downcast_ref::<String>() {
        return message.clone();
    }
    if let Some(message) = panic.downcast_ref::<&str>() {
        return (*message).to_owned();
    }
    "<non-string panic>".to_owned()
}

#[test]
fn diagnostic_codes_reject_unknown_strings_with_a_stable_error() {
    for code in DiagnosticCode::ALL {
        assert_eq!(
            DiagnosticCode::from_str(code.as_str()).expect("round-trip parse"),
            *code
        );
        assert_eq!(code.as_str(), *code);
    }

    let error = DiagnosticCode::from_str("NOT_A_REAL_CODE").expect_err("invalid diagnostic code");
    assert_eq!(error.to_string(), "unknown HTMLCut diagnostic code");
}
#[test]
fn cli_choice_and_non_empty_contract_values_reject_drift_and_blank_inputs() {
    assert_eq!(
        <ValueType as crate::CliChoice>::parse_cli_str("inner-html"),
        Some(ValueType::InnerHtml)
    );
    assert_eq!(
        <WhitespaceMode as crate::CliChoice>::parse_cli_str("normalize"),
        Some(WhitespaceMode::Normalize)
    );
    assert_eq!(
        <PatternMode as crate::CliChoice>::parse_cli_str("literal"),
        Some(PatternMode::Literal)
    );
    assert_eq!(
        <FetchPreflightMode as crate::CliChoice>::parse_cli_str("head-first"),
        Some(FetchPreflightMode::HeadFirst)
    );
    assert_eq!(
        <ValueType as crate::CliChoice>::parse_cli_str("not-real"),
        None
    );

    let attribute = AttributeName::new("href").expect("attribute");
    assert_eq!(attribute.as_ref(), "href");
    let blank = AttributeName::new("   ").expect_err("blank attribute");
    assert_eq!(
        blank,
        crate::contracts::ContractValueError::Empty {
            field: "attribute name"
        }
    );
}
#[test]
fn render_cli_value_covers_every_public_variant() {
    assert_eq!(
        crate::cli_contract::render_cli_value(crate::cli_contract::CliValue::SelectionMode(
            crate::cli_contract::CliSelectionMode::Nth
        )),
        "nth"
    );
    assert_eq!(
        crate::cli_contract::render_cli_value(crate::cli_contract::CliValue::ValueType(
            ValueType::InnerHtml
        )),
        "inner-html"
    );
    assert_eq!(
        crate::cli_contract::render_cli_value(crate::cli_contract::CliValue::OutputMode(
            crate::cli_contract::CliOutputMode::Html
        )),
        "html"
    );
    assert_eq!(
        crate::cli_contract::render_cli_value(crate::cli_contract::CliValue::WhitespaceMode(
            WhitespaceMode::Normalize
        )),
        "normalize"
    );
    assert_eq!(
        crate::cli_contract::render_cli_value(crate::cli_contract::CliValue::PatternMode(
            PatternMode::Regex
        )),
        "regex"
    );
    assert_eq!(
        crate::cli_contract::render_cli_value(crate::cli_contract::CliValue::FetchPreflightMode(
            FetchPreflightMode::GetOnly
        )),
        "get-only"
    );
    assert_eq!(
        crate::cli_contract::render_cli_value(crate::cli_contract::CliValue::Boolean(true)),
        "true"
    );
    assert_eq!(
        crate::cli_contract::render_cli_value(crate::cli_contract::CliValue::Usize(12)),
        "12"
    );
    assert_eq!(
        crate::cli_contract::render_cli_value(crate::cli_contract::CliValue::U64(64)),
        "64"
    );
}

#[test]
fn catalog_and_schema_contract_strings_stay_canonical() {
    assert!(!crate::catalog::operation_catalog().is_empty());
    assert!(
        crate::catalog::operation_catalog_contract_string_errors_for_tests().is_empty(),
        "unexpected operation catalog drift"
    );
    crate::catalog::assert_operation_catalog_contract_strings_for_tests(
        crate::catalog::operation_catalog(),
    );
    assert!(!crate::schema::schema_catalog().is_empty());
    assert!(
        crate::schema::schema_catalog_contract_string_errors_for_tests().is_empty(),
        "unexpected schema catalog drift"
    );
    crate::schema::assert_schema_catalog_contract_strings_for_tests(crate::schema::schema_catalog());
    crate::cli_contract::assert_cli_aux_command_catalog_for_tests(
        crate::cli_contract::cli_aux_command_catalog(),
    );
    crate::cli_contract::assert_cli_help_catalog_errors_for_tests(Vec::new());
    crate::cli_contract::assert_cli_operation_catalog_consistency_for_tests(
        crate::cli_contract::cli_operation_catalog(),
    );
}

#[test]
fn catalog_and_schema_contract_string_guards_reject_drift() {
    let canonical_operation = crate::catalog::operation_catalog()[0];
    let drifted_operation = crate::catalog::OperationDescriptor {
        core_surface: "",
        request_contract: crate::catalog::OperationContract {
            rust_shape: "",
            ..canonical_operation.request_contract
        },
        result_contract: crate::catalog::OperationContract {
            rust_shape: "",
            ..canonical_operation.result_contract
        },
        ..canonical_operation
    };
    let operation_errors =
        crate::catalog::operation_catalog_contract_string_errors_for_tests_with(&[
            drifted_operation,
        ]);
    assert!(
        operation_errors
            .iter()
            .any(|error| error.contains("empty core_surface"))
    );
    assert!(
        operation_errors
            .iter()
            .any(|error| error.contains("empty request rust_shape"))
    );
    assert!(
        operation_errors
            .iter()
            .any(|error| error.contains("empty result rust_shape"))
    );
    let duplicate_operation_errors =
        crate::catalog::operation_catalog_contract_string_errors_for_tests_with(&[
            canonical_operation,
            canonical_operation,
        ]);
    assert!(
        duplicate_operation_errors
            .iter()
            .any(|error| error.contains("appears more than once"))
    );

    let panic = std::panic::catch_unwind(|| {
        crate::catalog::assert_operation_catalog_contract_strings_for_tests(&[drifted_operation]);
    })
    .expect_err("catalog assertion should reject drift");
    let panic_text = panic_message(panic);
    assert!(panic_text.contains("operation catalog contract strings drifted"));

    let canonical_schema = crate::schema::schema_catalog()[0];
    let drifted_schema = crate::schema::SchemaDescriptor {
        schema_ref: crate::SchemaRef::new("htmlcut.unknown", 99),
        ..canonical_schema
    };
    let schema_errors =
        crate::schema::schema_catalog_contract_string_errors_for_tests_with(&[drifted_schema]);
    assert!(
        schema_errors
            .iter()
            .any(|error| error.contains("is not part of the maintained schema inventory"))
    );
    let duplicate_schema_errors =
        crate::schema::schema_catalog_contract_string_errors_for_tests_with(&[
            canonical_schema,
            canonical_schema,
        ]);
    assert!(
        duplicate_schema_errors
            .iter()
            .any(|error| error.contains("appears more than once"))
    );

    let panic = std::panic::catch_unwind(|| {
        crate::schema::assert_schema_catalog_contract_strings_for_tests(&[drifted_schema]);
    })
    .expect_err("schema assertion should reject drift");
    let panic_text = panic_message(panic);
    assert!(panic_text.contains("schema catalog contract strings drifted"));

    assert_eq!(
        crate::schema::expected_schema_rust_shape_for_tests(crate::SchemaRef::new(
            "htmlcut.unknown",
            99,
        )),
        None
    );
    let unknown_schema_errors =
        crate::schema::schema_catalog_contract_string_errors_for_tests_with(&[
            crate::schema::SchemaDescriptor {
                schema_ref: crate::SchemaRef::new("htmlcut.unknown", 99),
                owner_surface: "htmlcut-core",
                rust_shape: "Unknown",
                stability: crate::SchemaStability::Versioned,
                json_schema: || Ok(serde_json::json!({})),
            },
        ]);
    assert!(
        unknown_schema_errors
            .iter()
            .any(|error| error.contains("is not part of the maintained schema inventory"))
    );
}

#[test]
fn cli_contract_debug_guard_rejects_drift() {
    let mut drifted = crate::cli_contract::cli_operation_contract(OperationId::SelectExtract)
        .expect("select contract")
        .clone();
    drifted.command_path = &["bogus-select"];

    let errors = crate::cli_contract::cli_operation_catalog_validation_errors_for(
        crate::catalog::operation_catalog(),
        &[drifted.clone()],
    );
    assert!(
        errors
            .iter()
            .any(|error| error.contains("display command drifted"))
    );
    let missing_descriptor_errors =
        crate::cli_contract::cli_operation_catalog_validation_errors_for(&[], &[drifted.clone()]);
    assert!(
        missing_descriptor_errors
            .iter()
            .any(|error| error.contains("is missing from OPERATION_CATALOG"))
    );

    let panic = std::panic::catch_unwind(|| {
        crate::cli_contract::assert_cli_operation_catalog_consistency_for_tests(&[drifted]);
    })
    .expect_err("CLI contract assertion should reject drift");
    let panic_text = panic_message(panic);
    assert!(panic_text.contains("cli_operation_catalog drifted"));
}

#[test]
fn cli_aux_command_catalog_guards_reject_drift() {
    let empty_errors =
        crate::cli_contract::cli_aux_command_catalog_validation_errors_for_tests(&[]);
    assert!(
        empty_errors
            .iter()
            .any(|error| error.contains("cli_aux_command_catalog() is empty"))
    );

    let malformed = [crate::cli_contract::CliAuxCommandDescriptor {
        id: crate::cli_contract::CliAuxCommandId::Catalog,
        command_path: &[],
        about: "   ",
    }];

    let errors =
        crate::cli_contract::cli_aux_command_catalog_validation_errors_for_tests(&malformed);
    assert!(
        errors
            .iter()
            .any(|error| error.contains("empty about string"))
    );
    assert!(
        errors
            .iter()
            .any(|error| error.contains("empty command path"))
    );

    let duplicate = [
        crate::cli_contract::CliAuxCommandDescriptor {
            id: crate::cli_contract::CliAuxCommandId::Catalog,
            command_path: crate::cli_contract::CliAuxCommandId::Catalog.command_path(),
            about: "Catalog help",
        },
        crate::cli_contract::CliAuxCommandDescriptor {
            id: crate::cli_contract::CliAuxCommandId::Catalog,
            command_path: crate::cli_contract::CliAuxCommandId::Catalog.command_path(),
            about: "Catalog help duplicate",
        },
    ];
    let duplicate_errors =
        crate::cli_contract::cli_aux_command_catalog_validation_errors_for_tests(&duplicate);
    assert!(
        duplicate_errors
            .iter()
            .any(|error| error.contains("appears more than once"))
    );

    let panic = std::panic::catch_unwind(|| {
        crate::cli_contract::assert_cli_aux_command_catalog_for_tests(&malformed);
    })
    .expect_err("aux command assertion should reject drift");
    let panic_text = panic_message(panic);
    assert!(panic_text.contains("cli_aux_command_catalog drifted"));
}

#[test]
fn cli_help_catalog_assertion_surfaces_drift() {
    let panic = std::panic::catch_unwind(|| {
        crate::cli_contract::assert_cli_help_catalog_errors_for_tests(vec![
            "synthetic drift".to_owned(),
        ]);
    })
    .expect_err("help assertion should reject drift");
    let panic_text = panic_message(panic);
    assert!(panic_text.contains("cli_help_catalog drifted"));
}
