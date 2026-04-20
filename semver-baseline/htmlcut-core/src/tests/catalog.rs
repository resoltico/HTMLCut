use super::*;

#[test]
fn defaults_cover_public_default_contracts() {
    assert_eq!(WhitespaceMode::default(), WhitespaceMode::Preserve);
    assert_eq!(SelectionSpec::default(), SelectionSpec::First);
    assert_eq!(ValueSpec::default().value_type(), ValueType::Text);
    let slice = slice_spec("<p>", "</p>");
    assert_eq!(slice.mode(), PatternMode::Literal);
    assert!(!slice.include_start);
    assert!(!slice.include_end);
    assert_eq!(slice.flags(), None);
    assert_eq!(
        ExtractionSpec::selector(selector_query("article")).strategy(),
        ExtractionStrategy::Selector
    );
    assert!(!NormalizationOptions::default().rewrite_urls);
    assert_eq!(
        OutputOptions::default().preview_chars,
        NonZeroUsize::new(DEFAULT_PREVIEW_CHARS).expect("preview chars")
    );
    assert_eq!(SourceRequest::stdin().kind(), SourceKind::Stdin);
    assert_eq!(
        selector_request("<article />").spec_version,
        CORE_SPEC_VERSION
    );
    assert_eq!(RuntimeOptions::default().max_bytes, DEFAULT_MAX_BYTES);
    assert_eq!(default_spec_version(), CORE_SPEC_VERSION);
    assert_eq!(default_preview_chars(), DEFAULT_PREVIEW_CHARS);
    assert_eq!(
        InspectionOptions::default().sample_limit,
        DEFAULT_INSPECTION_SAMPLE_LIMIT
    );
    assert_eq!(
        default_inspection_sample_limit(),
        DEFAULT_INSPECTION_SAMPLE_LIMIT
    );
    assert_eq!(default_regex_flags(), DEFAULT_REGEX_FLAGS);
    assert!(default_true());
    assert_eq!(default_max_bytes(), DEFAULT_MAX_BYTES);
    assert_eq!(default_fetch_timeout_ms(), DEFAULT_FETCH_TIMEOUT_MS);
    assert_eq!(format_byte_size(1024), "1 KB");
    assert_eq!(format_byte_size(1024 * 1024), "1 MB");
    assert_eq!(format_byte_size(1024 * 1024 * 1024), "1 GB");
    assert_eq!(format_byte_size(1024 * 1024 + 1), "1048577 bytes");
    assert_eq!(format_byte_size(1024 * 1024 * 1024 + 1), "1073741825 bytes");
    assert_eq!(format_byte_size(1536), "1536 bytes");
}

#[test]
fn operation_catalog_is_unique_and_complete() {
    let ids = operation_catalog()
        .iter()
        .map(|descriptor| descriptor.id)
        .collect::<BTreeSet<_>>();
    let id_strings = operation_catalog()
        .iter()
        .map(|descriptor| descriptor.id.as_str())
        .collect::<BTreeSet<_>>();

    assert_eq!(operation_catalog().len(), 6);
    assert_eq!(ids.len(), operation_catalog().len());
    assert_eq!(
        id_strings,
        BTreeSet::from([
            "document.parse",
            "source.inspect",
            "select.preview",
            "slice.preview",
            "select.extract",
            "slice.extract",
        ])
    );
    let document_parse = operation_catalog()
        .iter()
        .find(|descriptor| descriptor.id == OperationId::DocumentParse)
        .expect("document.parse should stay in the catalog");
    let select_extract = operation_catalog()
        .iter()
        .find(|descriptor| descriptor.id == OperationId::SelectExtract)
        .expect("select.extract should stay in the catalog");
    let select_preview = operation_catalog()
        .iter()
        .find(|descriptor| descriptor.id == OperationId::SelectPreview)
        .expect("select.preview should stay in the catalog");

    assert_eq!(document_parse.cli_surface, None);
    assert_eq!(select_extract.cli_surface, Some("select"));
    assert!(select_extract.core_surface.contains("extract"));
    assert_eq!(select_preview.cli_surface, Some("inspect select"));
    assert!(select_preview.core_surface.contains("preview_extraction"));
    assert_eq!(OperationId::DocumentParse.to_string(), "document.parse");
    assert_eq!(
        "slice.extract"
            .parse::<OperationId>()
            .expect("operation id"),
        OperationId::SliceExtract
    );
    assert!(matches!(
        "nope".parse::<OperationId>(),
        Err(OperationIdParseError)
    ));
    assert_eq!(
        OperationIdParseError.to_string(),
        "unknown HTMLCut operation ID"
    );
    assert_eq!(
        operation_descriptor(OperationId::SourceInspect).cli_surface,
        Some("inspect source")
    );

    let select_contract =
        crate::cli_operation_contract(OperationId::SelectExtract).expect("select cli contract");
    assert_eq!(select_contract.display_command(), "select");
    assert_eq!(select_contract.report_command(), "select");
    assert_eq!(
        crate::find_cli_operation_by_command_path(&["inspect", "slice"])
            .expect("inspect slice contract")
            .operation_id,
        OperationId::SlicePreview
    );
}

