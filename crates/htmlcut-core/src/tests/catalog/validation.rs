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
fn contract_lint_diagnostic_codes_reject_unknown_strings_with_a_stable_error() {
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
fn contract_lint_cli_choice_and_non_empty_contract_values_reject_drift_and_blank_inputs() {
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
    let whitespace = AttributeName::new("href ").expect_err("whitespace attribute");
    assert_eq!(
        whitespace,
        crate::contracts::ContractValueError::ContainsWhitespace {
            field: "attribute name"
        }
    );
    let blank_selector = SelectorQuery::new("   ").expect_err("blank selector");
    assert_eq!(
        blank_selector,
        crate::contracts::ContractValueError::Empty { field: "selector" }
    );
    assert_eq!(
        crate::DEFAULT_FETCH_PREFLIGHT_MODE,
        FetchPreflightMode::HeadFirst
    );
}

#[test]
fn contract_lint_catalog_and_schema_contract_strings_stay_canonical() {
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
fn schema_descriptor_constructor_preserves_fields() {
    fn synthetic_schema() -> Result<serde_json::Value, crate::SchemaExportError> {
        Ok(serde_json::json!({ "type": "string" }))
    }

    let descriptor = crate::schema::catalog_schema_descriptor_for_tests(
        crate::SchemaRef::new("htmlcut.synthetic_core_schema", 1),
        "htmlcut-core",
        "SyntheticCoreSchema",
        synthetic_schema,
    );
    assert_eq!(
        descriptor.schema_ref,
        crate::SchemaRef::new("htmlcut.synthetic_core_schema", 1)
    );
    assert_eq!(descriptor.owner_surface, "htmlcut-core");
    assert_eq!(descriptor.rust_shape, "SyntheticCoreSchema");
    assert_eq!(descriptor.stability, crate::SchemaStability::Versioned);
    assert_eq!(
        (descriptor.json_schema)().expect("synthetic schema")["type"],
        "string"
    );
}
