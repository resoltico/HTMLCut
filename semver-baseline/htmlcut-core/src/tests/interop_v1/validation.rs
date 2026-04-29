use super::*;

#[test]
fn interop_result_validation_rejects_zero_counts_range_drift_and_metadata_mismatch() {
    let execution = ResultExecution::new(
        TEST_PLAN_DIGEST_SHA256,
        StrategyKind::CssSelector,
        SelectionMode::Single,
        1,
    );
    let source = ResultSource {
        input_base_url: None,
        effective_base_url: None,
        document_title: None,
    };

    let mut zero = InteropResult::new(
        ResultExecution::new(
            TEST_PLAN_DIGEST_SHA256,
            StrategyKind::CssSelector,
            SelectionMode::Single,
            0,
        ),
        source.clone(),
        selector_selected_matches(),
        Vec::new(),
    );
    let zero_error = zero.validate().expect_err("zero candidates should fail");
    assert!(matches!(zero_error, ContractError::ZeroCandidateCount));

    let no_selected_matches = InteropResult::new(
        ResultExecution::new(
            TEST_PLAN_DIGEST_SHA256,
            StrategyKind::CssSelector,
            SelectionMode::Single,
            1,
        ),
        source.clone(),
        Vec::new(),
        Vec::new(),
    );
    let no_selected_matches_error = no_selected_matches
        .validate()
        .expect_err("missing selected matches should fail");
    assert!(matches!(
        no_selected_matches_error,
        ContractError::ZeroSelectedMatchCount
    ));

    let mut out_of_range = InteropResult::new(
        execution.clone(),
        source.clone(),
        selector_selected_matches(),
        Vec::new(),
    )
    .with_computed_digest()
    .expect("digest");
    out_of_range.candidate_count = 1;
    out_of_range.selected_matches[0].candidate_index = NonZeroUsize::new(2).expect("index");
    let range_error = out_of_range
        .validate()
        .expect_err("selected candidate out of range");
    assert!(matches!(
        range_error,
        ContractError::SelectedCandidateOutOfRange {
            selected: 2,
            candidate_count: 1
        }
    ));

    let valid_result = InteropResult::new(
        execution.clone(),
        source.clone(),
        selector_selected_matches(),
        Vec::new(),
    )
    .with_computed_digest()
    .expect("digest");
    assert!(valid_result.validate().is_ok());

    let mut mismatched_kind =
        InteropResult::new(execution, source, selector_selected_matches(), Vec::new())
            .with_computed_digest()
            .expect("digest");
    mismatched_kind.strategy_kind = StrategyKind::DelimiterPair;
    let mismatch_error = mismatched_kind
        .validate()
        .expect_err("metadata kind mismatch");
    assert!(matches!(
        mismatch_error,
        ContractError::MetadataKindMismatch {
            strategy_kind: StrategyKind::DelimiterPair,
            metadata_kind: StrategyKind::CssSelector
        }
    ));

    let mut mismatched_metadata = InteropResult::new(
        ResultExecution::new(
            TEST_PLAN_DIGEST_SHA256,
            StrategyKind::CssSelector,
            SelectionMode::Single,
            1,
        ),
        ResultSource {
            input_base_url: None,
            effective_base_url: None,
            document_title: None,
        },
        selector_selected_matches(),
        Vec::new(),
    )
    .with_computed_digest()
    .expect("digest");
    mismatched_metadata.candidate_count = 2;
    let metadata_error = mismatched_metadata
        .validate()
        .expect_err("metadata cardinality mismatch");
    assert!(matches!(
        metadata_error,
        ContractError::SelectedCandidateOutOfRange {
            selected: 1,
            candidate_count: 2
        }
    ));

    let duplicate_selected_candidate = InteropResult::new(
        ResultExecution::new(
            TEST_PLAN_DIGEST_SHA256,
            StrategyKind::CssSelector,
            SelectionMode::All,
            2,
        ),
        ResultSource {
            input_base_url: None,
            effective_base_url: None,
            document_title: None,
        },
        vec![
            selector_selected_match_with(2, 1),
            selector_selected_match_with(2, 1),
        ],
        Vec::new(),
    );
    let duplicate_error = duplicate_selected_candidate
        .validate()
        .expect_err("duplicate selected candidates should fail");
    assert!(matches!(
        duplicate_error,
        ContractError::DuplicateSelectedCandidate { selected: 1 }
    ));

    let mut metadata_index_mismatch = InteropResult::new(
        ResultExecution::new(
            TEST_PLAN_DIGEST_SHA256,
            StrategyKind::CssSelector,
            SelectionMode::Single,
            1,
        ),
        ResultSource {
            input_base_url: None,
            effective_base_url: None,
            document_title: None,
        },
        selector_selected_matches(),
        Vec::new(),
    )
    .with_computed_digest()
    .expect("digest");
    let SelectedMatchMetadata::CssSelector {
        ref mut candidate_index,
        ..
    } = metadata_index_mismatch.selected_matches[0].metadata
    else {
        unreachable!("selector metadata");
    };
    *candidate_index = NonZeroUsize::new(2).expect("candidate index");
    let index_mismatch_error = metadata_index_mismatch
        .validate()
        .expect_err("metadata candidate index mismatch");
    assert!(matches!(
        index_mismatch_error,
        ContractError::SelectedCandidateOutOfRange {
            selected: 1,
            candidate_count: 1
        }
    ));

    let all_count_mismatch = InteropResult::new(
        ResultExecution::new(
            TEST_PLAN_DIGEST_SHA256,
            StrategyKind::CssSelector,
            SelectionMode::All,
            2,
        ),
        ResultSource {
            input_base_url: None,
            effective_base_url: None,
            document_title: None,
        },
        selector_selected_matches(),
        Vec::new(),
    );
    let all_count_mismatch_error = all_count_mismatch
        .validate()
        .expect_err("all-selection count mismatch should fail");
    assert!(matches!(
        all_count_mismatch_error,
        ContractError::SelectionModeCountMismatch {
            selection_mode: SelectionMode::All,
            selected_match_count: 1,
            expected_selected_matches: 2,
            candidate_count: 2
        }
    ));

    let mut result_identity_drift = InteropResult::new(
        ResultExecution::new(
            TEST_PLAN_DIGEST_SHA256,
            StrategyKind::CssSelector,
            SelectionMode::Single,
            1,
        ),
        ResultSource {
            input_base_url: None,
            effective_base_url: None,
            document_title: None,
        },
        selector_selected_matches(),
        Vec::new(),
    );
    result_identity_drift.schema_name = "wrong".to_owned();
    let identity_error = result_identity_drift
        .validate()
        .expect_err("public result validate should reject drift");
    assert!(matches!(
        identity_error,
        ContractError::InvalidIdentity {
            field: "schema_name",
            ..
        }
    ));

    zero.result_digest_sha256 = "ignored".to_owned();
}
#[test]
fn interop_stable_json_digest_helpers_cover_object_and_scalar_values() {
    let object_digest =
        v1::digest_stable_json_omitting_field_for_tests(&json!({"keep": 1, "drop": 2}), "drop")
            .expect("object digest");
    let expected_object_digest =
        v1::digest_stable_json_omitting_field_for_tests(&json!({"keep": 1}), "drop")
            .expect("object digest");
    assert_eq!(object_digest, expected_object_digest);

    let scalar_digest = v1::digest_stable_json_omitting_field_for_tests(&json!("value"), "drop")
        .expect("scalar digest");
    let expected_scalar_digest =
        v1::digest_stable_json_omitting_field_for_tests(&json!("value"), "other")
            .expect("scalar digest");
    assert_eq!(scalar_digest, expected_scalar_digest);
}