#[test]
fn diagnostic_codes_reject_unknown_strings_with_a_stable_error() {
    for code in DiagnosticCode::ALL {
        assert_eq!(
            DiagnosticCode::from_str(code.as_str()).expect("round-trip parse"),
            *code
        );
    }

    let error = DiagnosticCode::from_str("NOT_A_REAL_CODE").expect_err("invalid diagnostic code");
    assert_eq!(error.to_string(), "unknown HTMLCut diagnostic code");
}

#[test]
fn contract_lint_cli_operation_catalog_stays_consistent() {
    assert_eq!(
        crate::cli_contract::cli_operation_catalog_validation_errors(),
        Vec::<String>::new()
    );
}

#[test]
fn contract_lint_cli_help_catalog_stays_consistent() {
    assert_eq!(
        crate::cli_contract::cli_help_catalog_validation_errors(),
        Vec::<String>::new()
    );

    let root_help = crate::cli_root_help_document();
    assert!(
        root_help
            .sections
            .iter()
            .any(|section| section.title.contains("operator-facing entry points")),
        "root help should describe the top-level inventory"
    );
    assert!(
        root_help
            .examples
            .iter()
            .all(|example| example.starts_with("htmlcut ")),
        "root help examples should stay executable htmlcut commands"
    );

    let inspect_help = crate::cli_aux_command_help_document(crate::CliAuxCommandId::Inspect);
    let inspect_lines = inspect_help
        .sections
        .iter()
        .flat_map(|section| section.lines.iter())
        .collect::<Vec<_>>();
    assert!(
        inspect_lines
            .iter()
            .any(|line| line.starts_with("inspect source"))
    );
    assert!(
        inspect_lines
            .iter()
            .any(|line| line.starts_with("inspect select"))
    );
    assert!(
        inspect_lines
            .iter()
            .any(|line| line.starts_with("inspect slice"))
    );

    let select_help = crate::cli_operation_help_document(OperationId::SelectExtract)
        .expect("select extract help should exist");
    assert!(
        select_help
            .sections
            .iter()
            .flat_map(|section| section.lines.iter())
            .any(|line| line.contains("CSS selector matches")),
        "operation help should carry the canonical extraction summary"
    );
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
        crate::cli_contract::render_cli_value(crate::CliValue::SelectionMode(
            crate::CliSelectionMode::Nth
        )),
        "nth"
    );
    assert_eq!(
        crate::cli_contract::render_cli_value(crate::CliValue::ValueType(ValueType::InnerHtml)),
        "inner-html"
    );
    assert_eq!(
        crate::cli_contract::render_cli_value(crate::CliValue::OutputMode(
            crate::CliOutputMode::Html
        )),
        "html"
    );
    assert_eq!(
        crate::cli_contract::render_cli_value(crate::CliValue::WhitespaceMode(
            WhitespaceMode::Normalize
        )),
        "normalize"
    );
    assert_eq!(
        crate::cli_contract::render_cli_value(crate::CliValue::PatternMode(PatternMode::Regex)),
        "regex"
    );
    assert_eq!(
        crate::cli_contract::render_cli_value(crate::CliValue::FetchPreflightMode(
            FetchPreflightMode::GetOnly
        )),
        "get-only"
    );
    assert_eq!(
        crate::cli_contract::render_cli_value(crate::CliValue::Boolean(true)),
        "true"
    );
    assert_eq!(
        crate::cli_contract::render_cli_value(crate::CliValue::Usize(12)),
        "12"
    );
    assert_eq!(
        crate::cli_contract::render_cli_value(crate::CliValue::U64(64)),
        "64"
    );
}

