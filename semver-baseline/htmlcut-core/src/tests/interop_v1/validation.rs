use super::*;

#[test]
fn interop_result_validation_rejects_zero_counts_range_drift_and_metadata_mismatch() {
    let execution = ResultExecution::new(
        "plan-digest",
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
            "plan-digest",
            StrategyKind::CssSelector,
            SelectionMode::Single,
            0,
        ),
        source.clone(),
        selector_selected_match(),
        Vec::new(),
    );
    let zero_error = zero.validate().expect_err("zero candidates should fail");
    assert!(matches!(zero_error, ContractError::ZeroCandidateCount));

    let mut out_of_range = InteropResult::new(
        execution.clone(),
        source.clone(),
        selector_selected_match(),
        Vec::new(),
    );
    out_of_range.candidate_count = 1;
    out_of_range.selected_match.candidate_index = NonZeroUsize::new(2).expect("index");
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
        selector_selected_match(),
        Vec::new(),
    );
    assert!(valid_result.validate().is_ok());

    let mut mismatched_kind =
        InteropResult::new(execution, source, selector_selected_match(), Vec::new());
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
            "plan-digest",
            StrategyKind::CssSelector,
            SelectionMode::Single,
            2,
        ),
        ResultSource {
            input_base_url: None,
            effective_base_url: None,
            document_title: None,
        },
        selector_selected_match(),
        Vec::new(),
    );
    let SelectedMatchMetadata::CssSelector {
        ref mut candidate_count,
        ..
    } = mismatched_metadata.selected_match.metadata
    else {
        unreachable!("selector metadata");
    };
    *candidate_count = 1;
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

    let mut metadata_index_mismatch = InteropResult::new(
        ResultExecution::new(
            "plan-digest",
            StrategyKind::CssSelector,
            SelectionMode::Single,
            1,
        ),
        ResultSource {
            input_base_url: None,
            effective_base_url: None,
            document_title: None,
        },
        selector_selected_match(),
        Vec::new(),
    );
    let SelectedMatchMetadata::CssSelector {
        ref mut candidate_index,
        ..
    } = metadata_index_mismatch.selected_match.metadata
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

    let mut result_identity_drift = InteropResult::new(
        ResultExecution::new(
            "plan-digest",
            StrategyKind::CssSelector,
            SelectionMode::Single,
            1,
        ),
        ResultSource {
            input_base_url: None,
            effective_base_url: None,
            document_title: None,
        },
        selector_selected_match(),
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