#[test]
fn interop_validation_rejects_invalid_and_mismatched_digests_for_results_and_errors() {
    let source = ResultSource {
        input_base_url: None,
        effective_base_url: None,
        document_title: None,
    };
    let execution = ResultExecution::new(
        TEST_PLAN_DIGEST_SHA256,
        StrategyKind::CssSelector,
        SelectionMode::Single,
        1,
    );

    let invalid_result = InteropResult::new(
        execution.clone(),
        source.clone(),
        selector_selected_matches(),
        Vec::new(),
    );
    let invalid_result_error = invalid_result
        .validate()
        .expect_err("empty result digest should fail");
    assert!(matches!(
        invalid_result_error,
        ContractError::InvalidDigest {
            field: "result_digest_sha256",
            ..
        }
    ));

    let mut mismatched_result =
        InteropResult::new(execution, source, selector_selected_matches(), Vec::new())
            .with_computed_digest()
            .expect("digest");
    mismatched_result.result_digest_sha256 =
        "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff".to_owned();
    let result_error = mismatched_result
        .validate()
        .expect_err("mismatched result digest should fail");
    assert!(matches!(
        result_error,
        ContractError::DigestMismatch {
            field: "result_digest_sha256",
            ..
        }
    ));

    let invalid_error = InteropError::new(
        TEST_PLAN_DIGEST_SHA256,
        ErrorCode::NoMatch,
        "No matches were found.",
        Some(StrategyKind::CssSelector),
        BTreeMap::new(),
        Vec::new(),
    );
    let invalid_error_digest = invalid_error
        .validate()
        .expect_err("empty error digest should fail");
    assert!(matches!(
        invalid_error_digest,
        ContractError::InvalidDigest {
            field: "error_digest_sha256",
            ..
        }
    ));

    let mut mismatched_error = InteropError::new(
        TEST_PLAN_DIGEST_SHA256,
        ErrorCode::NoMatch,
        "No matches were found.",
        Some(StrategyKind::CssSelector),
        BTreeMap::new(),
        Vec::new(),
    )
    .with_computed_digest()
    .expect("digest");
    mismatched_error.error_digest_sha256 =
        "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff".to_owned();
    let mismatch_error = mismatched_error
        .validate()
        .expect_err("mismatched error digest should fail");
    assert!(matches!(
        mismatch_error,
        ContractError::DigestMismatch {
            field: "error_digest_sha256",
            ..
        }
    ));

    let mut identity_drift = InteropError::new(
        TEST_PLAN_DIGEST_SHA256,
        ErrorCode::NoMatch,
        "No matches were found.",
        Some(StrategyKind::CssSelector),
        BTreeMap::new(),
        Vec::new(),
    )
    .with_computed_digest()
    .expect("digest");
    identity_drift.interop_profile = "wrong-profile".to_owned();
    let identity_error = identity_drift
        .validate()
        .expect_err("wrong interop profile should fail");
    assert!(matches!(
        identity_error,
        ContractError::InvalidIdentity {
            field: "interop_profile",
            ..
        }
    ));
}