#[test]
fn contract_lint_rejects_malformed_command_contracts_and_conditions() {
    let mut contract = crate::cli_operation_contract(OperationId::SelectExtract)
        .expect("select contract")
        .clone();
    contract.command_path = &[];
    contract.invocation = "select";
    contract.examples = vec!["select ./page.html --css article"];
    contract.default_match = Some(crate::CliSelectionMode::Single);
    contract.selection_modes = vec![crate::CliSelectionMode::First];
    contract.default_value = Some(ValueType::Structured);
    contract.value_modes = vec![ValueType::Text];
    contract.default_output = Some(crate::CliOutputMode::Json);
    contract.output_modes = vec![crate::CliOutputMode::Text];
    contract.default_output_overrides = vec![
        crate::CliConditionalDefault {
            value: crate::CliValue::ValueType(ValueType::Text),
            when: crate::CliCondition {
                parameter: crate::CliParameterId::Pattern,
                values: vec![crate::CliValue::PatternMode(PatternMode::Regex)],
            },
        },
        crate::CliConditionalDefault {
            value: crate::CliValue::OutputMode(crate::CliOutputMode::Json),
            when: crate::CliCondition {
                parameter: crate::CliParameterId::Output,
                values: vec![crate::CliValue::OutputMode(crate::CliOutputMode::Html)],
            },
        },
    ];
    contract.parameters = vec![
        crate::CliParameterDescriptor {
            section: crate::CliParameterSection::Extraction,
            id: crate::CliParameterId::Output,
            kind: crate::CliParameterKind::Option,
            requirement: crate::CliParameterRequirement::Optional,
            value_hint: Some("<MODE>"),
            default: Some(crate::CliValue::OutputMode(crate::CliOutputMode::Json)),
            allowed_values: vec![crate::CliValue::OutputMode(crate::CliOutputMode::Text)],
            summary: "output",
        },
        crate::CliParameterDescriptor {
            section: crate::CliParameterSection::Extraction,
            id: crate::CliParameterId::Output,
            kind: crate::CliParameterKind::Option,
            requirement: crate::CliParameterRequirement::Optional,
            value_hint: Some("<MODE>"),
            default: None,
            allowed_values: vec![crate::CliValue::OutputMode(crate::CliOutputMode::Text)],
            summary: "duplicate output",
        },
        crate::CliParameterDescriptor {
            section: crate::CliParameterSection::Selection,
            id: crate::CliParameterId::Match,
            kind: crate::CliParameterKind::Option,
            requirement: crate::CliParameterRequirement::Optional,
            value_hint: Some("<MATCH>"),
            default: Some(crate::CliValue::SelectionMode(
                crate::CliSelectionMode::Single,
            )),
            allowed_values: vec![crate::CliValue::SelectionMode(
                crate::CliSelectionMode::First,
            )],
            summary: "match",
        },
        crate::CliParameterDescriptor {
            section: crate::CliParameterSection::Extraction,
            id: crate::CliParameterId::Value,
            kind: crate::CliParameterKind::Option,
            requirement: crate::CliParameterRequirement::Optional,
            value_hint: Some("<VALUE>"),
            default: Some(crate::CliValue::ValueType(ValueType::Structured)),
            allowed_values: vec![crate::CliValue::ValueType(ValueType::Text)],
            summary: "value",
        },
        crate::CliParameterDescriptor {
            section: crate::CliParameterSection::Extraction,
            id: crate::CliParameterId::RewriteUrls,
            kind: crate::CliParameterKind::Flag,
            requirement: crate::CliParameterRequirement::Optional,
            value_hint: Some("<BOOL>"),
            default: Some(crate::CliValue::Boolean(true)),
            allowed_values: Vec::new(),
            summary: "rewrite URLs",
        },
        crate::CliParameterDescriptor {
            section: crate::CliParameterSection::Extraction,
            id: crate::CliParameterId::Bundle,
            kind: crate::CliParameterKind::Option,
            requirement: crate::CliParameterRequirement::RequiredUnless(crate::CliParameterId::Css),
            value_hint: Some("<DIR>"),
            default: None,
            allowed_values: Vec::new(),
            summary: "bundle",
        },
        crate::CliParameterDescriptor {
            section: crate::CliParameterSection::Extraction,
            id: crate::CliParameterId::Attribute,
            kind: crate::CliParameterKind::Option,
            requirement: crate::CliParameterRequirement::AllowedOnlyWhen(crate::CliCondition {
                parameter: crate::CliParameterId::Bundle,
                values: vec![crate::CliValue::Boolean(true)],
            }),
            value_hint: Some("<NAME>"),
            default: None,
            allowed_values: Vec::new(),
            summary: "attribute",
        },
    ];
    contract.constraints = vec![
        crate::CliConstraint::RequiresParameter {
            parameter: crate::CliParameterId::Css,
            when: crate::CliCondition {
                parameter: crate::CliParameterId::Output,
                values: vec![crate::CliValue::OutputMode(crate::CliOutputMode::Text)],
            },
        },
        crate::CliConstraint::RestrictsParameterValues {
            parameter: crate::CliParameterId::Css,
            allowed_values: vec![crate::CliValue::OutputMode(crate::CliOutputMode::None)],
            when: crate::CliCondition {
                parameter: crate::CliParameterId::Output,
                values: vec![crate::CliValue::OutputMode(crate::CliOutputMode::Text)],
            },
        },
        crate::CliConstraint::RestrictsParameterValues {
            parameter: crate::CliParameterId::Output,
            allowed_values: vec![crate::CliValue::OutputMode(crate::CliOutputMode::None)],
            when: crate::CliCondition {
                parameter: crate::CliParameterId::Output,
                values: vec![crate::CliValue::OutputMode(crate::CliOutputMode::Html)],
            },
        },
    ];

    let errors = crate::cli_contract::validate_command_contract_for_tests(&contract);
    for expected in [
        "has an empty command path",
        "invocation \"select\" does not start with",
        "example \"select ./page.html --css article\" does not start with",
        "default_match \"single\" is not present in selection_modes",
        "default_value \"structured\" is not present in value_modes",
        "default_output \"json\" is not present in output_modes",
        "default_output_override \"text\" is not an output mode",
        "default_output_override \"json\" is not present in output_modes",
        "lists --output more than once",
        "flag --rewrite-urls carries a value_hint",
        "flag --rewrite-urls should default to false",
        "parameter --output default \"json\" is not present in allowed_values",
        "parameter --bundle depends on missing parameter --css",
        "references condition parameter --bundle without allowed_values",
        "constraint references missing parameter --css",
        "value restriction references missing parameter --css",
        "references unsupported values for --output",
        "value restriction on --output references values outside its allowed_values",
    ] {
        assert!(
            errors.iter().any(|error| error.contains(expected)),
            "missing validation error containing {expected:?}: {errors:#?}"
        );
    }

    let missing_condition_errors = crate::cli_contract::validate_condition_for_tests(
        OperationId::SelectExtract,
        "default_output_override",
        &crate::CliCondition {
            parameter: crate::CliParameterId::Pattern,
            values: vec![crate::CliValue::PatternMode(PatternMode::Regex)],
        },
        &contract.parameters,
    );
    assert!(
        missing_condition_errors
            .iter()
            .any(|error| error.contains("references missing condition parameter --pattern")),
        "missing explicit condition-lint error: {missing_condition_errors:#?}"
    );

    let mut match_drift = crate::cli_operation_contract(OperationId::SelectExtract)
        .expect("select contract")
        .clone();
    match_drift.selection_modes = vec![crate::CliSelectionMode::All];
    let match_drift_errors = crate::cli_contract::validate_command_contract_for_tests(&match_drift);
    assert!(
        match_drift_errors
            .iter()
            .any(|error| error.contains("parameter --match drifted from selection_modes")),
        "missing selection-mode drift error: {match_drift_errors:#?}"
    );

    let mut value_drift = crate::cli_operation_contract(OperationId::SelectExtract)
        .expect("select contract")
        .clone();
    value_drift.value_modes = vec![ValueType::Structured];
    let value_drift_errors = crate::cli_contract::validate_command_contract_for_tests(&value_drift);
    assert!(
        value_drift_errors
            .iter()
            .any(|error| error.contains("parameter --value drifted from value_modes")),
        "missing value-mode drift error: {value_drift_errors:#?}"
    );

    let mut output_drift = crate::cli_operation_contract(OperationId::SelectExtract)
        .expect("select contract")
        .clone();
    output_drift.output_modes = vec![crate::CliOutputMode::Json];
    let output_drift_errors =
        crate::cli_contract::validate_command_contract_for_tests(&output_drift);
    assert!(
        output_drift_errors
            .iter()
            .any(|error| error.contains("parameter --output drifted from output_modes")),
        "missing output-mode drift error: {output_drift_errors:#?}"
    );
}

#[test]
fn contract_lint_covers_optional_output_and_empty_restriction_target_branches() {
    let base_contract = crate::cli_operation_contract(OperationId::SelectExtract)
        .expect("select contract")
        .clone();

    let mut no_default_output = base_contract.clone();
    no_default_output.default_output = None;
    let no_default_output_errors =
        crate::cli_contract::validate_command_contract_for_tests(&no_default_output);
    assert!(
        no_default_output_errors.is_empty(),
        "unexpected errors for optional default_output: {no_default_output_errors:#?}"
    );

    let mut without_output_parameter = base_contract.clone();
    without_output_parameter.default_output = None;
    without_output_parameter.default_output_overrides.clear();
    without_output_parameter.constraints.clear();
    without_output_parameter
        .parameters
        .retain(|parameter| parameter.id != crate::CliParameterId::Output);
    let without_output_parameter_errors =
        crate::cli_contract::validate_command_contract_for_tests(&without_output_parameter);
    assert!(
        without_output_parameter_errors.is_empty(),
        "unexpected errors for a contract without --output: {without_output_parameter_errors:#?}"
    );

    let mut empty_target_allowed_values = base_contract;
    empty_target_allowed_values.constraints =
        vec![crate::CliConstraint::RestrictsParameterValues {
            parameter: crate::CliParameterId::Bundle,
            allowed_values: vec![],
            when: crate::CliCondition {
                parameter: crate::CliParameterId::Output,
                values: vec![crate::CliValue::OutputMode(crate::CliOutputMode::Json)],
            },
        }];
    let empty_target_allowed_values_errors =
        crate::cli_contract::validate_command_contract_for_tests(&empty_target_allowed_values);
    assert!(
        empty_target_allowed_values_errors.is_empty(),
        "unexpected errors for empty restriction targets: {empty_target_allowed_values_errors:#?}"
    );
}

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
    let mut slice_contract = crate::cli_operation_contract(OperationId::SliceExtract)
        .expect("slice contract")
        .clone();
    slice_contract.command_path = &["inspect", "slice"];
    let duplicate = slice_contract.clone();
    let mut core_only_leak = crate::cli_operation_contract(OperationId::SelectExtract)
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
    let mut select_contract = crate::cli_operation_contract(OperationId::SelectExtract)
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

#[test]
fn schema_catalog_is_unique_and_covers_core_and_interop_contracts() {
    let identities = schema_catalog()
        .iter()
        .map(|descriptor| {
            (
                descriptor.schema_ref.schema_name,
                descriptor.schema_ref.schema_version,
            )
        })
        .collect::<BTreeSet<_>>();

    assert_eq!(identities.len(), schema_catalog().len());
    assert!(identities.contains(&(SOURCE_REQUEST_SCHEMA_NAME, CORE_REQUEST_SCHEMA_VERSION)));
    assert!(identities.contains(&(RUNTIME_OPTIONS_SCHEMA_NAME, CORE_REQUEST_SCHEMA_VERSION)));
    assert!(identities.contains(&(EXTRACTION_REQUEST_SCHEMA_NAME, CORE_REQUEST_SCHEMA_VERSION)));
    assert!(identities.contains(&(CORE_RESULT_SCHEMA_NAME, CORE_RESULT_SCHEMA_VERSION)));
    assert!(identities.contains(&(
        CORE_SOURCE_INSPECTION_SCHEMA_NAME,
        CORE_SOURCE_INSPECTION_SCHEMA_VERSION,
    )));
    assert!(identities.contains(&(
        interop::v1::PLAN_SCHEMA_NAME,
        interop::v1::PLAN_SCHEMA_VERSION,
    )));
    assert!(identities.contains(&(
        interop::v1::RESULT_SCHEMA_NAME,
        interop::v1::RESULT_SCHEMA_VERSION,
    )));
    assert!(identities.contains(&(
        interop::v1::ERROR_SCHEMA_NAME,
        interop::v1::ERROR_SCHEMA_VERSION,
    )));

    let extraction_result_schema =
        schema_descriptor(CORE_RESULT_SCHEMA_NAME, CORE_RESULT_SCHEMA_VERSION)
            .expect("extraction result schema");
    assert_eq!(extraction_result_schema.owner_surface, "htmlcut-core");
    assert_eq!(extraction_result_schema.rust_shape, "ExtractionResult");

    let interop_result_schema = schema_descriptor(
        interop::v1::RESULT_SCHEMA_NAME,
        interop::v1::RESULT_SCHEMA_VERSION,
    )
    .expect("interop result schema");
    assert_eq!(
        interop_result_schema.owner_surface,
        "htmlcut_core::interop::v1"
    );
    assert_eq!(interop_result_schema.stability, SchemaStability::Frozen);
}

#[test]
fn schemas_cover_inner_html_and_structured_metadata_variants() {
    let extraction_request_schema =
        (schema_descriptor(EXTRACTION_REQUEST_SCHEMA_NAME, CORE_REQUEST_SCHEMA_VERSION)
            .expect("extraction request schema")
            .json_schema)();
    let value_spec_variants = extraction_request_schema["$defs"]["ValueSpec"]["oneOf"]
        .as_array()
        .expect("value spec variants");
    let serialized_value_modes = value_spec_variants
        .iter()
        .filter_map(|variant| variant.pointer("/properties/type/const"))
        .filter_map(Value::as_str)
        .collect::<BTreeSet<_>>();
    assert!(serialized_value_modes.contains("inner-html"));
    assert!(!serialized_value_modes.contains("html"));

    let extraction_result_schema =
        (schema_descriptor(CORE_RESULT_SCHEMA_NAME, CORE_RESULT_SCHEMA_VERSION)
            .expect("extraction result schema")
            .json_schema)();
    let metadata_variants = extraction_result_schema["$defs"]["ExtractionMatchMetadata"]["oneOf"]
        .as_array()
        .expect("metadata variants");
    let metadata_kinds = metadata_variants
        .iter()
        .filter_map(|variant| variant.pointer("/properties/kind/const"))
        .filter_map(Value::as_str)
        .collect::<BTreeSet<_>>();
    assert_eq!(
        metadata_kinds,
        BTreeSet::from(["delimiter-pair", "selector"])
    );

    let value_type_variants = extraction_result_schema["$defs"]["ValueType"]["oneOf"]
        .as_array()
        .expect("value type variants");
    let serialized_value_types = value_type_variants
        .iter()
        .filter_map(|variant| variant.get("const"))
        .filter_map(Value::as_str)
        .collect::<BTreeSet<_>>();
    assert!(serialized_value_types.contains("inner-html"));
    assert!(!serialized_value_types.contains("html"));
}
